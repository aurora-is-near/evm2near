// This is free and unencumbered software released into the public domain.

use evm_rs::{Opcode, Program};
use relooper::graph::{
    caterpillar::{unfold_dyn_edges, EvmLabel},
    cfg::{Cfg, CfgEdge},
    relooper::ReSeq,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::Range,
};

pub type Label = usize;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Edge {
    Entry,
    Exit,
    Static(Label, bool), // bool mean true\false branch. If node has only one successor it is false branch
    Dynamic,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Block {
    pub label: Label,
    pub code: Range<usize>,
    pub succ: BTreeSet<Edge>,
    pub is_jumpdest: bool,       // if true than this block have jumpdest as 1st opcode
    pub is_dyn: bool,            // if true than this block have dynamic edge.
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
            succ: BTreeSet::new(),
            is_dyn: false,
            is_jumpdest: false,
            closed: false,
        }
    }

    #[allow(dead_code)]
    pub fn code<'a>(&self, program: &'a [Opcode]) -> &'a [Opcode] {
        &program[self.code.start..self.code.end]
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


pub fn analyze_cfg(program: &Program) -> Cfg<EvmLabel<usize>> {
    let mut blocks: HashMap<Label, Block> = HashMap::new();
    let mut pc: usize = 0; // program counter
    let entry = pc;
    let mut prev_op: Option<&Opcode> = None;
    blocks.insert(pc, Block::at(pc, 0, 0));
    let mut block: &mut Block = blocks.get_mut(&pc).unwrap();
    for (op_idx, op) in program.0.iter().enumerate() {
        use Opcode::*;
        match op {
            JUMP => {
                block.code.end = op_idx + 1;
                match prev_op {
                    Some(PUSH1(pc)) => block.add_succ(Edge::Static(usize::from(*pc), false)),
                    Some(PUSHn(_, pc, _)) => block.add_succ(Edge::Static(pc.as_usize(), false)),
                    Some(_) => block.is_dyn = true,
                    None => unreachable!(),
                }
                block.close();
            }
            JUMPI => {
                block.code.end = op_idx + 1;
                match prev_op {
                    Some(PUSH1(pc)) => block.add_succ(Edge::Static(usize::from(*pc), true)),
                    Some(PUSHn(_, pc, _)) => block.add_succ(Edge::Static(pc.as_usize(), true)),
                    Some(_) => block.is_dyn = true,
                    None => unreachable!(),
                }
                block.add_succ(Edge::Static(pc + 1, false));
                block.close();
            }
            JUMPDEST => {
                if !block.closed {
                    // no JUMP/JUMPI ending the previous block
                    assert!(block.succ.is_empty());
                    block.add_succ(Edge::Static(pc, false));
                    block.close();
                }
                blocks.insert(pc, Block::at(pc, op_idx, op_idx + 1));
                block = blocks.get_mut(&pc).unwrap();
                block.is_jumpdest = true;   // I am not sure if it should be here...
            }
            _ => {
                if block.closed {
                    blocks.insert(pc, Block::at(pc, op_idx, op_idx + 1));
                    block = blocks.get_mut(&pc).unwrap();
                }
                block.code.end = op_idx + 1;
                if op.is_halt() {
                    block.add_succ(Edge::Exit);
                    block.close();
                }
            }
        };
        prev_op = Some(op);
        pc += op.size();
    }
    block.add_succ(Edge::Exit);
    // move edges to CFGEdge
    let mut edges: HashMap<EvmLabel<usize>, CfgEdge<EvmLabel<usize>>> = HashMap::default();
    let entry_block = blocks.get(&entry).unwrap();
    let entry_evm_label = EvmLabel::<usize> {
        cfg_label: entry_block.label,
        is_dynamic: entry_block.is_dyn,
        is_jumpdest: entry_block.is_jumpdest,
        code_begin: entry_block.code.start,
        code_end: entry_block.code.end,
    };
    let mut evm_labels: HashMap<Label, EvmLabel<usize>> = HashMap::default();
    evm_labels = blocks
        .into_iter()
        .map(|(label, block)| {
            (
                label,
                EvmLabel {
                    cfg_label: block.label,
                    is_dynamic: block.is_dyn,
                    is_jumpdest: block.is_jumpdest,
                    code_begin: entry_block.code.start,
                    code_end: entry_block.code.end,
                },
            )
        })
        .collect();
    for (cfg_label, evm_label) in &evm_labels {
        if blocks.get(cfg_label).unwrap().succ.len() == 1 {
            let edge = blocks.get(cfg_label).unwrap().succ.into_iter().next().unwrap();
            match edge {
                Edge::Static(dest, cond) => {
                    if cond {
                        panic!("This edge must be uncond!!!");
                    }
                    edges.insert(*evm_label, CfgEdge::Uncond(*evm_labels.get(&dest).unwrap()));
                }
                _ => {}
            }
        } else if blocks.get(cfg_label).unwrap().succ.len() == 2 {
            let cond_dest: EvmLabel<usize>;
            let uncond_dest: EvmLabel<usize>;
            for edge in &blocks.get(cfg_label).unwrap().succ {
                match edge {
                    Edge::Static(dest, cond) => {
                        if *cond {
                            cond_dest = *evm_labels.get(dest).unwrap();
                        } else {
                            uncond_dest = *evm_labels.get(dest).unwrap();
                        }
                    }
                    _ => {}
                }
            }
            edges.insert(*evm_label, CfgEdge::Cond(cond_dest, uncond_dest));
        }
    }
    Cfg::from_edges(entry_evm_label, &edges).unwrap()
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FinalLabel {
    pub label: usize,
    pub code_begin: usize,
    pub code_end: usize,
}

use relooper::graph::supergraph::SLabel;
use relooper::graph::caterpillar::CaterpillarLabel;

pub fn relooped_cfg(cfg: Cfg<EvmLabel<usize>>) -> ReSeq<SLabel<CaterpillarLabel<FinalLabel>>> {
    todo!();
}
