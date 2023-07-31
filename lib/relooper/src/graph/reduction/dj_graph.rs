use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use crate::graph::{
    cfg::{Cfg, CfgLabel},
    domtree::DomTree,
    GEdgeColl, Graph,
};

/// 'D' edge is dominator tree edge (regardless of whether this edge is present in graph itself)
/// 'J' edges are all other edges from graph itself
/// 'JB' edge is graph "back edge" (where destination node dominates source node)
/// 'JC' edge is graph "cross edge" (all other edges)
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DJEdge<T> {
    D(T),
    JC(T),
    JB(T),
}

impl<T> DJEdge<T> {
    pub fn label(&self) -> &T {
        match self {
            Self::D(x) => x,
            Self::JC(x) => x,
            Self::JB(x) => x,
        }
    }
}

#[derive(Debug)]
pub struct DJGraph<T>(pub HashMap<T, HashSet<DJEdge<T>>>);

impl<'a, T: Eq + Hash + Clone + 'a> Graph<'a, T, DJEdge<T>> for DJGraph<T> {
    type EdgeColl = HashSet<DJEdge<T>>;

    fn lower_edge(&'a self, edge: &'a DJEdge<T>) -> &'a T {
        edge.label()
    }

    fn edges(&'a self) -> &HashMap<T, Self::EdgeColl> {
        &self.0
    }
}

impl<T: CfgLabel> DJGraph<T> {
    pub fn new(cfg: &Cfg<T>, dom_tree: &DomTree<T>) -> Self {
        //todo to .map_label
        let mut dj_graph: HashMap<T, HashSet<DJEdge<T>>> = Default::default();
        for (&from, dom_edge_set) in dom_tree.edges() {
            dj_graph.insert(from, dom_edge_set.iter().map(|&x| DJEdge::D(x)).collect());
        }

        for (f, e) in cfg.edges() {
            let d_edge = dom_tree.edge(f);
            for t in e.iter() {
                if !d_edge.contains(t) {
                    let j_edge = if dom_tree.is_dom(t, f) {
                        DJEdge::JB(*t)
                    } else {
                        DJEdge::JC(*t)
                    };
                    dj_graph.entry(*f).or_default().insert(j_edge);
                }
            }
        }

        DJGraph(dj_graph)
    }
}
