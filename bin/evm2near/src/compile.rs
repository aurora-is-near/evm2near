// This is free and unencumbered software released into the public domain.

use std::collections::{BTreeSet, HashMap};

use evm_rs::{parse_opcode, Opcode, Program};
use parity_wasm::{
    builder::{FunctionBuilder, ModuleBuilder, SignatureBuilder},
    elements::{
        BlockType, ElementSegment, ExportEntry, FuncBody, ImportCountType, InitExpr, Instruction,
        Instructions, Internal, Local, Module, TableType, ValueType,
    },
};

use crate::{
    abi::Functions,
    analyze::{analyze_cfg, Block, CFGProgram, Edge, Label},
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
    let input_cfg = analyze_cfg(input_program);

    let mut compiler = Compiler::new(runtime_library, config);
    compiler.compile_cfg(&input_cfg, input_program);
    compiler.emit_abi(input_abi);

    let mut output_module = compiler.builder.build();

    let tables = output_module.table_section_mut().unwrap().entries_mut();
    //let table_size = tables.first().unwrap().limits().initial();
    tables[0] = TableType::new(0xFFFF, Some(0xFFFF)); // grow the table to 65,535 elements

    let elements = output_module.elements_section_mut().unwrap().entries_mut();
    for (label, func_idx) in &compiler.jump_table {
        // TODO: sorted by label
        use Instruction::*;
        elements.push(ElementSegment::new(
            0, // table
            Some(InitExpr::new(vec![
                I32Const((*label as i32) + TABLE_OFFSET),
                End,
            ])),
            vec![*func_idx],
        ));
    }

    output_module
}

type FunctionIndex = u32;

struct Compiler {
    config: CompilerConfig,
    op_table: HashMap<Opcode, FunctionIndex>,
    jump_table: HashMap<Label, FunctionIndex>,
    init_function: FunctionIndex,
    prepare_function: FunctionIndex,
    pop_function: FunctionIndex,
    jumpi_function: FunctionIndex,
    execute_function: FunctionIndex,
    function_import_count: usize,
    builder: ModuleBuilder,
}

impl Compiler {
    /// Instantiates a new compiler state.
    fn new(runtime_library: Module, config: CompilerConfig) -> Compiler {
        Compiler {
            config,
            op_table: make_op_table(&runtime_library),
            jump_table: HashMap::new(),
            init_function: find_runtime_function(&runtime_library, "_init_evm").unwrap(),
            prepare_function: find_runtime_function(&runtime_library, "_prepare").unwrap(),
            pop_function: find_runtime_function(&runtime_library, "_pop_u32").unwrap(),
            jumpi_function: find_runtime_function(&runtime_library, "jumpi").unwrap(),
            execute_function: 0, // filled in during compile_cfg()
            function_import_count: runtime_library.import_count(ImportCountType::Function),
            builder: parity_wasm::builder::from_module(runtime_library),
        }
    }

    /// Synthesizes public wrapper methods for each function in the Solidity
    /// contract's ABI, enabling users to directly call a contract method
    /// without going through the low-level `execute()` EVM dispatcher.
    pub fn emit_abi(self: &mut Compiler, input_abi: Option<Functions>) {
        assert_ne!(self.execute_function, 0); // filled in during compile_cfg()

        for func in input_abi.unwrap_or_default() {
            self.emit_function(
                Some(func.name.clone()),
                vec![
                    Instruction::I32Const(func.selector() as i32),
                    Instruction::Call(self.prepare_function),
                    Instruction::Call(self.execute_function),
                ],
            );
        }
    }

