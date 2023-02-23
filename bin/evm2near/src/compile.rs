// This is free and unencumbered software released into the public domain.

use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    fmt::Display,
    io::Write,
    path::PathBuf,
};

use evm_rs::{parse_opcode, Opcode, Program};
use parity_wasm::{
    builder::{FunctionBuilder, ModuleBuilder, SignatureBuilder},
    elements::{
        BlockType, ExportEntry, FuncBody, ImportCountType, Instruction, Instructions, Internal,
        Local, Module, TableType, ValueType,
    },
};
use relooper::graph::{
    caterpillar::EvmCfgLabel,
    relooper::{reloop, ReBlock},
    supergraph::reduce,
};
use relooper::graph::{
    caterpillar::{unfold_dyn_edges, CaterpillarLabel},
    relooper::ReSeq,
    supergraph::SLabel,
};

use crate::{
    abi::Functions,
    analyze::{basic_cfg, BasicCfg, Idx, Offs},
    config::CompilerConfig,
    encode::encode_push,
};

const TABLE_OFFSET: i32 = 0x1000;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvmBlock {
    pub label: Offs,
    pub code_start: Idx,
    pub code_end: Idx,
}

impl EvmBlock {
    fn new(label: Offs, code_start: Idx, code_end: Idx) -> Self {
        Self {
            label,
            code_start,
            code_end,
        }
    }
}

impl Display for EvmBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}_to_{}", self.label, self.code_start, self.code_end)
    }
}

fn evm_idx_to_offs(program: &Program) -> HashMap<Idx, Offs> {
    let mut idx2offs: HashMap<Idx, Offs> = Default::default();
    program
        .0
        .iter()
        .enumerate()
        .fold(Offs(0), |offs, (cnt, opcode)| {
            idx2offs.insert(Idx(cnt), offs);
            Offs(offs.0 + opcode.size())
        });
    idx2offs
}

pub fn compile(
    input_program: &Program,
    input_abi: Option<Functions>,
    runtime_library: Module,
    config: CompilerConfig,
) -> Module {
    let mut compiler = Compiler::new(runtime_library, config);
    compiler.emit_wasm_start();
    compiler.emit_evm_start();
    compiler.compile_cfg(input_program);
    compiler.emit_abi_execute();
    let abi_data = compiler.emit_abi_methods(input_abi).unwrap();

    let mut output_module = compiler.builder.build();

    let tables = output_module.table_section_mut().unwrap().entries_mut();
    tables[0] = TableType::new(0xFFFF, Some(0xFFFF)); // grow the table to 65,535 elements

    // Overwrite the `_abi_buffer` data segment in evmlib with the ABI data
    // (function parameter names and types) for all public Solidity contract
    // methods:
    let abi_buffer_ptr: usize = compiler.abi_buffer_off.try_into().unwrap();
    for data in output_module.data_section_mut().unwrap().entries_mut() {
        let min_ptr: usize = match data.offset().as_ref().unwrap().code() {
            [Instruction::I32Const(off), Instruction::End] => (*off).try_into().unwrap(),
            _ => continue, // skip any nonstandard data segments
        };
        let max_ptr: usize = min_ptr + data.value().len();
        if abi_buffer_ptr >= min_ptr && abi_buffer_ptr < max_ptr {
            let min_off = abi_buffer_ptr - min_ptr;
            let max_off = min_off + abi_data.len();
            assert!(min_ptr + max_off <= max_ptr);
            data.value_mut()[min_off..max_off].copy_from_slice(&abi_data);
            break; // found it
        }
    }
    output_module
}

type DataOffset = i32;
type FunctionIndex = u32;

