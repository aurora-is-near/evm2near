use crate::cfg::CfgEdge::{Cond, Terminal, Uncond};
use std::collections::{HashMap, HashSet};
use std::iter::once;

pub type CfgLabel = usize;
#[derive(Copy, Clone)]
pub enum CfgEdge {
    Uncond(CfgLabel),
    Cond(CfgLabel, CfgLabel),
    Terminal,
}

impl CfgEdge {
    pub fn to_vec(&self) -> Vec<CfgLabel> {
        match self {
            Uncond(u) => vec![*u],
            Cond(cond, fallthrough) => vec![*cond, *fallthrough],
            Terminal => vec![],
        }
    }
}

pub struct Cfg {
    pub(crate) out_edges: HashMap<CfgLabel, CfgEdge>,
}

impl From<Vec<(CfgLabel, CfgLabel, bool)>> for Cfg {
    fn from(edges: Vec<(CfgLabel, CfgLabel, bool)>) -> Self {
        let mut temp_edges: HashMap<CfgLabel, Vec<(CfgLabel, bool)>> = HashMap::new();
        for &(from, to, is_conditional) in &edges {
            temp_edges
                .entry(from)
                .or_default()
                .push((to, is_conditional));
        }

        let mut out_edges = HashMap::new();
        for node in edges.iter().flat_map(|(f, t, _)| vec![f, t]) {
            let edge = temp_edges.get(node).map_or(
                Terminal, // assuming that every non-terminal node from input graph has edge out and every terminal node doesnt
                |to| match to[..] {
                    [(uncond, false)] => Uncond(uncond),
                    [(true_br, true), (false_br, false)] | [(false_br, false), (true_br, true)] => {
                        Cond(true_br, false_br)
                    }
                    _ => panic!("unexpected edges configuration"),
                },
            );
            out_edges.insert(*node, edge);
        }

        Cfg { out_edges }
    }
}

impl Cfg {
    pub fn nodes(&self) -> HashSet<CfgLabel> {
        self.out_edges
            .iter()
            .flat_map(|(&from, &to)| once(from).chain(to.to_vec()))
            .collect()
    }

    pub fn edge(&self, label: CfgLabel) -> &CfgEdge {
        self.out_edges
            .get(&label)
            .expect("any node should have outgoing edges")
    }

    pub fn children(&self, label: CfgLabel) -> HashSet<CfgLabel> {
        self.out_edges
            .get(&label)
            .into_iter()
            .flat_map(|edge| edge.to_vec())
            .collect()
    }
}
