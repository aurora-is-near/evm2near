// This is free and unencumbered software released into the public domain.

use evm_rs::{Opcode, Program};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CFGProgram(pub BTreeMap<Label, Block>);

pub type Label = usize;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Edge {
    Entry,
    Exit,
    Static(Label),
    Dynamic,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Block {
    pub label: Label,
    pub code: Range<usize>,
    pub pred: BTreeSet<Edge>,
    pub succ: BTreeSet<Edge>,
    closed: bool,
}

impl Block {
    pub fn new() -> Block {
        Self::at(0, 0, 0)
    }

    pub fn at(label: Label, start: usize, end: usize) -> Block {
        Block {
            label,
            code: Range { start, end },
            pred: BTreeSet::new(),
            succ: BTreeSet::new(),
            closed: false,
        }
    }

    #[allow(dead_code)]
    pub fn code<'a>(&self, program: &'a [Opcode]) -> &'a [Opcode] {
        &program[self.code.start..self.code.end]
    }

    pub fn add_pred(&mut self, edge: Edge) {
        _ = self.pred.insert(edge)
    }

    pub fn add_succ(&mut self, edge: Edge) {
        _ = self.succ.insert(edge)
    }

    pub fn close(&mut self) {
        self.closed = true;
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

pub fn analyze_cfg(program: &Program) -> CFGProgram {
    let mut blocks: BTreeMap<Label, Block> = BTreeMap::new();

    let mut pc: usize = 0; // program counter
    let mut prev_op: Option<&Opcode> = None;

    blocks.insert(pc, Block::at(pc, 0, 0));
    let mut block: &mut Block = blocks.get_mut(&pc).unwrap();
    block.add_pred(Edge::Entry);

    for (op_idx, op) in program.0.iter().enumerate() {
        use Opcode::*;
        match op {
            JUMP | JUMPI => {
                block.code.end = op_idx + 1;
                match prev_op {
                    Some(PUSH1(pc)) => block.add_succ(Edge::Static(usize::from(*pc))),
                    Some(PUSHn(_, pc, _)) => block.add_succ(Edge::Static(pc.as_usize())),
                    Some(_) => block.add_succ(Edge::Dynamic),
                    None => unreachable!(),
                }
                if op == &JUMPI {
                    block.add_succ(Edge::Static(pc + 1));
                }
                block.close();
            }
            JUMPDEST => {
                blocks.insert(pc, Block::at(pc, op_idx, op_idx + 1));
                block = blocks.get_mut(&pc).unwrap();
            }
            _ => {
                if block.closed {
                    blocks.insert(pc, Block::at(pc, op_idx, op_idx + 1));
                    block = blocks.get_mut(&pc).unwrap();
                }
                block.code.end = op_idx + 1;
                if op.is_halt() {
                    block.add_succ(Edge::Exit);
                }
            }
        };
        prev_op = Some(op);
        pc += op.size();
    }

    block.add_succ(Edge::Exit);

    let mut result = blocks.clone();
    for block in blocks.values() {
        for pred_edge in &block.pred {
            if let Edge::Static(pred_label) = pred_edge {
                if let Some(pred_block) = result.get_mut(pred_label) {
                    pred_block.add_succ(Edge::Static(block.label));
                }
            }
        }
        for succ_edge in &block.succ {
            if let Edge::Static(succ_label) = succ_edge {
                if let Some(succ_block) = result.get_mut(succ_label) {
                    succ_block.add_pred(Edge::Static(block.label));
                }
            }
        }
    }

    CFGProgram(result)
}
