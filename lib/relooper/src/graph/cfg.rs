use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
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
    pub(crate) entry: CfgLabel,
}

impl Cfg {
    pub fn from_edges(
        edges: Vec<(CfgLabel, CfgEdge)>,
        entry: CfgLabel,
    ) -> Result<Self, &'static str> {
        let mut out_edges = HashMap::new();
        let mut nodes = HashSet::new();
        for (from, edge) in edges {
            let old_val = out_edges.insert(from, edge);
            if old_val.is_some() {
                return Err("repeating source node");
            }
            nodes.insert(from);
            nodes.extend(edge.to_vec());
        }

        for n in nodes {
            out_edges.entry(n).or_insert(Terminal);
        }

        Ok(Self { out_edges, entry })
    }

    pub fn from_strings(strings: Vec<String>) -> Result<Self, &'static str> {
        let entry = strings
            .get(0)
            .ok_or("no entry line specified")
            .and_then(|e_str| e_str.parse::<usize>().map_err(|x| "invalid entry format"))?;
        let edges: Vec<_> = strings
            .iter()
            .skip(1)
            .map(|s| {
                let split: Vec<_> = s.split(" ").map(|s| s.parse::<usize>()).collect();
                let split_r: Result<Vec<_>, _> = split.into_iter().collect();

                split_r
                    .map_err(|_err| "usize parse error")
                    .and_then(|split_v| {
                        let from = split_v[0];

                        let edge = match split_v[1..] {
                            [to] => Ok(Uncond(to)),
                            [t, f] => Ok(Cond(t, f)),
                            _ => Err("invalid edge description"),
                        };
                        edge.map(|e| (from, e))
                    })
            })
            .collect();
        let edges_result: Result<Vec<(CfgLabel, CfgEdge)>, _> = edges.into_iter().collect();

        edges_result.and_then(|edges| Self::from_edges(edges, entry))
    }

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
