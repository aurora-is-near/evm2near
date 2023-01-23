use super::cfg::{Cfg, CfgEdge};

use std::collections::HashMap;
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
    Original(usize),
    Generated(usize, usize), // (unique_id, offset of associated jumpdest)
}

impl CfgLabel for CaterpillarLabel {}

impl Display for CaterpillarLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            CaterpillarLabel::Generated(id, offset) => write!(f, "{}_{}", id, offset),
            CaterpillarLabel::Original(id) => write!(f, "{}", id),
        }
    }
}

pub fn unfold_dyn_edges(cfg: Cfg<EvmLabel>) -> Cfg<CaterpillarLabel> {
    let to_caterpillar_edge = |(label, edge): (&EvmLabel, &CfgEdge<EvmLabel>)| -> (CaterpillarLabel, CfgEdge<CaterpillarLabel>) {
        match edge {
            CfgEdge::Cond(cond, uncond) => {
                (
                    CaterpillarLabel::Original(label.cfg_label),
                    CfgEdge::Cond(
                        CaterpillarLabel::Original(cond.cfg_label),
                        CaterpillarLabel::Original(uncond.cfg_label),
                    )
                )
            }
            CfgEdge::Uncond(uncond) => {
                (
                    CaterpillarLabel::Original(label.cfg_label),
                    CfgEdge::Uncond(CaterpillarLabel::Original(uncond.cfg_label)),
                )
            }
            CfgEdge::Terminal => {
                (
                    CaterpillarLabel::Original(label.cfg_label),
                    CfgEdge::Terminal,
                )
            }
        }
    };
    let mut outedges: HashMap<CaterpillarLabel, CfgEdge<CaterpillarLabel>> = cfg
        .out_edges
        .iter()
        .filter(|(label, _)| !label.is_dynamic)
        .map(to_caterpillar_edge)
        .collect();
    let jumpdests: Vec<usize> = cfg
        .out_edges
        .iter()
        .filter_map(|(label, _)| {
            if label.is_jumpdest {
                Some(label.cfg_label)
            } else {
                None
            }
        })
        .collect();
    let new_nodes: Vec<CaterpillarLabel> = jumpdests
        .iter()
        .enumerate()
        .map(|(index, jumpdest)| CaterpillarLabel::Generated(index, *jumpdest))
        .collect();
    for label in cfg.out_edges.keys() {
        if !label.is_dynamic {
            continue;
        }
        outedges.insert(
            CaterpillarLabel::Original(label.cfg_label),
            CfgEdge::Uncond(new_nodes[0]),
        );
    }
    let node_pairs = new_nodes.iter().zip(new_nodes.iter().skip(1));
    for (node, next_node) in node_pairs {
        let offset = match node {
            CaterpillarLabel::Generated(_id, offset) => offset,
            CaterpillarLabel::Original(_) => panic!("It must be Generated"),
        };
        outedges.insert(
            *node,
            CfgEdge::Cond(CaterpillarLabel::Original(*offset), *next_node),
        );
    }
    let offset = match new_nodes.last().unwrap() {
        CaterpillarLabel::Original(_orig) => {
            panic!("It must be Generated");
        }
        CaterpillarLabel::Generated(_id, offst) => offst,
    };
    outedges.insert(
        *new_nodes.last().unwrap(),
        CfgEdge::Uncond(CaterpillarLabel::Original(*offset)),
    );
    let res: Cfg<CaterpillarLabel> = Cfg::<CaterpillarLabel>::from_edges(
        CaterpillarLabel::Original(cfg.entry.cfg_label),
        &outedges,
    )
    .unwrap();
    res
}

#[cfg(test)]
mod tests {
    use crate::graph::caterpillar::{unfold_dyn_edges, EvmLabel};
    use crate::graph::cfg::{Cfg, CfgEdge};
    use crate::graph::EnrichedCfg;
    use std::collections::HashMap;

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
        let caterpillar = unfold_dyn_edges(cfg);

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
}
