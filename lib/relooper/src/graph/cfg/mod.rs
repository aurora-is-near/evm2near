use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
use crate::traversal::graph::bfs::Bfs;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

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

    pub(crate) fn map<'a, U, F: Fn(&'a TLabel) -> U>(&'a self, mapping: F) -> CfgEdge<U> {
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
    out_edges: HashMap<TLabel, CfgEdge<TLabel>>,
}

impl<T> Cfg<T> {
    pub fn edges(&self) -> &HashMap<T, CfgEdge<T>> {
        &self.out_edges
    }
}

impl<T: Eq + Hash + Clone> Cfg<T> {
    pub fn new(entry: T) -> Cfg<T> {
        Self {
            entry,
            out_edges: Default::default(),
        }
    }

    pub fn map_label<'a, M, U: Eq + Hash>(&'a self, mapping: M) -> Cfg<U>
    where
        M: Fn(&'a T) -> U,
    {
        let out_edges: HashMap<U, CfgEdge<U>> = self
            .out_edges
            .iter()
            .map(|(from, e)| (mapping(from), e.map(&mapping)))
            .collect();

        Cfg {
            entry: mapping(&self.entry),
            out_edges,
        }
    }

    pub fn to_borrowed(&self) -> Cfg<&T> {
        self.map_label(|l| l)
    }

    pub fn nodes(&self) -> HashSet<&T> {
        self.out_edges.keys().collect()
    }

    pub fn children(&self, label: &T) -> HashSet<&T> {
        self.out_edges
            .get(label)
            .into_iter()
            .flat_map(|edge| edge.to_vec())
            .collect()
    }

    pub fn add_edge(&mut self, from: T, edge: CfgEdge<T>) {
        let nodes = self.nodes();
        // todo unncecessary collect
        for n in edge
            .to_vec()
            .into_iter()
            .filter(|n| !nodes.contains(n))
            .collect::<Vec<_>>()
        {
            // todo clone
            self.add_node(n.clone());
        }

        let prev_edge = self.out_edges.insert(from, edge);
        match prev_edge {
            None | Some(Terminal) => {}
            _ => panic!("adding edge over already present one"),
        }
    }

    pub fn add_node(&mut self, n: T) {
        self.out_edges.insert(n, Terminal);
    }

    pub fn remove_edge(&mut self, from: T, edge: CfgEdge<T>) {
        let removed_edge = self.out_edges.remove(&from);
        assert!(removed_edge == Some(edge));
    }

    pub fn add_edge_or_promote(&mut self, from: T, to: T) {
        match self.out_edges.remove(&from) {
            None | Some(Terminal) => self.out_edges.insert(from, Uncond(to)),
            Some(Uncond(uncond)) => self.out_edges.insert(from, Cond(to, uncond)),
            _ => panic!("edge (should be absent) or (shouldn't be `Cond`)"),
        };
    }

    pub fn edge(&self, label: &T) -> &CfgEdge<T> {
        self.out_edges
            .get(label)
            .expect("any node should have outgoing edges")
    }
}

impl<TLabel: Eq + Hash + Copy> Cfg<TLabel> {
    pub fn from_edges(entry: TLabel, edges: &HashMap<TLabel, CfgEdge<TLabel>>) -> Self {
        let mut cfg = Cfg::new(entry);
        for (&from, &edge) in edges.iter() {
            cfg.add_edge(from, edge);
        }

        cfg
    }
}

impl<TLabel: CfgLabel> Cfg<TLabel> {
    pub fn in_edges(&self) -> HashMap<TLabel, HashSet<TLabel>> {
        let mut in_edges: HashMap<TLabel, HashSet<TLabel>> = HashMap::default();

        for (&from, to_edge) in &self.out_edges {
            for &to in to_edge.to_vec() {
                in_edges.entry(to).or_default().insert(from);
            }
        }

        in_edges
    }

    fn reachable_nodes(&self) -> HashSet<&TLabel> {
        Bfs::start_from(&self.entry, |label| self.children(label)).collect()
    }

    pub fn strip_unreachable(&mut self) {
        let nodes: HashSet<TLabel> = self.nodes().into_iter().copied().collect(); // TODO get rid of copies
        let reachable: HashSet<TLabel> = self.reachable_nodes().into_iter().copied().collect();
        for unreachable in nodes.difference(&reachable) {
            self.out_edges.remove(unreachable);
        }
    }
}