struct Compiler {
    config: CompilerConfig,
    abi_buffer_off: DataOffset,
    abi_buffer_len: usize,
    op_table: HashMap<Opcode, FunctionIndex>,
    evm_start_function: FunctionIndex,     // _evm_start
    evm_init_function: FunctionIndex,      // _evm_init
    evm_call_function: FunctionIndex,      // _evm_call
    evm_exec_function: FunctionIndex,      // _evm_exec
    evm_post_exec_function: FunctionIndex, // _evm_post_exec
    evm_pop_function: FunctionIndex,       // _evm_pop_u32
    evm_push_function: FunctionIndex,      // _evm_push_u32
    evm_burn_gas: FunctionIndex,           // _evm_burn_gas
    evm_pc_function: FunctionIndex,        // _evm_set_pc
    function_import_count: usize,
    builder: ModuleBuilder,
}

impl Compiler {
    /// Instantiates a new compiler state.
    fn new(runtime_library: Module, config: CompilerConfig) -> Compiler {
        Compiler {
            config,
            abi_buffer_off: find_abi_buffer(&runtime_library).unwrap(),
            abi_buffer_len: 0xFFFF, // TODO: ensure this matches _abi_buffer.len() in evmlib
            op_table: make_op_table(&runtime_library),
            evm_start_function: 0, // filled in during emit_start()
            evm_init_function: find_runtime_function(&runtime_library, "_evm_init").unwrap(),
            evm_call_function: find_runtime_function(&runtime_library, "_evm_call").unwrap(),
            evm_post_exec_function: find_runtime_function(&runtime_library, "_evm_post_exec")
                .unwrap(),
            evm_exec_function: 0, // filled in during compile_cfg()
            evm_pop_function: find_runtime_function(&runtime_library, "_evm_pop_u32").unwrap(),
            evm_push_function: find_runtime_function(&runtime_library, "_evm_push_u32").unwrap(),
            evm_burn_gas: find_runtime_function(&runtime_library, "_evm_burn_gas").unwrap(),
            evm_pc_function: find_runtime_function(&runtime_library, "_evm_set_pc").unwrap(),
            function_import_count: runtime_library.import_count(ImportCountType::Function),
            builder: parity_wasm::builder::from_module(runtime_library),
        }
    }

    fn debug<TPath: Into<PathBuf>, CF: Fn() -> String>(&self, path: TPath, contents: CF) {
        if let Some(base_path) = &self.config.debug_path {
            let mut full_path = base_path.clone();
            full_path.push(path.into());

            std::fs::write(full_path, contents()).expect("fs error while writing debug file");
        }
    }

    /// Emit an empty `_start` function to make all WebAssembly runtimes happy.
    fn emit_wasm_start(self: &mut Compiler) {
        _ = self.emit_function(Some("_start".to_string()), vec![]);
    }

    /// Synthesizes a start function that initializes the EVM state with the
    /// correct configuration.
    fn emit_evm_start(self: &mut Compiler) {
        assert_ne!(self.evm_init_function, 0);

        self.evm_start_function = self.emit_function(
            Some("_evm_start".to_string()),
            vec![
                Instruction::I32Const(TABLE_OFFSET),
                Instruction::I64Const(self.config.chain_id.try_into().unwrap()), // --chain-id
                Instruction::I64Const(0),                                        // TODO: --balance
                Instruction::Call(self.evm_init_function),
            ],
        );
    }

    fn emit_abi_execute(self: &mut Compiler) {
        assert_ne!(self.evm_start_function, 0);
        assert_ne!(self.evm_exec_function, 0); // filled in during compile_cfg()

        _ = self.emit_function(
            Some("execute".to_string()),
            vec![
                Instruction::Call(self.evm_start_function),
                Instruction::Call(self.evm_exec_function),
                Instruction::I32Const(0),
                Instruction::I32Const(0), // output_types_len == 0 means no JSON encoding
                Instruction::Call(self.evm_post_exec_function),
            ],
        );
    }

