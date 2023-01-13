use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
use anyhow::ensure;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::iter::once;

pub trait CfgLabel: Copy + Hash + Eq + Ord + Display + Debug {}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CfgEdge<TLabel: CfgLabel> {
    Uncond(TLabel),
    Cond(TLabel, TLabel),
    Terminal,
}

impl<TLabel: CfgLabel> CfgEdge<TLabel> {
    pub fn to_vec(&self) -> Vec<TLabel> {
        match self {
            Uncond(u) => vec![*u],
            Cond(cond, fallthrough) => vec![*cond, *fallthrough],
            Terminal => vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Cfg<TLabel: CfgLabel> {
    pub(crate) entry: TLabel,
    pub(crate) out_edges: HashMap<TLabel, CfgEdge<TLabel>>,
    pub(crate) in_edges: HashMap<TLabel, HashSet<TLabel>>,
}

impl<TLabel: CfgLabel> Cfg<TLabel> {
    pub fn from_edges(
        edges: Vec<(TLabel, CfgEdge<TLabel>)>,
        entry: TLabel,
    ) -> Result<Self, anyhow::Error> {
        let mut out_edges = HashMap::new();
        let mut nodes = HashSet::new();
        for (from, edge) in edges {
            let old_val = out_edges.insert(from, edge);

            ensure!(old_val.is_none(), "repeating source node");

            nodes.insert(from);
            nodes.extend(edge.to_vec());
        }

        for n in nodes {
            out_edges.entry(n).or_insert(Terminal);
        }

        let mut in_edges: HashMap<TLabel, HashSet<TLabel>> = HashMap::default();

        for (&from, to_edge) in &out_edges {
            for to in to_edge.to_vec() {
                in_edges.entry(to).or_default().insert(from);
            }
        }

        Ok(Self {
            entry,
            out_edges,
            in_edges,
        })
    }

    pub fn out_edges(&self) -> HashMap<TLabel, Vec<TLabel>> {
        self.out_edges
            .iter()
            .map(|(&f, e)| (f, e.to_vec()))
            .collect()
    }

    pub fn nodes(&self) -> HashSet<TLabel> {
        self.out_edges
            .iter()
            .flat_map(|(&from, &to)| once(from).chain(to.to_vec()))
            .collect()
    }

    pub fn edge(&self, label: TLabel) -> &CfgEdge<TLabel> {
        self.out_edges
            .get(&label)
            .expect("any node should have outgoing edges")
    }

    pub fn children(&self, label: TLabel) -> HashSet<TLabel> {
        self.out_edges
            .get(&label)
            .into_iter()
            .flat_map(|edge| edge.to_vec())
            .collect()
    }
}
