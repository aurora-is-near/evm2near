use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
use anyhow::ensure;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::once;

mod cfg_mut;
mod cfg_parsing;

pub trait CfgLabel: Copy + Hash + Eq + Ord + Sized {}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum CfgEdge<TLabel> {
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

/// This struct is used as `Cfg` assembly description.
/// The main purpose of its existence is to provide `to_borrowed` function
/// that allows us to parse lines into that structure (parametrized by `T`, for example)
/// and re-parameterize that description with `&T` which can be used as CfgLabel afterwards
/// (while `T` can not impl `Copy` or other traits). The main motivation is `String` labels so far.
pub struct CfgDescr<TLabel> {
    pub(crate) entry: TLabel,
    pub(crate) edges: HashMap<TLabel, CfgEdge<TLabel>>,
}

impl<T: Eq + Hash> CfgDescr<T> {
    //TODO duplication with next one, cant simplify due to lifetime bounds =(
    pub fn map_label<M, U: Eq + Hash>(&self, mapping: M) -> CfgDescr<U>
    where
        M: Fn(&T) -> U,
    {
        let edges: HashMap<U, CfgEdge<U>> = self
            .edges
            .iter()
            .map(|(from, e)| {
                (
                    mapping(from),
                    //TODO is there any simpler way of transforming `&CfgEdge<T>` to `CfgEdge<&T>`?
                    match e {
                        Uncond(t) => Uncond(mapping(t)),
                        Cond(t, f) => Cond(mapping(t), mapping(f)),
                        Terminal => Terminal,
                    },
                )
            })
            .collect();

        CfgDescr {
            entry: mapping(&self.entry),
            edges,
        }
    }

    pub fn to_borrowed(&self) -> CfgDescr<&T> {
        let edges: HashMap<&T, CfgEdge<&T>> = self
            .edges
            .iter()
            .map(|(from, e)| {
                (
                    from,
                    //TODO is there any simpler way of transforming `&CfgEdge<T>` to `CfgEdge<&T>`?
                    match e {
                        Uncond(t) => Uncond(t),
                        Cond(t, f) => Cond(t, f),
                        Terminal => Terminal,
                    },
                )
            })
            .collect();

        CfgDescr {
            entry: &self.entry,
            edges,
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
    pub fn from_descr(descr: &CfgDescr<TLabel>) -> Result<Self, anyhow::Error> {
        Self::from_edges(descr.entry, &descr.edges)
    }

    pub fn descr(&self) -> CfgDescr<TLabel> {
        CfgDescr {
            entry: self.entry,
            edges: self.out_edges.clone(),
        }
    }

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

    pub fn from_vec(
        entry: TLabel,
        edges: &Vec<(TLabel, CfgEdge<TLabel>)>,
    ) -> Result<Self, anyhow::Error> {
        let edges_map: HashMap<TLabel, CfgEdge<TLabel>> = edges.into_iter().copied().collect();
        Self::from_edges(entry, &edges_map)
    }

    pub fn nodes(&self) -> HashSet<TLabel> {
        self.out_edges
            .iter()
            .flat_map(|(&from, &to)| once(from).chain(to.to_vec()))
            .collect()
    }

    pub fn edge(&self, label: TLabel) -> &CfgEdge<TLabel> {
        // TODO to &TLabel
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