    /// Synthesizes public wrapper methods for each function in the Solidity
    /// contract's ABI, enabling users to directly call a contract method
    /// without going through the low-level `execute` EVM dispatcher.
    pub fn emit_abi_methods(
        self: &mut Compiler,
        input_abi: Option<Functions>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        assert_ne!(self.evm_start_function, 0);
        assert_ne!(self.evm_call_function, 0);
        assert_ne!(self.evm_exec_function, 0); // filled in during compile_cfg()

        let mut data = Vec::with_capacity(self.abi_buffer_len);
        for func in input_abi.unwrap_or_default() {
            let names_off = data.len();
            for (i, input) in func.inputs.iter().enumerate() {
                if i > 0 {
                    write!(data, ",")?;
                }
                write!(data, "{}", input.name)?;
            }
            let names_len = data.len() - names_off;
            data.push(0); // NUL

            let types_off = data.len();
            for (i, input) in func.inputs.iter().enumerate() {
                if i > 0 {
                    write!(data, ",")?;
                }
                if abi_types::parse_param_type(&input.r#type).is_err() {
                    panic!("Unknown ABI type: {}", input.r#type);
                }
                write!(data, "{}", input.r#type)?;
            }
            let types_len = data.len() - types_off;
            data.push(0); // NUL

            let output_types_off = data.len();
            for (i, output) in func.outputs.iter().enumerate() {
                if i > 0 {
                    write!(data, ",")?;
                }
                if abi_types::parse_param_type(&output.r#type).is_err() {
                    panic!("Unknown ABI type: {}", output.r#type);
                }
                write!(data, "{}", output.r#type)?;
            }
            let output_types_len = data.len() - output_types_off;
            data.push(0); // NUL

            _ = self.emit_function(
                Some(func.name.clone()),
                vec![
                    Instruction::Call(self.evm_start_function),
                    Instruction::I32Const(func.selector() as i32),
                    Instruction::I32Const(names_off.try_into().unwrap()), // params_names_ptr
                    Instruction::I32Const(names_len.try_into().unwrap()), // params_names_len
                    Instruction::I32Const(types_off.try_into().unwrap()), // params_types_ptr
                    Instruction::I32Const(types_len.try_into().unwrap()), // params_types_len
                    Instruction::Call(self.evm_call_function),
                    Instruction::Call(self.evm_exec_function),
                    Instruction::I32Const(output_types_off.try_into().unwrap()), // output_types_off
                    Instruction::I32Const(output_types_len.try_into().unwrap()), // output_types_len
                    Instruction::Call(self.evm_post_exec_function),
                ],
            );
        }
        Ok(data)
    }

    fn relooped_cfg(&self, basic_cfg: &BasicCfg) -> ReSeq<SLabel<CaterpillarLabel<EvmBlock>>> {
        let cfg = basic_cfg.cfg.map_label(|label| {
            let code_range = basic_cfg
                .code_ranges
                .get(label)
                .unwrap_or_else(|| panic!("no code ranges for {}", label));
            let &node_info = basic_cfg.node_info.get(label).unwrap();
            let evm_label = EvmBlock::new(*label, code_range.start, code_range.end);
            EvmCfgLabel {
                cfg_label: evm_label,
                is_jumpdest: node_info.is_jumpdest,
                is_dynamic: node_info.is_dynamic,
            }
        });

        let mut dynamic_materialized = unfold_dyn_edges(&cfg);
        dynamic_materialized.strip_unreachable();
        let reduced = reduce(&dynamic_materialized);
        reloop(&reduced)
    }

    //TODO self is only used for `evm_pop_function`
    fn unfold_cfg(
        &self,
        program: &Program,
        cfg_part: &ReSeq<SLabel<CaterpillarLabel<EvmBlock>>>,
        res: &mut Vec<Instruction>,
        wasm_idx2evm_idx: &mut HashMap<Idx, Idx>,
    ) {
        for block in cfg_part.0.iter() {
            match block {
                ReBlock::Block(inner_seq) => {
                    res.push(Instruction::Block(BlockType::NoResult));
                    self.unfold_cfg(program, inner_seq, res, wasm_idx2evm_idx);
                    res.push(Instruction::End);
                }
                ReBlock::Loop(inner_seq) => {
                    res.push(Instruction::Loop(BlockType::NoResult));
                    self.unfold_cfg(program, inner_seq, res, wasm_idx2evm_idx);
                    res.push(Instruction::End);
                }
                ReBlock::If(true_branch, false_branch) => {
                    res.push(Instruction::Call(self.evm_pop_function));
                    res.push(Instruction::If(BlockType::NoResult));
                    self.unfold_cfg(program, true_branch, res, wasm_idx2evm_idx);
                    res.push(Instruction::Else);
                    self.unfold_cfg(program, false_branch, res, wasm_idx2evm_idx);
                    res.push(Instruction::End);
                }
                ReBlock::Br(levels) => {
                    res.push(Instruction::Br((*levels).try_into().unwrap()));
                }
                ReBlock::Return => {
                    res.push(Instruction::Return);
                }
                ReBlock::Actions(block) => {
                    match block.origin {
                        CaterpillarLabel::Original(orig_label) => {
                            let block_code =
                                &program.0[orig_label.code_start.0..orig_label.code_end.0];
                            let block_len = orig_label.code_end.0 - orig_label.code_start.0;
                            let mut curr_idx = 0;
                            let mut evm_offset: usize = 0;
                            while curr_idx < block_len {
                                match &block_code[curr_idx..] {
                                    [p, j, ..] if p.is_push() && j.is_jump() => {
                                        // this is static jump, already accounted during cfg analysis. we only need to burn gas there
                                        let jump_gas = if j == &Opcode::JUMP { 8 } else { 10 };
                                        res.extend(vec![
                                            Instruction::I32Const(3),             // any push costs 3 gas
                                            Instruction::Call(self.evm_burn_gas), // burn it
                                            Instruction::I32Const(jump_gas),
                                            Instruction::Call(self.evm_burn_gas),
                                        ]);
                                        curr_idx += 2;
                                        evm_offset += p.size() + j.size();
                                    }
                                    [j, ..] if j.is_jump() => {
                                        // this is dynamic jump
                                        let jump_gas = if j == &Opcode::JUMP { 8 } else { 10 };
                                        res.extend(vec![
                                            Instruction::Call(self.evm_pop_function),
                                            Instruction::SetLocal(0),
                                            Instruction::I32Const(jump_gas),
                                            Instruction::Call(self.evm_burn_gas),
                                        ]);
                                        curr_idx += 1;
                                        evm_offset += j.size();
                                    }
                                    [op, ..] => {
                                        wasm_idx2evm_idx.insert(
                                            Idx(res.len()),
                                            Idx(curr_idx + orig_label.code_start.0),
                                        );
                                        if self.config.program_counter {
                                            let pc = orig_label.label.0 + evm_offset;
                                            res.extend(vec![
                                                Instruction::I32Const(pc.try_into().unwrap()),
                                                Instruction::Call(self.evm_pc_function),
                                            ]);
                                        }
                                        if op.is_push() {
                                            let operands = encode_push(op);
                                            res.extend(operands);
                                        }
                                        let call = self.compile_operator(op);
                                        res.push(call);
                                        if op == &Opcode::RETURN {
                                            res.push(Instruction::Return);
                                        }
                                        curr_idx += 1;
                                        evm_offset += op.size();
                                    }
                                    [] => {
                                        unreachable!()
                                    }
                                }
                            }
                        }
                        CaterpillarLabel::Generated(a) => {
                            res.extend(vec![
                                Instruction::GetLocal(0),
                                Instruction::I32Const(a.label.0.try_into().unwrap()),
                                Instruction::I32Eq,
                                Instruction::Call(self.evm_push_function),
                            ]);
                        }
                    }
                }
            }
        }
    }

    fn evm_wasm_dot_debug(
        &self,
        program: &Program,
        basic_cfg: &BasicCfg,
        _input_cfg: &ReSeq<SLabel<CaterpillarLabel<EvmBlock>>>,
        wasm: &[Instruction],
        wasm_idx2evm_idx: &HashMap<Idx, Idx>,
    ) {
        let evm_idx2offs = evm_idx_to_offs(program);

        let mut opcode_lines: Vec<String> = vec![];
        program.0.iter().fold(Offs(0), |offs, opcode| {
            opcode_lines.push(format!("0x{:02x}\t{}", offs.0, opcode));
            Offs(offs.0 + opcode.size())
        });
        self.debug("opcodes.evm", || opcode_lines.join("\n"));

        let mut code_ranges: Vec<_> = basic_cfg.code_ranges.iter().collect();
        code_ranges.sort_by_key(|&(Offs(offs), _r)| offs);

        let evm_blocks: Vec<_> = code_ranges
            .iter()
            .map(|(offs, range)| {
                let range_nodes: Vec<_> = (range.start.0..range.end.0)
                    .map(|idx| {
                        let idx = Idx(idx);
                        let op_offs = evm_idx2offs.get(&idx).unwrap();
                        let e_op = &program.0[idx.0];
                        format!("evm_{}[label=\"0x{:x}: {}\"];", idx, op_offs.0, e_op)
                    })
                    .collect();
                format!(
                    "subgraph cluster_evm_{} {{ label = \"{}-{}, 0x{:x}\";
{}
}}",
                    offs.0,
                    range.start,
                    range.end,
                    offs.0,
                    range_nodes.join("\n")
                )
            })
            .collect();

        let evm_seq_links: Vec<_> = (0..program.0.len())
            .collect::<Vec<_>>()
            .windows(2) // TODO use `array_windows` (unstable for now)
            .map(|pair| {
                let a = pair[0];
                let b = pair[1];
                format!("evm_{a} -> evm_{b};")
            })
            .collect();

        let mut evm_lines = Vec::default();
        evm_lines.extend(evm_blocks);
        evm_lines.extend(evm_seq_links);

        let wasm_nodes: Vec<_> = wasm
            .iter()
            .enumerate()
            .map(|(idx, w_op)| format!("wasm_{}[label=\"{}\"];", idx, w_op))
            .collect();

        let wasm_seq_links: Vec<_> = (0..wasm.len())
            .collect::<Vec<_>>()
            .windows(2)
            .map(|pair| {
                let a = pair[0];
                let b = pair[1];
                format!("wasm_{a} -> wasm_{b};")
            })
            .collect();

        let mut wasm_lines = Vec::default();
        wasm_lines.extend(wasm_nodes);
        wasm_lines.extend(wasm_seq_links);

        let evm_block_starts: HashSet<_> = code_ranges.iter().map(|(_, b)| b.start).collect();
        let wasm2evm_lines: Vec<_> = wasm_idx2evm_idx
            .iter()
            .filter_map(|(w, e)| {
                if evm_block_starts.contains(e) {
                    Some(format!("wasm_{w} -> evm_{e}[constraint=false];"))
                } else {
                    None
                }
            })
            .collect();

        self.debug("dbg.dot", || {
            format!(
                "digraph {{
subgraph cluster_evm {{ label = \"evm\"
{}
}}
subgraph cluster_wasm {{ label = \"wasm\"
{}
}}
{}
}}",
                evm_lines.join("\n"),
                wasm_lines.join("\n"),
                wasm2evm_lines.join("\n")
            )
        });
    }

