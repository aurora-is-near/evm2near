// This is free and unencumbered software released into the public domain.

use evm_rs::{Opcode, Program};
use relooper::graph::{
    caterpillar::{unfold_dyn_edges, EvmCfgLabel},
    cfg::{Cfg, CfgEdge},
    relooper::ReSeq,
};
use std::{
    collections::{BTreeSet, HashMap},
    ops::Range,
};

use relooper::graph::caterpillar::CaterpillarLabel;
use relooper::graph::relooper::reloop;
use relooper::graph::supergraph::{reduce, SLabel};

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
use std::fmt::Display;
impl Display for EvmLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "code_start_{}_code_end_{}", self.code_start, self.code_end)
    }
}


fn relooped_cfg(cfg: Cfg<EvmCfgLabel<EvmLabel>>) -> ReSeq<SLabel<CaterpillarLabel<EvmLabel>>> {
    
    std::fs::write("cfg.dot", format!("digraph {{{}}}", cfg.cfg_to_dot("generated")).to_string()).expect("fs error");
    let undyned = unfold_dyn_edges(&cfg);
    std::fs::write("cater.dot", format!("digraph {{{}}}", undyned.cfg_to_dot("cater")).to_string()).expect("fs error");
    let reduced = reduce(&undyned);
    reloop(&reduced)
}

pub fn analyze_cfg(program: &Program) -> ReSeq<SLabel<CaterpillarLabel<EvmLabel>>> {
    let mut bc_offs: usize = 0; // program counter
    let entry = bc_offs;
    let mut node_info: HashMap<usize, (bool, bool)> = Default::default(); // label => (is_jumpdest, is_dynamic);
    let mut code_ranges: HashMap<usize, Range<usize>> = Default::default();
    let mut prev_op: Option<&Opcode> = None;
    let mut current_label = entry;
    let mut cfg = Cfg::from_edges(entry, &Default::default()).unwrap();
    let mut closed: bool = false;
    let mut was_jumpdest = false;
    let mut begin_idx: usize = 0;
    let mut was_dynamic = false;

    // rewrite this closure to macros
    // let start_new_block = |next_opode_idx: &usize| {
    //     node_info.insert(current_label, (was_jumpdest, was_dynamic));
    //     code_ranges.insert(current_label, begin_idx..*next_opode_idx);
    //     begin_idx = *next_opode_idx;
    //     was_dynamic = false;
    //     was_jumpdest = false;
    //     current_label = bc_offs;
    //     closed = false;
    // };
    macro_rules! start_new_block {
        ($next_opode_idx: expr) => {
            
            node_info.insert(current_label, (was_jumpdest, was_dynamic));
            code_ranges.insert(current_label, begin_idx..$next_opode_idx);

            begin_idx = $next_opode_idx;
            was_dynamic = false;
            was_jumpdest = false;
            current_label = bc_offs;
            closed = false; 
        }; 
    } 

    for (op_idx, op) in program.0.iter().enumerate() {
        // let next_opode_idx = op_idx + 1;
        use Opcode::*;
        match op {
            JUMP => {
                match prev_op {
                    Some(PUSH1(addr)) => {
                        cfg.add_edge(current_label, CfgEdge::Uncond(usize::from(*addr)))
                    }
                    Some(PUSHn(_, addr, _)) => {
                        cfg.add_edge(current_label, CfgEdge::Uncond(addr.as_usize()))
                    }
                    Some(_) => {
                        was_dynamic = true;
                    }
                    None => unreachable!(),
                }
                closed = true;
            }
            JUMPI => {
                match prev_op {
                    Some(PUSH1(addr)) => cfg.add_edge(
                        current_label,
                        CfgEdge::Cond(usize::from(*addr), (bc_offs + 1).into()),
                    ),
                    Some(PUSHn(_, addr, _)) => cfg.add_edge(
                        current_label,
                        CfgEdge::Cond(addr.as_usize(), (bc_offs + 1).into()),
                    ),
                    Some(_) => {
                        was_dynamic = true;
                    }
                    None => unreachable!(),
                }
                closed = true;
            }
            JUMPDEST => {
                if !closed {
                    cfg.add_edge(current_label, CfgEdge::Uncond(bc_offs));
                }
                start_new_block!(op_idx + 1);
                was_jumpdest = true;
            }
            _ => {
                if closed {
                    start_new_block!(op_idx + 1);
                }
                if op.is_halt() {
                    cfg.add_edge(current_label, CfgEdge::Terminal);
                    start_new_block!(op_idx + 2);
                    // closed = true;
                }
            }
        }
        prev_op = Some(op);
        bc_offs += op.size();
    }
    let opcodes: Vec<String> = program.clone().0.into_iter().map(|opcode| opcode.to_string()).collect();
    std::fs::write("opcodes.evm", opcodes.join("\n")).expect("fs error");
    // start_new_block!(program.0.len());

    // code_ranges.insert(current_label, current_label..pc);
    // node_info.insert(current_label, (was_jumpdest, false));
    // cfg.add_edge(current_label, CfgEdge::Terminal);










    println!("Existing labels:");
    for node in &cfg.nodes() {
        println!("{}", node);
    }
    println!("There is all labels");
    let with_ranges = cfg.map_label(|int| EvmLabel {
        label: *int,
        code_start: code_ranges.get(&int).expect(format!("no code ranges for {}", *int).as_str()).start,
        code_end: code_ranges.get(&int).unwrap().end,
    });
    let with_flags = with_ranges.map_label(|evm_label| EvmCfgLabel {
        cfg_label: *evm_label,
        is_jumpdest: node_info.get(&evm_label.label).unwrap().0,
        is_dynamic: node_info.get(&evm_label.label).unwrap().1,
    });
    relooped_cfg(with_flags)
}
