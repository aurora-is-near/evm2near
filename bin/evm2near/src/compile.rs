// This is free and unencumbered software released into the public domain.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    convert::TryInto,
    fmt::Display,
    fs::File,
    io::Write,
    path::PathBuf,
};

use evm_rs::{parse_opcode, Opcode, Program};
use relooper::graph::{enrichments::EnrichedCfg, relooper::ReBlock};
use relooper::graph::{reduction::SLabel, relooper::ReSeq};
use wasm_encoder::{BlockType, ExportKind, Function, Instruction, Module, ValType};

use crate::{
    abi::Functions,
    analyze::{basic_cfg, BasicCfg, CfgNode, Idx, Offs},
    config::CompilerConfig,
    encode::encode_push,
    wasm_translate::{translator::DataMode, Export, ModuleBuilder, Signature},
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

pub fn compile<'a>(
    input_program: &'a Program,
    input_abi: Option<Functions>,
    runtime_library: ModuleBuilder<'a>,
    config: CompilerConfig,
) -> Module {
    let mut compiler = Compiler::new(runtime_library, config);
    compiler.emit_wasm_start();
    compiler.emit_evm_start();
    flame::span_of("compiling cfg", || compiler.compile_cfg(input_program));
    compiler.emit_abi_execute();
    let abi_data = compiler.emit_abi_methods(input_abi).unwrap();

    let abi_buffer_ptr: usize = compiler.abi_buffer_off.try_into().unwrap();
    for data in compiler.builder.data.iter_mut() {
        let min_ptr: usize = match data.mode {
            DataMode::Active {
                memory_index: _,
                offset_instr: Instruction::I32Const(off),
            } => off.try_into().unwrap(),
            _ => continue,
        };
        let max_ptr: usize = min_ptr + data.data.len();

        if abi_buffer_ptr >= min_ptr && abi_buffer_ptr < max_ptr {
            let min_off = abi_buffer_ptr - min_ptr;
            let max_off = min_off + abi_data.len();
            assert!(min_ptr + max_off <= max_ptr);
            data.data[min_off..max_off].copy_from_slice(&abi_data);
            break; // found it
        }
    }

    compiler.debug_write("flamegraph.html", |w| {
        flame::dump_html(w).expect("flamegraph dump error")
    });

    compiler.builder.build()
}

type DataOffset = i32;
type FunctionIndex = u32;

struct Compiler<'a> {
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
    evm_burn_gas: FunctionIndex,           // _evm_burn_gas
    evm_pc_function: FunctionIndex,        // _evm_set_pc
    builder: ModuleBuilder<'a>,
}