    /// Compiles the program's control-flow graph.
    fn compile_cfg(self: &mut Compiler, input_cfg: &CFGProgram, input_program: &Program) {
        let start_func_idx = self.emit_function(
            Some("_start".to_string()),
            vec![
                Instruction::I32Const(TABLE_OFFSET),
                Instruction::I64Const(self.config.chain_id.try_into().unwrap()), // --chain-id
                Instruction::I64Const(0),                                        // TODO: --balance
                Instruction::Call(self.init_function),
            ],
        );

        self.jump_table = self.make_jump_table(input_cfg);

        for block in input_cfg.0.values() {
            let block_id = make_block_id(block);

            let mut block_pc: usize = 0;
            let mut block_wasm = vec![];
            let mut emit = |pc: usize, evm: Option<&Opcode>, wasm: Vec<Instruction>| {
                if wasm.is_empty() {
                    eprintln!(
                        "{:04x} {:<73}",
                        pc,
                        evm.map(|op| op.to_string()).unwrap_or_default()
                    ); // DEBUG
                } else {
                    for wasm_op in wasm {
                        eprintln!(
                            "{:04x} {:<73} {}",
                            pc,
                            evm.map(|op| op.to_string()).unwrap_or_default(),
                            wasm_op
                        ); // DEBUG
                        block_wasm.push(wasm_op);
                    }
                }
            };

            if block.label == 0 {
                emit(block_pc, None, vec![Instruction::Call(start_func_idx)]);
            }

            let block_code = block.code(&input_program.0);
            let mut block_pos = 0;
            while block_pos < block_code.len() {
                use Opcode::*;
                let code = &block_code[block_pos..];
                match code {
                    [op @ JUMPDEST, ..] => {
                        emit(block_pc, Some(op), vec![]);
                        block_pc += op.size();
                        block_pos += 1;
                    }
                    [push @ PUSH1(label), jump @ (JUMP | JUMPI), ..] => {
                        // Static unconditional/conditional branch:
                        let label = usize::from(*label);
                        emit(block_pc, Some(push), vec![]);
                        emit(
                            block_pc,
                            Some(jump),
                            match jump {
                                JUMP => self.compile_static_jump(label),
                                JUMPI => self.compile_static_jumpi(label, &block.succ),
                                _ => unreachable!("impossible match"),
                            },
                        );
                        block_pc += push.size() + jump.size();
                        block_pos += 2;
                    }
                    [push @ PUSHn(_, label, _), jump @ (JUMP | JUMPI), ..] => {
                        // Static unconditional/conditional branch:
                        let label = label.as_usize();
                        emit(block_pc, Some(push), vec![]);
                        emit(
                            block_pc,
                            Some(jump),
                            match jump {
                                JUMP => self.compile_static_jump(label),
                                JUMPI => self.compile_static_jumpi(label, &block.succ),
                                _ => unreachable!("impossible match"),
                            },
                        );
                        block_pc += push.size() + jump.size();
                        block_pos += 2;
                    }
                    [jump @ JUMP, ..] => {
                        // Dynamic unconditional branch:
                        emit(block_pc, Some(jump), self.compile_dynamic_jump());
                        block_pc += jump.size();
                        block_pos += 1;
                    }
                    [jump @ JUMPI, ..] => {
                        // Dynamic conditional branch:
                        emit(
                            block_pc,
                            Some(jump),
                            self.compile_dynamic_jumpi(&block.succ),
                        );
                        block_pc += jump.size();
                        block_pos += 1;
                    }
                    [op, ..] => {
                        let operands = encode_operands(op);
                        if !operands.is_empty() {
                            emit(block_pc, Some(op), operands);
                        }
                        let call = self.compile_operator(op);
                        emit(block_pc, Some(op), vec![call]);
                        block_pc += op.size();
                        block_pos += 1;
                    }
                    [] => unreachable!("impossible match"),
                }
            }

            emit(block_pc, None, vec![Instruction::End]);

            let func_id = self.emit_function(Some(block_id), block_wasm);
            if block.label == 0 {
                self.execute_function = func_id
            }
        }
    }

    /// Compiles a static unconditional branch (`PUSH target; JUMP`).
    fn compile_static_jump(&self, target: Label) -> Vec<Instruction> {
        vec![self.compile_jump_to_block(target)]
    }

