use super::cfg::{Cfg, CfgEdge, CfgLabel};

use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::hash::Hash;

#[derive(PartialOrd, PartialEq, Clone, Copy, Hash, Eq, Ord)]
pub struct EvmLabel<T> {
    cfg_label: T,
    is_dynamic: bool,
    is_jumpdest: bool,
}

#[derive(PartialOrd, PartialEq, Clone, Copy, Hash, Eq, Ord)]
pub enum CaterpillarLabel<T> {
    Original(T),
    Generated(T), // (unique_id, offset of associated jumpdest)
}

impl<T: Display> Display for CaterpillarLabel<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            CaterpillarLabel::Generated(offset) => write!(f, "g{}",offset),
            CaterpillarLabel::Original(id) => write!(f, "{}", id),
        }
    }
}

pub fn unfold_dyn_edges<T: CfgLabel>(cfg: Cfg<EvmLabel<T>>) -> Cfg<CaterpillarLabel<T>> {
    let mut cat_cfg = cfg.map_label(|&label| CaterpillarLabel::Original(label.cfg_label));

    let dyn_nodes: Vec<_> = cfg.nodes().into_iter().filter(|l| l.is_dynamic).collect();
    let jumpdests: Vec<_> = cfg.nodes().into_iter().filter(|l| l.is_jumpdest).collect();

    match &jumpdests[..] {
        [] => cat_cfg,
        [single] => {
            let single_edge = CaterpillarLabel::Original(single.cfg_label);
            for d in dyn_nodes {
                cat_cfg.add_edge(CaterpillarLabel::Original(d.cfg_label), CfgEdge::Uncond(single_edge));
            }
            cat_cfg
        },
        [first, second, rest @ ..] => {
            let last = (CaterpillarLabel::Original(first.cfg_label), CaterpillarLabel::Original(second.cfg_label));

            let (f_c, f_u) = rest.iter().fold(last, |(c, u), jumpdest| {
                let new_edge = CfgEdge::Cond(c, u);
                let j_gen = CaterpillarLabel::Generated(jumpdest.cfg_label);
                cat_cfg.add_edge(j_gen, new_edge);

                (CaterpillarLabel::Original(jumpdest.cfg_label), j_gen)
            });

            for d in dyn_nodes {
                cat_cfg.add_edge(CaterpillarLabel::Original(d.cfg_label), CfgEdge::Cond(f_c, f_u));
            }

            cat_cfg
        }
    }
}


//TODO temporary disabled, will fix a bit later
// #[cfg(test)]
// mod tests {
//     use crate::graph::caterpillar::{unfold_dyn_edges, EvmLabel};
//     use crate::graph::cfg::{Cfg, CfgEdge};
//     use crate::graph::EnrichedCfg;
//     use std::collections::HashMap;

//     #[test]
//     pub fn test_caterpillar() {
//         let mut nodes: Vec<EvmLabel> = Vec::default();
//         for i in 0..10 {
//             nodes.push(EvmLabel {
//                 cfg_label: i,
//                 is_dynamic: i % 3 == 0,
//                 is_jumpdest: i % 2 == 0,
//             });
//         }
//         nodes[0].is_dynamic = false;
//         let mut edges: HashMap<EvmLabel, CfgEdge<EvmLabel>> = HashMap::default();
//         edges.insert(nodes[0], CfgEdge::Cond(nodes[1], nodes[2]));
//         edges.insert(nodes[1], CfgEdge::Uncond(nodes[3]));
//         edges.insert(nodes[2], CfgEdge::Uncond(nodes[3]));
//         edges.insert(nodes[4], CfgEdge::Cond(nodes[5], nodes[6]));
//         edges.insert(nodes[5], CfgEdge::Uncond(nodes[6]));
//         edges.insert(nodes[8], CfgEdge::Cond(nodes[7], nodes[9]));
//         let cfg = Cfg::from_edges(nodes[0], &edges).unwrap();
//         let caterpillar = unfold_dyn_edges(cfg);

//         println!("Caterpillar:");
//         for (label, edge) in &caterpillar.out_edges {
//             match edge {
//                 CfgEdge::Cond(cond, uncond) => {
//                     println!("CEdge from {}. cond = {}, uncond = {}", label, cond, uncond);
//                 }
//                 CfgEdge::Uncond(uncond) => {
//                     println!("UEdge from {} to {}", label, uncond);
//                 }
//                 CfgEdge::Terminal => {
//                     println!("Terminal edge from {}", label);
//                 }
//             }
//         }
//         println!("End of caterpillar");

//         let e_graph = EnrichedCfg::new(caterpillar);
//         let dot_lines: Vec<String> = vec![
//             "digraph {".to_string(),
//             e_graph.cfg_to_dot("reduced"),
//             "}".to_string(),
//         ];
//         std::fs::write("caterpillar.dot", dot_lines.join("\n")).expect("fs error");
//     }
// }
