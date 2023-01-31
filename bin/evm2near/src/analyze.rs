// This is free and unencumbered software released into the public domain.

use evm_rs::{Opcode, Program};
use relooper::graph::{
    caterpillar::{EvmCfgLabel, unfold_dyn_edges},
    cfg::{Cfg, CfgEdge},
    relooper::ReSeq,
};
use std::{
    collections::{BTreeSet, HashMap},
    ops::Range,
};

use relooper::graph::caterpillar::CaterpillarLabel;
use relooper::graph::supergraph::{SLabel, reduce};
use relooper::graph::relooper::reloop;

pub type Label = usize;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvmLabel {
    pub label: usize,
    pub code_start: usize,
    pub code_end: usize,
}

impl EvmLabel {
    fn new(label: usize, code_start: usize, code_end: usize) -> Self {
        Self {
            label,
            code_start,
            code_end,
        }
    }
}



fn relooped_cfg(cfg: Cfg<EvmCfgLabel<EvmLabel>>) -> ReSeq<SLabel<CaterpillarLabel<EvmLabel>>> {
    let undyned = unfold_dyn_edges(&cfg);
    let reduced = reduce(&undyned);
    reloop(&reduced)
}




pub fn analyze_cfg(program: &Program) -> ReSeq<SLabel<CaterpillarLabel<EvmLabel>>> {
    let mut pc: usize = 0; // program counter
    let entry = pc;
    let mut node_info: HashMap<usize, (bool, bool)> = Default::default(); // label => (is_jumpdest, is_dynamic);
    let mut code_ranges: HashMap<usize, Range<usize>> = Default::default();
    let mut prev_op: Option<&Opcode> = None;
    let mut current_label = entry; 
    let mut cfg = Cfg::from_edges(entry, &Default::default()).unwrap();
    let mut closed: bool = false;
    let mut was_jumpdest = false;
    for (op_idx, op) in program.0.iter().enumerate() {
        use Opcode::*;
        match op {
            JUMP => {
                code_ranges.insert(current_label, current_label..(op_idx + 1));
                match prev_op {
                    Some(PUSH1(addr)) => cfg.add_edge(current_label, CfgEdge::Uncond(usize::from(*addr))),
                    Some(PUSHn(_, addr, _)) => cfg.add_edge(current_label, CfgEdge::Uncond(addr.as_usize())),
                    Some(_) => {node_info.insert(current_label, (was_jumpdest, true)); was_jumpdest = false;}
                    None => unreachable!(),
                }
                closed = true;
            }
            JUMPI => {
                code_ranges.insert(current_label, current_label..(op_idx + 1));
                match prev_op {
                    Some(PUSH1(addr)) => cfg.add_edge(current_label, CfgEdge::Cond(usize::from(*addr), (pc + 1).into())),
                    Some(PUSHn(_, addr, _)) => cfg.add_edge(current_label, CfgEdge::Cond(addr.as_usize(), (pc + 1).into())),
                    Some(_) => {node_info.insert(current_label, (was_jumpdest, true)); was_jumpdest = false;}
                    None => unreachable!(),
                }
                closed = true;
            }
            JUMPDEST => {
                if !closed {
                    cfg.add_edge(current_label, CfgEdge::Uncond(pc));
                }
                code_ranges.insert(current_label, current_label..op_idx);
                was_jumpdest = true;
                closed = false;
                current_label = pc;
            }
            _ => {
                if closed {
                    closed = false;
                    current_label = pc;
                    was_jumpdest = false;
                }
                if op.is_halt() {
                    cfg.add_edge(current_label, CfgEdge::Terminal);
                    closed = true;
                }
            }
        }
        prev_op = Some(op);
        pc += op.size();
    }
    let with_ranges = cfg.map_label(|int| EvmLabel {label: *int,
                                                                                  code_start: code_ranges.get(&int).unwrap().start,
                                                                                  code_end: code_ranges.get(&int).unwrap().end});
    let with_flags = with_ranges.map_label(|evm_label| EvmCfgLabel {cfg_label: *evm_label,
                                                                                                                     is_jumpdest: node_info.get(&evm_label.label).unwrap().0,
                                                                                                                     is_dynamic:node_info.get(&evm_label.label).unwrap().1});
    relooped_cfg(with_flags)
}