    /// Compiles the program's control-flow graph.
    fn compile_cfg(self: &mut Compiler, program: &Program) {
        assert_ne!(self.evm_start_function, 0); // filled in during emit_start()
        assert_eq!(self.evm_exec_function, 0); // filled in below

        let basic_cfg = basic_cfg(program);
        self.debug("basic_cfg.dot", || {
            format!("digraph {{{}}}", basic_cfg.cfg.cfg_to_dot("basic"))
        });
        let relooped_cfg = self.relooped_cfg(&basic_cfg);
        self.debug("relooped.dot", || {
            format!("digraph {{{}}}", relooped_cfg.to_dot())
        });

        let mut wasm: Vec<Instruction> = Default::default();
        let mut wasm_idx2evm_idx = Default::default();
        self.unfold_cfg(program, &relooped_cfg, &mut wasm, &mut wasm_idx2evm_idx);
        wasm.push(Instruction::End);

        if self.config.debug_path.is_some() {
            self.evm_wasm_dot_debug(program, &basic_cfg, &relooped_cfg, &wasm, &wasm_idx2evm_idx);
        }

        let func_id = self.emit_function(Some("_evm_exec".to_string()), wasm);
        self.evm_exec_function = func_id;
    }

    /// Compiles the invocation of an EVM operator (operands must be already pushed).
    fn compile_operator(&self, op: &Opcode) -> Instruction {
        let op = op.zeroed();
        let op_idx = self.op_table.get(&op).unwrap();
        Instruction::Call(*op_idx)
    }

