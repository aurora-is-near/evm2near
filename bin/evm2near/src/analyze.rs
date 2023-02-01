// This is free and unencumbered software released into the public domain.

use evm_rs::{Opcode, Program};
use relooper::graph::{
    caterpillar::{unfold_dyn_edges, EvmCfgLabel},
    cfg::{Cfg, CfgEdge},
    relooper::ReSeq,
};
use std::{collections::HashMap, ops::Range};

use relooper::graph::caterpillar::CaterpillarLabel;
use relooper::graph::relooper::reloop;
use relooper::graph::supergraph::{reduce, SLabel};

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
        write!(f, "{}_to_{}", self.code_start, self.code_end)
    }
}

fn relooped_cfg(cfg: Cfg<EvmCfgLabel<EvmLabel>>) -> ReSeq<SLabel<CaterpillarLabel<EvmLabel>>> {
    std::fs::write(
        "cfg.dot",
        format!("digraph {{{}}}", cfg.cfg_to_dot("generated")),
    )
    .expect("fs error");
    let mut undyned = unfold_dyn_edges(&cfg);
    undyned.strip_unreachable();
    std::fs::write(
        "cater.dot",
        format!("digraph {{{}}}", undyned.cfg_to_dot("cater")),
    )
    .expect("fs error");
    let reduced = reduce(&undyned);
    reloop(&reduced)
}

#[derive(Debug)]
struct BlockStart(usize, usize, bool);

pub fn analyze_cfg(program: &Program) -> ReSeq<SLabel<CaterpillarLabel<EvmLabel>>> {
    let mut cfg = Cfg::from_edges(0, &Default::default()).unwrap();
    let mut node_info: HashMap<usize, (bool, bool)> = Default::default(); // label => (is_jumpdest, is_dynamic);
    let mut code_ranges: HashMap<usize, Range<usize>> = Default::default();

    let mut curr_offs = 0_usize;
    let mut block_start: Option<BlockStart> = None;
    let mut prev_op: Option<&Opcode> = None;

    for (curr_idx, op) in program.0.iter().enumerate() {
        let next_idx = curr_idx + 1;
        let next_offs = curr_offs + op.size();

        use Opcode::*;
        block_start = match op {
            JUMP | JUMPI => {
                let BlockStart(start_offs, start_idx, is_jmpdest) =
                    block_start.expect("block should be present at any jump opcode");

                let label = match prev_op {
                    Some(PUSH1(addr)) => Some(usize::from(*addr)),
                    Some(PUSHn(_, addr, _)) => Some(addr.as_usize()),
                    Some(_) => None,
                    None => unreachable!(),
                };
                let is_dyn = match label {
                    Some(l) => {
                        let edge = if op == &JUMP {
                            CfgEdge::Uncond(l)
                        } else {
                            CfgEdge::Cond(l, next_offs)
                        };
                        cfg.add_edge(start_offs, edge);
                        false
                    }
                    None => true,
                };
                node_info.insert(start_offs, (is_jmpdest, is_dyn));
                code_ranges.insert(start_offs, start_idx..next_idx);

                None
            }
            JUMPDEST => {
                if let Some(BlockStart(start_offs, start_idx, is_jmpdest)) = block_start {
                    let edge = CfgEdge::Uncond(curr_offs);
                    cfg.add_edge(start_offs, edge);
                    node_info.insert(start_offs, (is_jmpdest, false));
                    code_ranges.insert(start_offs, start_idx..curr_idx);
                }

                Some(BlockStart(curr_offs, curr_idx, true))
            }
            _ => {
                let bs @ BlockStart(start_offs, start_idx, is_jmpdest) =
                    block_start.unwrap_or(BlockStart(curr_offs, curr_idx, false));

                if op.is_halt() {
                    cfg.add_edge(bs.0, CfgEdge::Terminal);
                    node_info.insert(start_offs, (is_jmpdest, false));
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

    if let Some(BlockStart(start_offs, start_idx, is_jmpdest)) = block_start {
        node_info.insert(start_offs, (is_jmpdest, false));
        code_ranges.insert(start_offs, start_idx..curr_offs);
    }

    let opcodes: Vec<String> = program
        .clone()
        .0
        .into_iter()
        .enumerate()
        .map(|(idx, opcode)| format!("\t{}\t{}", idx, opcode))
        .collect();
    std::fs::write("opcodes.evm", opcodes.join("\n")).expect("fs error");

    let with_ranges = cfg.map_label(|label| {
        let code_range = code_ranges
            .get(label)
            .unwrap_or_else(|| panic!("no code ranges for {}", *label));
        EvmLabel::new(*label, code_range.start, code_range.end)
    });
    let with_flags = with_ranges.map_label(|evm_label| EvmCfgLabel {
        cfg_label: *evm_label,
        is_jumpdest: node_info.get(&evm_label.label).unwrap().0,
        is_dynamic: node_info.get(&evm_label.label).unwrap().1,
    });
    relooped_cfg(with_flags)
}
