use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
use anyhow::ensure;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::once;

mod cfg_mut;
mod cfg_parsing;

pub trait CfgLabel: Copy + Hash + Eq + Ord + Sized {}

impl<T: Copy + Hash + Eq + Ord + Sized> CfgLabel for T {}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum CfgEdge<TLabel> {
    Uncond(TLabel),
    Cond(TLabel, TLabel),
    Terminal,
}

impl<TLabel> CfgEdge<TLabel> {
    pub fn to_vec(&self) -> Vec<&TLabel> {
        match self {
            Uncond(u) => vec![u],
            Cond(cond, fallthrough) => vec![cond, fallthrough],
            Terminal => vec![],
        }
    }

    fn as_ref(&self) -> CfgEdge<&TLabel> {
        match *self {
            Uncond(ref to) => Uncond(to),
            Cond(ref t, ref f) => Cond(t, f),
            Terminal => Terminal,
        }
    }

    fn map<U, F: Fn(&TLabel) -> U>(&self, mapping: F) -> CfgEdge<U> {
        match self {
            Uncond(t) => Uncond(mapping(t)),
            Cond(t, f) => Cond(mapping(t), mapping(f)),
            Terminal => Terminal,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Cfg<TLabel> {
    pub(crate) entry: TLabel,
    pub(crate) out_edges: HashMap<TLabel, CfgEdge<TLabel>>,
}

impl<T: Eq + Hash> Cfg<T> {
    //TODO duplication with next one, cant simplify due to lifetime bounds =(
    pub fn map_label<M, U: Eq + Hash>(&self, mapping: M) -> Cfg<U>
    where
        M: Fn(&T) -> U,
    {
        let out_edges: HashMap<U, CfgEdge<U>> = self
            .out_edges
            .iter()
            .map(|(from, e)| {
                (
                    mapping(from),
                    //TODO is there any simpler way of transforming `&CfgEdge<T>` to `CfgEdge<&T>`?
                    e.map(&mapping),
                )
            })
            .collect();

        Cfg {
            entry: mapping(&self.entry),
            out_edges,
        }
    }

    pub fn to_borrowed(&self) -> Cfg<&T> {
        let out_edges: HashMap<&T, CfgEdge<&T>> = self
            .out_edges
            .iter()
            .map(|(from, e)| (from, e.as_ref()))
            .collect();

        Cfg {
            entry: &self.entry,
            out_edges,
        }
    }
}

impl<TLabel: Eq + Hash + Copy> Cfg<TLabel> {
    pub fn from_edges(
        entry: TLabel,
        edges: &HashMap<TLabel, CfgEdge<TLabel>>,
    ) -> Result<Self, anyhow::Error> {
        let mut out_edges = HashMap::new();
        let mut nodes = HashSet::new();
        for (&from, &edge) in edges.iter() {
            let old_val = out_edges.insert(from, edge);

            ensure!(old_val.is_none(), "repeating source node");

            nodes.insert(from);
            nodes.extend(edge.to_vec());
        }

        for n in nodes {
            out_edges.entry(n).or_insert(Terminal);
        }

        Ok(Self { entry, out_edges })
    }

    pub fn from_vec(
        entry: TLabel,
        edges: &Vec<(TLabel, CfgEdge<TLabel>)>,
    ) -> Result<Self, anyhow::Error> {
        let edges_map: HashMap<TLabel, CfgEdge<TLabel>> = edges.into_iter().copied().collect();
        Self::from_edges(entry, &edges_map)
    }
}

impl<TLabel: CfgLabel> Cfg<TLabel> {
    pub fn nodes(&self) -> HashSet<&TLabel> {
        self.out_edges
            .iter()
            .flat_map(|(from, to)| once(from).chain(to.to_vec()))
            .collect()
    }

    pub fn edge(&self, label: TLabel) -> &CfgEdge<TLabel> {
        // TODO to &TLabel
        self.out_edges
            .get(&label)
            .expect("any node should have outgoing edges")
    }

    pub fn children(&self, label: TLabel) -> HashSet<&TLabel> {
        self.out_edges
            .get(&label)
            .into_iter()
            .flat_map(|edge| edge.to_vec())
            .collect()
    }

    pub fn in_edges(&self) -> HashMap<TLabel, HashSet<TLabel>> {
        let mut in_edges: HashMap<TLabel, HashSet<TLabel>> = HashMap::default();

        for (&from, to_edge) in &self.out_edges {
            for &to in to_edge.to_vec() {
                in_edges.entry(to).or_default().insert(from);
            }
        }

        in_edges
    }
}
