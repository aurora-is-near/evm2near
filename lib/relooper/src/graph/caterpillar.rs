use super::cfg::{Cfg, CfgEdge, CfgLabel};
use super::EnrichedCfg;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;

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

impl Display for CaterpillarLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            CaterpillarLabel::generated(id, offset) => write!(f, "{}_{}", id, offset),
            CaterpillarLabel::original(id) => write!(f, "{}", id),
        }
    }
}

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

#[test]
pub fn test_caterpillar() {
    let mut nodes: Vec<EvmLabel> = Vec::default();
    for i in 0..10 {
        nodes.push(EvmLabel {
            cfg_label: i,
            is_dynamic: i % 3 == 0,
            is_jumpdest: i % 2 == 0,
        });
    }
    nodes[0].is_dynamic = false;
    let mut edges: HashMap<EvmLabel, CfgEdge<EvmLabel>> = HashMap::default();
    edges.insert(nodes[0], CfgEdge::Cond(nodes[1], nodes[2]));
    edges.insert(nodes[1], CfgEdge::Uncond(nodes[3]));
    edges.insert(nodes[2], CfgEdge::Uncond(nodes[3]));
    edges.insert(nodes[4], CfgEdge::Cond(nodes[5], nodes[6]));
    edges.insert(nodes[5], CfgEdge::Uncond(nodes[6]));
    edges.insert(nodes[8], CfgEdge::Cond(nodes[7], nodes[9]));
    let cfg = Cfg::from_edges(nodes[0], &edges).unwrap();
    let caterpillar = make_caterpillar(cfg);

    println!("Caterpillar:");
    for (label, edge) in &caterpillar.out_edges {
        match edge {
            CfgEdge::Cond(cond, uncond) => {
                println!("CEdge from {}. cond = {}, uncond = {}", label, cond, uncond);
            }
            CfgEdge::Uncond(uncond) => {
                println!("UEdge from {} to {}", label, uncond);
            }
            CfgEdge::Terminal => {
                println!("Terminal edge from {}", label);
            }
        }
    }
    println!("End of caterpillar");

    let e_graph = EnrichedCfg::new(caterpillar);
    let dot_lines: Vec<String> = vec![
        "digraph {".to_string(),
        e_graph.cfg_to_dot("reduced"),
        "}".to_string(),
    ];
    std::fs::write("caterpillar.dot", dot_lines.join("\n")).expect("fs error");
}
