// This is free and unencumbered software released into the public domain.

use evm_rs::{Opcode, Program};
use relooper::graph::cfg::{Cfg, CfgEdge};
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    ops::Range,
};

/// This struct represents offset of instruction in EVM bytecode.
/// Also look at docs of Idx struct
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offs(pub usize);

/// This struct represents the serial number of instruction.
/// Serial number and offset are two different numbers
/// If you have EVM bytecode
/// 0x00  PUSH
/// 0x03  PUSH
/// 0x06  ADD
///
/// Then,  first PUSH will have idx = 0 and offs = 0x00, second idx = 1 and offs = 0x03,
///  ADD will have idx = 2 and offs = 0x06
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Idx(pub usize);

impl Debug for Offs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Offs({})", self)
    }
}

impl Display for Offs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

impl Display for Idx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeInfo {
    pub is_jumpdest: bool,
    pub is_dynamic: bool,
}

/// Represents either original node or artificial `Dynamic` node used for dynamic edges translation.
/// During codegen phase, all dynamic node edges will be converted to table branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CfgNode<T> {
    Orig(T),
    Dynamic,
}

impl<T: Display> Display for CfgNode<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Orig(offs) => write!(f, "{}", offs),
            Self::Dynamic => write!(f, "dynamic"),
        }
    }
}

#[derive(Debug)]
pub struct BasicCfg {
    pub cfg: Cfg<CfgNode<Offs>>,
    pub code_ranges: HashMap<Offs, Range<Idx>>,
}

pub fn basic_cfg(program: &Program) -> BasicCfg {
    struct BlockStart {
        start_offs: Offs,
        start_idx: Idx,
    }

    let mut cfg = Cfg::new(CfgNode::Orig(Offs(0)));
    let mut code_ranges: HashMap<Offs, Range<Idx>> = Default::default();
    let mut jumpdests: Vec<Offs> = Default::default();

    let mut curr_offs = Offs(0);
    let mut block_start: Option<BlockStart> = None;
    let mut prev_op: Option<&Opcode> = None;
    let mut next_idx = Idx(1);

    for (curr_idx, op) in program.0.iter().enumerate().map(|(i, op)| (Idx(i), op)) {
        next_idx = Idx(curr_idx.0 + 1);
        let next_offs = Offs(curr_offs.0 + op.size());

        use Opcode::*;
        block_start = match op {
            JUMP | JUMPI => {
                let BlockStart {
                    start_offs,
                    start_idx,
                } = block_start.expect("block should be present at any jump opcode");

                let label = match prev_op {
                    Some(PUSH1(addr)) => Some(Offs(usize::from(*addr))),
                    Some(PUSHn(_, addr, _)) => Some(Offs(addr.as_usize())),
                    Some(_) => None,
                    None => unreachable!(),
                };

                let jump_to = label.map(CfgNode::Orig).unwrap_or(CfgNode::Dynamic);
                let edge = if op == &JUMP {
                    CfgEdge::Uncond(jump_to)
                } else {
                    CfgEdge::Cond(jump_to, CfgNode::Orig(next_offs))
                };
                cfg.add_edge(CfgNode::Orig(start_offs), edge);

                code_ranges.insert(start_offs, start_idx..next_idx);

                None
            }
            JUMPDEST => {
                jumpdests.push(curr_offs);
                if let Some(BlockStart {
                    start_offs,
                    start_idx,
                }) = block_start
                {
                    let edge = CfgEdge::Uncond(CfgNode::Orig(curr_offs));
                    cfg.add_edge(CfgNode::Orig(start_offs), edge);
                    code_ranges.insert(start_offs, start_idx..curr_idx);
                }

                Some(BlockStart {
                    start_offs: curr_offs,
                    start_idx: curr_idx,
                })
            }
            _ => {
                let bs @ BlockStart {
                    start_offs,
                    start_idx,
                } = block_start.unwrap_or(BlockStart {
                    start_offs: curr_offs,
                    start_idx: curr_idx,
                });

                if op.is_halt() {
                    cfg.add_edge(CfgNode::Orig(bs.start_offs), CfgEdge::Terminal);
                    code_ranges.insert(start_offs, start_idx..next_idx);
                    None
                } else {
                    Some(bs)
                }
            }
        };

        curr_offs = next_offs;
        prev_op = Some(op);
    }

    if let Some(BlockStart {
        start_offs,
        start_idx,
    }) = block_start
    {
        code_ranges.insert(start_offs, start_idx..next_idx);
    }

    let jump_table: Vec<_> = jumpdests
        .into_iter()
        .map(|j| (j.0, CfgNode::Orig(j)))
        .collect();
    cfg.add_edge(CfgNode::Dynamic, CfgEdge::Switch(jump_table));

    BasicCfg { cfg, code_ranges }
}
