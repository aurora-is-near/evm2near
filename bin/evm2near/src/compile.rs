// This is free and unencumbered software released into the public domain.

use std::{collections::HashMap, convert::TryInto, io::Write};

use evm_rs::{parse_opcode, Opcode, Program};
use parity_wasm::{
    builder::{FunctionBuilder, ModuleBuilder, SignatureBuilder},
    elements::{
        BlockType, ElementSegment, ExportEntry, FuncBody, ImportCountType, InitExpr, Instruction,
        Instructions, Internal, Local, Module, TableType, Type, ValueType,
    },
};
use relooper::graph::relooper::ReBlock;
use relooper::graph::{caterpillar::CaterpillarLabel, relooper::ReSeq, supergraph::SLabel};

use crate::{
    abi::Functions,
    analyze::{analyze_cfg, EvmLabel},
    config::CompilerConfig,
    encode::encode_operands,
};

const TABLE_OFFSET: i32 = 0x1000;

pub fn compile(
    input_program: &Program,
    input_abi: Option<Functions>,
    runtime_library: Module,
    config: CompilerConfig,
) -> Module {
    let relooped_cfg: ReSeq<SLabel<CaterpillarLabel<EvmLabel>>> = analyze_cfg(input_program);

    let mut compiler = Compiler::new(runtime_library, config);
    compiler.emit_wasm_start();
    compiler.emit_evm_start();
    compiler.compile_cfg(&relooped_cfg, input_program);
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
type TypeIndex = u32;

struct Compiler {
    config: CompilerConfig,
    abi_buffer_off: DataOffset,
    abi_buffer_len: usize,
    op_table: HashMap<Opcode, FunctionIndex>,
    function_type: TypeIndex,
    evm_start_function: FunctionIndex,     // _evm_start
    evm_init_function: FunctionIndex,      // _evm_init
    evm_call_function: FunctionIndex,      // _evm_call
    evm_exec_function: FunctionIndex,      // _evm_exec
    evm_post_exec_function: FunctionIndex, // _evm_post_exec
    evm_pop_function: FunctionIndex,       // _evm_pop_u32
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
            // jump_table: HashMap::new(),
            function_type: find_runtime_function_type(&runtime_library).unwrap(),
            evm_start_function: 0, // filled in during emit_start()
            evm_init_function: find_runtime_function(&runtime_library, "_evm_init").unwrap(),
            evm_call_function: find_runtime_function(&runtime_library, "_evm_call").unwrap(),
            evm_post_exec_function: find_runtime_function(&runtime_library, "_evm_post_exec")
                .unwrap(),
            evm_exec_function: 0, // filled in during compile_cfg()
            evm_pop_function: find_runtime_function(&runtime_library, "_evm_pop_u32").unwrap(),
            evm_pc_function: find_runtime_function(&runtime_library, "_evm_set_pc").unwrap(),
            function_import_count: runtime_library.import_count(ImportCountType::Function),
            builder: parity_wasm::builder::from_module(runtime_library),
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

    //TODO self is only used for `evm_pop_function`
    fn unfold_cfg(
        &self,
        program: &Program,
        cfg_part: &ReSeq<SLabel<CaterpillarLabel<EvmLabel>>>,
        res: &mut Vec<Instruction>,
    ) {
        for block in cfg_part.0.iter() {
            match block {
                ReBlock::Block(inner_seq) => {
                    res.push(Instruction::Block(BlockType::NoResult)); //TODO block type?
                    self.unfold_cfg(program, inner_seq, res);
                    res.push(Instruction::End);
                }
                ReBlock::Loop(inner_seq) => {
                    res.push(Instruction::Loop(BlockType::NoResult)); //TODO block type?
                    self.unfold_cfg(program, inner_seq, res);
                    res.push(Instruction::End);
                }
                ReBlock::If(true_branch, false_branch) => {
                    res.push(Instruction::Call(self.evm_pop_function));
                    res.push(Instruction::If(BlockType::NoResult)); //TODO block type?
                    self.unfold_cfg(program, true_branch, res);
                    res.push(Instruction::Else);
                    self.unfold_cfg(program, false_branch, res);
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
                            let code = &program.0[orig_label.code_start..orig_label.code_end];
                            for op in code {
                                let operands = encode_operands(op);
                                res.extend(operands);
                                let call = self.compile_operator(op);
                                res.push(call);
                                if op == &Opcode::RETURN {
                                    //TODO idk
                                    res.push(Instruction::Return);
                                }
                            }
                        }
                        CaterpillarLabel::Generated(a) => {
                            res.extend(vec![
                                Instruction::Call(self.evm_pop_function),
                                Instruction::I32Const(a.label.try_into().unwrap()),
                                Instruction::I32Eq,
                            ]);
                        }
                    }
                }
            }
        }
    }

    /// Compiles the program's control-flow graph.
    fn compile_cfg(
        self: &mut Compiler,
        input_cfg: &ReSeq<SLabel<CaterpillarLabel<EvmLabel>>>,
        input_program: &Program,
    ) {
        assert_ne!(self.evm_start_function, 0); // filled in during emit_start()
        assert_eq!(self.evm_exec_function, 0); // filled in below

        let mut wasm: Vec<Instruction> = Default::default();
        self.unfold_cfg(input_program, input_cfg, &mut wasm);
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
                | "_evm_post_exec" | "_evm_pop_u32" | "_evm_set_pc" | "execute" => {}
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

fn find_runtime_function_type(module: &Module) -> Option<TypeIndex> {
    for (type_id, r#type) in module.type_section().unwrap().types().iter().enumerate() {
        match r#type {
            Type::Function(function_type) => {
                if function_type.params().is_empty() && function_type.results().is_empty() {
                    return Some(type_id.try_into().unwrap());
                }
            }
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
