use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
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

#[derive(Clone)]
pub struct Cfg<TLabel: CfgLabel> {
    pub(crate) entry: TLabel,
    pub(crate) out_edges: HashMap<TLabel, CfgEdge<TLabel>>,
}

impl<TLabel: CfgLabel> Cfg<TLabel> {
    pub fn from_edges(
        edges: Vec<(TLabel, CfgEdge<TLabel>)>,
        entry: TLabel,
    ) -> Result<Self, String> {
        let mut out_edges = HashMap::new();
        let mut nodes = HashSet::new();
        for (from, edge) in edges {
            let old_val = out_edges.insert(from, edge);
            if old_val.is_some() {
                return Err("repeating source node".to_string());
            }
            nodes.insert(from);
            nodes.extend(edge.to_vec());
        }

        for n in nodes {
            out_edges.entry(n).or_insert(Terminal);
        }

        Ok(Self { out_edges, entry })
    }

    pub fn out_edges(&self) -> HashMap<TLabel, Vec<TLabel>> {
        self.out_edges
            .iter()
            .map(|(&f, e)| (f, e.to_vec()))
            .collect()
    }

    pub fn in_edges(&self) -> HashMap<TLabel, Vec<TLabel>> {
        let mut back_edges: HashMap<TLabel, Vec<TLabel>> = HashMap::default();

        for (&from, &to_edge) in &self.out_edges {
            for to in to_edge.to_vec() {
                back_edges.entry(to).or_default().push(from);
            }
        }

        back_edges
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
