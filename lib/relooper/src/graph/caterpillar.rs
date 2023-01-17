use std::collections::{HashMap, HashSet};

use super::cfg::{Cfg, CfgEdge, CfgLabel};

#[derive(PartialOrd, PartialEq, Clone, Copy, Hash, Eq, Ord)]
pub struct EvmLabel {
    cfg_label: usize,
    is_dynamic: bool,
    is_jumpdest: bool,
}

impl CfgLabel for EvmLabel {}

#[derive(PartialOrd, PartialEq, Clone, Copy, Hash, Eq, Ord)]
pub enum CaterpillarLabel {
    original(usize),
    generated(usize, usize), // (unique_id, offset of associated jumpdest)
}

impl CfgLabel for CaterpillarLabel {}

pub fn make_caterpillar(cfg: Cfg<EvmLabel>) -> Cfg<CaterpillarLabel> {
    let mut outedges: HashMap<CaterpillarLabel, CfgEdge<CaterpillarLabel>> = HashMap::default();
    for (label, edge) in &cfg.out_edges {
        if label.is_dynamic {
            continue;
        }
        match edge {
            CfgEdge::Cond(cond, uncond) => {
                outedges.insert(
                    CaterpillarLabel::original(label.cfg_label),
                    CfgEdge::Cond(
                        CaterpillarLabel::original(cond.cfg_label),
                        CaterpillarLabel::original(uncond.cfg_label),
                    ),
                );
            }
            CfgEdge::Uncond(uncond) => {
                outedges.insert(
                    CaterpillarLabel::original(label.cfg_label),
                    CfgEdge::Uncond(CaterpillarLabel::original(uncond.cfg_label)),
                );
            }
            CfgEdge::Terminal => {
                outedges.insert(
                    CaterpillarLabel::original(label.cfg_label),
                    CfgEdge::Terminal,
                );
            }
        }
    }
    let mut jumpdests: Vec<usize> = Vec::default();
    for (label, _edge) in &cfg.out_edges {
        if label.is_jumpdest {
            jumpdests.push(label.cfg_label);
        }
    }
    let mut new_nodes: Vec<CaterpillarLabel> = Vec::default();
    for i in 0..jumpdests.len() {
        new_nodes.push(CaterpillarLabel::generated(i, jumpdests[i]));
    }
    for (label, _edge) in &cfg.out_edges {
        if !label.is_dynamic {
            continue;
        }
        outedges.insert(
            CaterpillarLabel::original(label.cfg_label),
            CfgEdge::Uncond(new_nodes[0]),
        );
    }
    for i in 0..(new_nodes.len() - 1) {
        let mut offset;
        match new_nodes[i] {
            CaterpillarLabel::original(orig) => {
                panic!("It must be generated");
            }
            CaterpillarLabel::generated(id, offst) => offset = offst,
        }
        outedges.insert(
            new_nodes[i],
            CfgEdge::Cond(CaterpillarLabel::original(offset), new_nodes[i + 1]),
        );
    }
    let mut offset;
    match new_nodes[new_nodes.len() - 1] {
        CaterpillarLabel::original(orig) => {
            panic!("It must be generated");
        }
        CaterpillarLabel::generated(id, offst) => offset = offst,
    }
    outedges.insert(
        new_nodes[new_nodes.len() - 1],
        CfgEdge::Uncond(CaterpillarLabel::original(offset)),
    );
    let res: Cfg<CaterpillarLabel> = Cfg::<CaterpillarLabel>::from_edges(
        CaterpillarLabel::original(cfg.entry.cfg_label),
        &outedges,
    )
    .unwrap();
    res
}