    /// Compiles a dynamic unconditional branch (`...; JUMP`).
    fn compile_dynamic_jump(&self) -> Vec<Instruction> {
        use Instruction::*;
        vec![
            Call(self.pop_function),
            I32Const(TABLE_OFFSET),
            I32Add,
            Drop, //CallIndirect(9, 0), // FIXME: type lookup!
        ]
    }

    /// Compiles a static conditional branch (`PUSH target; JUMPI`).
    fn compile_static_jumpi(&self, target: Label, succ: &BTreeSet<Edge>) -> Vec<Instruction> {
        assert!(succ
            .iter()
            .all(|e| matches!(e, Edge::Static(_) /*| Edge::Exit*/)));

        let else_branch = succ.iter().find(|e| {
            matches!(e, Edge::Static(label) if *label != target) /*|| matches!(e, Edge::Exit)*/
        }); // FIXME

        use Instruction::*;
        vec![
            Call(self.jumpi_function),
            If(BlockType::NoResult),
            self.compile_jump_to_block(target),
            Else,
            match else_branch {
                Some(Edge::Static(target)) => self.compile_jump_to_block(*target), // JUMPI has static successor branch
                Some(Edge::Exit) => Instruction::Return,
                _ => unreachable!("invalid preconditions"),
            },
            End,
        ]
    }

    /// Compiles a dynamic conditional branch (`...; JUMPI`).
    fn compile_dynamic_jumpi(&self, succ: &BTreeSet<Edge>) -> Vec<Instruction> {
        assert!(succ.iter().any(|e| matches!(e, Edge::Dynamic))); // then branch
        assert!(succ
            .iter()
            .any(|e| matches!(e, Edge::Static(_) /*| Edge::Exit*/))); // else branch

        let else_branch = succ
            .iter()
            .find(|e| matches!(e, Edge::Static(_) /*| Edge::Exit*/));

        use Instruction::*;
        vec![
            Call(self.pop_function),
            SetLocal(0),
            Call(self.jumpi_function),
            If(BlockType::NoResult),
            GetLocal(0),
            I32Const(TABLE_OFFSET),
            I32Add,
            Drop, //CallIndirect(9, 0), // FIXME: type lookup!
            Else,
            match else_branch {
                Some(Edge::Static(target)) => self.compile_jump_to_block(*target), // JUMPI has static successor branch
                Some(Edge::Exit) => Instruction::Return,
                _ => unreachable!("invalid preconditions"),
            },
            End,
        ]
    }

    /// Compiles the transfer of control flow to another block.
    fn compile_jump_to_block(&self, target: Label) -> Instruction {
        let jump_idx = self.jump_table.get(&target).unwrap(); // FIXME
        Instruction::Call(*jump_idx)
    }

    fn make_jump_table(&mut self, input_cfg: &CFGProgram) -> HashMap<Label, FunctionIndex> {
        let mut result: HashMap<Label, FunctionIndex> = HashMap::new();
        let base_id = self.emit_function(None, vec![]); // a dummy function
        for (block_num, block) in input_cfg.0.values().enumerate() {
            let jump_idx = (base_id as usize + block_num + 1).try_into().unwrap();
            result.insert(block.label, jump_idx);
        }
        result
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

fn make_block_id(block: &Block) -> String {
    match block.label {
        0 => "execute".to_string(), // FIXME
        pc => format!("_{:04x}", pc),
    }
}

fn make_op_table(module: &Module) -> HashMap<Opcode, FunctionIndex> {
    let mut result: HashMap<Opcode, FunctionIndex> = HashMap::new();
    for export in module.export_section().unwrap().entries() {
        match export.internal() {
            &Internal::Function(op_idx) => match export.field() {
                "_start" | "_init_evm" | "_prepare" | "_pop_u32" | "execute" => {}
                export_sym => match parse_opcode(&export_sym.to_ascii_uppercase()) {
                    None => unreachable!(),
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
    None
}
