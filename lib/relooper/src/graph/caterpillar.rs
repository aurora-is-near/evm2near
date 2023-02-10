use super::cfg::{Cfg, CfgEdge, CfgLabel};

use std::fmt::Display;
use std::hash::Hash;

#[derive(PartialOrd, PartialEq, Clone, Copy, Hash, Eq, Ord)]
pub struct EvmCfgLabel<T> {
    pub cfg_label: T,
    pub is_dynamic: bool,
    pub is_jumpdest: bool,
}

impl<T: Display> Display for EvmCfgLabel<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.cfg_label,
            if self.is_dynamic { "d" } else { "" },
            if self.is_jumpdest { "j" } else { "" }
        )
    }
}

#[derive(PartialOrd, PartialEq, Clone, Copy, Hash, Eq, Ord, Debug)]
pub enum CaterpillarLabel<T> {
    Original(T),
    Generated(T), // (unique_id, offset of associated jumpdest)
}

impl<T: Copy> CaterpillarLabel<T> {
    fn label(&self) -> T {
        match self {
            Self::Original(l) => *l,
            Self::Generated(l) => *l,
        }
    }
}

impl<T: Display> Display for CaterpillarLabel<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            CaterpillarLabel::Generated(offset) => write!(f, "g{}", offset),
            CaterpillarLabel::Original(id) => write!(f, "{}", id),
        }
    }
}

pub fn unfold_dyn_edges<T: CfgLabel>(cfg: &Cfg<EvmCfgLabel<T>>) -> Cfg<CaterpillarLabel<T>> {
    let mut cat_cfg: Cfg<CaterpillarLabel<T>> =
        cfg.map_label(|&label| CaterpillarLabel::Original(label.cfg_label));
    let dyn_nodes: Vec<_> = cfg.nodes().into_iter().filter(|l| l.is_dynamic).collect();

    if dyn_nodes.is_empty() {
        return cat_cfg;
    }

    let jumpdests: Vec<_> = cfg.nodes().into_iter().filter(|l| l.is_jumpdest).collect();
    match &jumpdests[..] {
        [] => cat_cfg,
        [single] => {
            let single_edge = CaterpillarLabel::Original(single.cfg_label);
            for d in dyn_nodes {
                cat_cfg.add_edge_or_promote(CaterpillarLabel::Original(d.cfg_label), single_edge);
            }
            cat_cfg
        }
        [first, second, rest @ ..] => {
            let last_dyn_node = CaterpillarLabel::Generated(first.cfg_label);
            cat_cfg.add_edge(
                last_dyn_node,
                CfgEdge::Cond(
                    CaterpillarLabel::Original(first.cfg_label),
                    CaterpillarLabel::Original(second.cfg_label),
                ),
            );
            let first_dyn_node = rest.iter().fold(last_dyn_node, |dyn_node, jumpdest| {
                let j_gen = CaterpillarLabel::Generated(jumpdest.cfg_label);
                cat_cfg.add_edge(
                    j_gen,
                    CfgEdge::Cond(CaterpillarLabel::Original(jumpdest.cfg_label), dyn_node),
                );
                j_gen
            });
            for d in dyn_nodes {
                cat_cfg
                    .add_edge_or_promote(CaterpillarLabel::Original(d.cfg_label), first_dyn_node);
            }
            cat_cfg
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::caterpillar::{unfold_dyn_edges, EvmCfgLabel};
    use crate::graph::cfg::{Cfg, CfgEdge};
    use std::collections::HashMap;

    #[test]
    pub fn test_caterpillar() {
        let mut nodes: Vec<EvmCfgLabel<usize>> = Vec::default();
        for i in 0..10 {
            nodes.push(EvmCfgLabel {
                cfg_label: i,
                is_dynamic: i % 3 == 0,
                is_jumpdest: i % 2 == 0,
            });
        }
        nodes[0].is_dynamic = false;
        let mut edges: HashMap<EvmCfgLabel<usize>, CfgEdge<EvmCfgLabel<usize>>> =
            HashMap::default();
        edges.insert(nodes[0], CfgEdge::Cond(nodes[1], nodes[2]));
        edges.insert(nodes[1], CfgEdge::Uncond(nodes[3]));
        edges.insert(nodes[2], CfgEdge::Uncond(nodes[3]));
        edges.insert(nodes[4], CfgEdge::Cond(nodes[5], nodes[6]));
        edges.insert(nodes[5], CfgEdge::Uncond(nodes[6]));
        edges.insert(nodes[8], CfgEdge::Cond(nodes[7], nodes[9]));
        let cfg = Cfg::from_edges(nodes[0], &edges).unwrap();
        let _caterpillar = unfold_dyn_edges(&cfg);
    }
}