impl<'a> Compiler<'a> {
    /// Instantiates a new compiler state.
    fn new(runtime_library: ModuleBuilder, config: CompilerConfig) -> Compiler {
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
            evm_burn_gas: find_runtime_function(&runtime_library, "_evm_burn_gas").unwrap(),
            evm_pc_function: find_runtime_function(&runtime_library, "_evm_set_pc").unwrap(),
            builder: runtime_library,
        }
    }

    fn debug<TPath: Into<PathBuf>, CF: Fn() -> String>(&self, path: TPath, contents: CF) {
        if let Some(base_path) = &self.config.debug_path {
            let mut full_path = base_path.clone();
            full_path.push(path.into());

            std::fs::write(full_path, contents()).expect("fs error while writing debug file");
        }
    }

    fn debug_write<TPath: Into<PathBuf>, CF: Fn(&File)>(&self, path: TPath, writer: CF) {
        if let Some(base_path) = &self.config.debug_path {
            let mut full_path = base_path.clone();
            full_path.push(path.into());

            let w = std::fs::File::create(full_path).expect("fs error while writing debug file");
            writer(&w);
        }
    }

    /// Emit an empty `_start` function to make all WebAssembly runtimes happy.
    fn emit_wasm_start(&mut self) {
        _ = self.emit_function(Some("_start".to_string()), vec![]);
    }

    /// Synthesizes a start function that initializes the EVM state with the
    /// correct configuration.
    fn emit_evm_start(&mut self) {
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

    fn emit_abi_execute(&mut self) {
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
        &mut self,
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

    //TODO self is only used for `evm_pop_function`
    fn unfold_cfg(
        &self,
        program: &'a Program,
        cfg_part: &ReSeq<SLabel<CfgNode<EvmBlock>>>,
        res: &mut Vec<Instruction<'a>>,
        wasm_idx2evm_idx: &mut HashMap<Idx, Idx>,
    ) {
        for block in cfg_part.0.iter() {
            match block {
                ReBlock::Block(inner_seq) => {
                    res.push(Instruction::Block(BlockType::Empty));
                    self.unfold_cfg(program, inner_seq, res, wasm_idx2evm_idx);
                    res.push(Instruction::End);
                }
                ReBlock::Loop(inner_seq) => {
                    res.push(Instruction::Loop(BlockType::Empty));
                    self.unfold_cfg(program, inner_seq, res, wasm_idx2evm_idx);
                    res.push(Instruction::End);
                }
                ReBlock::If(true_branch, false_branch) => {
                    res.push(Instruction::Call(self.evm_pop_function));
                    res.push(Instruction::If(BlockType::Empty));
                    self.unfold_cfg(program, true_branch, res, wasm_idx2evm_idx);
                    res.push(Instruction::Else);
                    self.unfold_cfg(program, false_branch, res, wasm_idx2evm_idx);
                    res.push(Instruction::End);
                }
                ReBlock::Br(levels) => {
                    res.push(Instruction::Br(*levels));
                }
                ReBlock::Return => {
                    res.push(Instruction::Return);
                }
                ReBlock::Actions(block) => {
                    match block.origin {
                        CfgNode::Orig(orig_label) => {
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
                        CfgNode::Dynamic => {}
                    }
                }
                ReBlock::TableJump(table) => {
                    let (table_len, _) = table.last_key_value().unwrap(); // should be safe as switch of zero variants is meaningless
                    let mut linear_table = vec![0; table_len + 1];
                    for (&cond, &br_num) in table {
                        linear_table[cond] = br_num + 1; // increment due to additional block wrapping (for unreachable instruction)
                    }

                    res.push(Instruction::Block(BlockType::Empty));
                    res.push(Instruction::Call(self.evm_pop_function));
                    let cow = Cow::Owned(linear_table);
                    let br_table = Instruction::BrTable(cow, 0);
                    res.push(br_table);
                    res.push(Instruction::End);
                    res.push(Instruction::Unreachable);
                }
            }
        }
    }

    fn opcodes_debug(&self, program: &Program) {
        let mut opcode_lines: Vec<String> = vec![];
        program.0.iter().fold(Offs(0), |offs, opcode| {
            opcode_lines.push(format!("0x{:02x}\t{}", offs.0, opcode));
            Offs(offs.0 + opcode.size())
        });
        self.debug("opcodes.evm", || opcode_lines.join("\n"));
    }

    fn evm_wasm_dot_debug(
        &self,
        program: &Program,
        basic_cfg: &BasicCfg,
        wasm: &[Instruction],
        wasm_idx2evm_idx: &HashMap<Idx, Idx>,
    ) {
        let evm_idx2offs = evm_idx_to_offs(program);

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
            .map(|(idx, w_op)| format!("wasm_{}[label=\"{:?}\"];", idx, w_op))
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
    fn compile_cfg(&mut self, program: &'a Program) {
        assert_ne!(self.evm_start_function, 0); // filled in during emit_start()
        assert_eq!(self.evm_exec_function, 0); // filled in below

        self.opcodes_debug(program);

        let basic_cfg = flame::span_of("building basic cfg", || basic_cfg(program));
        self.debug("basic_cfg.dot", || {
            format!("digraph {{{}}}", basic_cfg.cfg.cfg_to_dot("basic"))
        });

        let mut evm_cfg = basic_cfg.cfg.map_label(|n| match n {
            CfgNode::Orig(l) => {
                let a = basic_cfg.code_ranges.get(l).unwrap();
                CfgNode::Orig(EvmBlock::new(*l, a.start, a.end))
            }
            CfgNode::Dynamic => CfgNode::Dynamic,
        });

        evm_cfg.strip_unreachable();
        self.debug("stripped.dot", || {
            format!("digraph {{{}}}", evm_cfg.cfg_to_dot("stripped"))
        });
        // println!("orig: {}", evm_cfg.nodes().len());
        // let old_reduced = relooper::graph::supergraph::reduce(&evm_cfg);
        let reduced = relooper::graph::reduction::reduce(&evm_cfg);
        // println!(
        //     "old: {}, new: {}",
        //     old_reduced.nodes().len(),
        //     reduced.nodes().len()
        // );
        self.debug("reduced.dot", || {
            format!("digraph {{{}}}", reduced.cfg_to_dot("reduced"))
        });
        let enriched = flame::span_of("enriching cfg", || EnrichedCfg::new(reduced));
        self.debug("enriched.dot", || {
            format!(
                "digraph {{{} {}}}",
                enriched.cfg_to_dot("enriched"),
                enriched.dom_to_dot()
            )
        });
        let relooped_cfg = flame::span_of("relooping", || enriched.reloop());

        self.debug("relooped.dot", || {
            format!("digraph {{{}}}", relooped_cfg.to_dot())
        });

        let mut wasm: Vec<Instruction> = Default::default();
        let mut wasm_idx2evm_idx = Default::default();
        self.unfold_cfg(program, &relooped_cfg, &mut wasm, &mut wasm_idx2evm_idx);
        wasm.push(Instruction::End);

        if self.config.debug_path.is_some() {
            self.evm_wasm_dot_debug(program, &basic_cfg, &wasm, &wasm_idx2evm_idx);
        }

        let func_id = self.emit_function(Some("_evm_exec".to_string()), wasm);
        self.evm_exec_function = func_id;
    }

    /// Compiles the invocation of an EVM operator (operands must be already pushed).
    fn compile_operator(&self, op: &Opcode) -> Instruction<'a> {
        let op = op.zeroed();
        let op_idx = self.op_table.get(&op).unwrap();
        Instruction::Call(*op_idx)
    }

    fn emit_function(&mut self, name: Option<String>, mut code: Vec<Instruction>) -> FunctionIndex {
        match code.last() {
            Some(Instruction::End) => {}
            Some(_) | None => code.push(Instruction::End),
        };

        let func_sig = Signature {
            params: vec![],
            results: vec![],
        };

        let mut func_body = Function::new_with_locals_types(vec![ValType::I32]);
        for instr in code {
            func_body.instruction(&instr);
        }

        let imports_len = u32::try_from(self.builder.imports.len()).unwrap();
        let func_idx = self.builder.add_function(func_sig, func_body) + imports_len;

        if let Some(name) = name {
            let func_export = Export {
                name,
                kind: ExportKind::Func,
                index: func_idx,
            };
            let _ = self.builder.add_export(func_export);
        }

        func_idx
    }
}

fn make_op_table(module: &ModuleBuilder) -> HashMap<Opcode, FunctionIndex> {
    let mut result: HashMap<Opcode, FunctionIndex> = HashMap::new();
    for export in module.exports.iter() {
        if let Export {
            name,
            kind: ExportKind::Func,
            index,
        } = export
        {
            match name.as_str() {
                "_abi_buffer" | "_evm_start" | "_evm_init" | "_evm_call" | "_evm_exec"
                | "_evm_post_exec" | "_evm_pop_u32" | "_evm_push_u32" | "_evm_burn_gas"
                | "_evm_set_pc" | "execute" => {}
                export_sym => match parse_opcode(&export_sym.to_ascii_uppercase()) {
                    None => unreachable!(), // TODO
                    Some(op) => _ = result.insert(op, *index),
                },
            }
        }
    }
    result
}

fn find_runtime_function(module: &ModuleBuilder, func_name: &str) -> Option<FunctionIndex> {
    for export in module.exports.iter() {
        if let Export {
            name,
            kind: ExportKind::Func,
            index,
        } = export
        {
            if name == func_name {
                return Some(*index);
            }
        }
    }
    None // not found
}

fn find_abi_buffer(module: &ModuleBuilder) -> Option<DataOffset> {
    for export in module.exports.iter() {
        if let Export {
            name,
            kind: ExportKind::Global,
            index,
        } = export
        {
            if name == "_abi_buffer" {
                let g = module.globals.get(*index as usize).unwrap();
                match g.init_instr {
                    wasm_encoder::Instruction::I32Const(off) => return Some(off),
                    _ => return None,
                }
            }
        }
    }
    None // not found
}