    fn emit_function(&mut self, name: Option<String>, mut code: Vec<Instruction>) -> FunctionIndex {
        match code.last() {
            Some(Instruction::End) => {}
            Some(_) | None => code.push(Instruction::End),
        };

        let func_sig = SignatureBuilder::new()
            .with_params(vec![])
            .with_results(vec![])
            .build_sig();

        let func_locals = vec![Local::new(1, ValueType::I32)]; // needed for dynamic branches
        let func_body = FuncBody::new(func_locals, Instructions::new(code));

        let func = FunctionBuilder::new()
            .with_signature(func_sig)
            .with_body(func_body)
            .build();

        let func_loc = self.builder.push_function(func);

        let func_idx = func_loc.signature + self.function_import_count as u32; // TODO: https://github.com/paritytech/parity-wasm/issues/304

        if let Some(name) = name {
            let func_export = ExportEntry::new(name, Internal::Function(func_idx));

            let _ = self.builder.push_export(func_export);
        }

        func_idx
    }
}

fn make_op_table(module: &Module) -> HashMap<Opcode, FunctionIndex> {
    let mut result: HashMap<Opcode, FunctionIndex> = HashMap::new();
    for export in module.export_section().unwrap().entries() {
        match export.internal() {
            &Internal::Function(op_idx) => match export.field() {
                "_abi_buffer" | "_evm_start" | "_evm_init" | "_evm_call" | "_evm_exec"
                | "_evm_post_exec" | "_evm_pop_u32" | "_evm_push_u32" | "_evm_burn_gas"
                | "_evm_set_pc" | "execute" => {}
                export_sym => match parse_opcode(&export_sym.to_ascii_uppercase()) {
                    None => unreachable!(), // TODO
                    Some(op) => _ = result.insert(op, op_idx),
                },
            },
            _ => continue,
        }
    }
    result
}

fn find_runtime_function(module: &Module, name: &str) -> Option<FunctionIndex> {
    for export in module.export_section().unwrap().entries() {
        match export.internal() {
            &Internal::Function(op_idx) => {
                if export.field() == name {
                    return Some(op_idx);
                }
            }
            _ => continue,
        }
    }
    None // not found
}

fn find_abi_buffer(module: &Module) -> Option<DataOffset> {
    for export in module.export_section().unwrap().entries() {
        match export.internal() {
            &Internal::Global(idx) => {
                if export.field() == "_abi_buffer" {
                    // found it
                    let global = module
                        .global_section()
                        .unwrap()
                        .entries()
                        .get(idx as usize)
                        .unwrap();
                    match global.init_expr().code().first().unwrap() {
                        Instruction::I32Const(off) => return Some(*off),
                        _ => return None,
                    }
                }
            }
            _ => continue,
        }
    }
    None // not found
}
