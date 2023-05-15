use crate::graph::cfg::CfgEdge::{Cond, Switch, Terminal, Uncond};
use crate::traversal::graph::bfs::Bfs;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

mod cfg_parsing;

pub trait CfgLabel: Copy + Hash + Eq + Ord + Debug {}

impl<T: Copy + Hash + Eq + Ord + Debug> CfgLabel for T {}

pub trait GEdgeColl {
    type Edge: Eq + Hash;
    type Iter<'a>: Iterator<Item = &'a Self::Edge>
    where
        Self: 'a;

    #[allow(clippy::needless_lifetimes)] // lint is wrong, probably due to generic_associated_types or associated_type_bounds features
    fn iter<'a>(&'a self) -> Self::Iter<'a>;
}

pub trait GEdgeCollMappable: GEdgeColl {
    type Output<U: Hash + Eq>: GEdgeColl<Edge = U>;
    fn map<U: Hash + Eq, F: Fn(&Self::Edge) -> U>(&self, mapping: F) -> Self::Output<U>;
}

impl<T: Eq + Hash> GEdgeColl for HashSet<T> {
    type Edge = T;
    type Iter<'a> = std::collections::hash_set::Iter<'a, T> where T: 'a;

    #[allow(clippy::needless_lifetimes)]
    fn iter<'a>(&'a self) -> Self::Iter<'a> {
        self.iter()
    }
}

pub trait Graph<'a, T: Eq + Hash + 'a, TE: 'a> {
    type EdgeColl: GEdgeColl<Edge = TE>; // todo try changing to IntoIter for a ref?

    fn lower_edge(edge: &TE) -> &T; //todo rename

    fn edges(&'a self) -> &HashMap<T, Self::EdgeColl>; // change return to Cow?
    fn nodes(&'a self) -> HashSet<&'a T> {
        self.edges().keys().collect()
    }

    fn edge(&'a self, label: &T) -> &'a Self::EdgeColl {
        self.edges()
            .get(label)
            .expect("given node should be present")
    }

    fn children(&'a self, label: &T) -> HashSet<&'a T> {
        self.edges()
            .get(label)
            .into_iter()
            .flat_map(|edge_coll| edge_coll.iter().map(|edge| Self::lower_edge(edge)))
            .collect()
    }

    fn parents(&'a self, label: &T) -> HashSet<&'a T> {
        self.edges()
            .iter()
            .filter_map(|(from, edge_coll)| {
                edge_coll.iter().find_map(|x| {
                    if Self::lower_edge(x) == label {
                        Some(from)
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    fn in_edges(&'a self) -> HashMap<&'a T, HashSet<&'a T>> {
        let mut in_edges: HashMap<&T, HashSet<&T>> = HashMap::default();

        let x = self.edges();
        for (from, to_edge) in x {
            for to in to_edge.iter() {
                in_edges
                    .entry(Self::lower_edge(to))
                    .or_default()
                    .insert(from);
            }
        }

        in_edges
    }

    fn is_reachable(&'a self, ancestor: &T, descendant: &T) -> bool {
        let mut descendants = Bfs::start_from(ancestor, |x| self.children(x));
        descendants.any(|x| x == descendant)
    }
}

pub trait GraphSameTypes<T: Eq + Hash> {}

// impl<T: Hash + Eq, GST: GraphSameTypes<T>> Graph<T, T> for GST {
//     fn lower_edge(edge: &T) -> &T {
//         edge
//     }

//     type EdgeColl;

//     fn edges(&self) -> &HashMap<T, Self::EdgeColl> {
//         todo!()
//     }
// }

pub trait GEdge {
    type Inside;
    fn lower(&self) -> &Self::Inside;
}

// pub trait GraphMappable<T: Eq + Hash, TE: GEdge<Inside = T>>: Graph<T, TE> {
//     type Output<U: Hash + Eq + Clone, UE: GEdge<Inside = U>>: Graph<
//         U,
//         UE,
//         EdgeColl: GEdgeColl<Label = UE>,
//     >;

//     // maybe it is better to implement IterMut instead
//     fn map_label<M, U: Eq + Hash + Clone, UE: GEdge<Inside = U>>(
//         &self,
//         mapping: M,
//     ) -> Self::Output<U, UE>
//     where
//         M: Fn(&<Self::EdgeColl as GEdgeColl>::Label) -> U,
//         Self: Sized;
// }

pub trait GraphMut<'a, T: Eq + Hash + 'a, TE: 'a>: Graph<'a, T, TE> {
    fn edge_mut(&mut self, label: &T) -> &mut Self::EdgeColl;

    fn add_node(&mut self, n: T);

    fn remove_node(&mut self, n: &T);

    fn add_edge(&mut self, from: T, edge: Self::EdgeColl);

    fn remove_edge(&mut self, from: T, edge: &Self::EdgeColl);

    fn add_edge_or_promote(&mut self, from: T, to: TE);
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum CfgEdge<TLabel> {
    Uncond(TLabel),
    Cond(TLabel, TLabel),
    Switch(Vec<(usize, TLabel)>),
    Terminal,
}

impl<TLabel> CfgEdge<TLabel> {
    pub(crate) fn apply<F: Fn(&TLabel) -> TLabel>(&mut self, mapping: F) {
        match self {
            Self::Uncond(t) => {
                *self = Self::Uncond(mapping(t));
            }
            Self::Cond(t, f) => {
                *self = Self::Cond(mapping(t), mapping(f));
            }
            Self::Switch(v) => {
                for (_, x) in v {
                    *x = mapping(x)
                }
            }
            Self::Terminal => {}
        }
    }
}

/// A struct which enables iterating over the nodes that make up a `CfgEdge`.
/// Internally it stores the data as a 2-array as opposed to a `Vec` to avoid heap allocation.
/// For the case of the `CfgEdge::Switch` variant an iterator over the contained `Vec` is
/// used directly.
#[derive(Debug, Clone)]
pub struct CfgEdgeIter<'a, T> {
    fixed: [Option<&'a T>; 2],
    allocated: std::slice::Iter<'a, (usize, T)>,
    index: usize,
}

impl<'a, T> Iterator for CfgEdgeIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < 2 {
            let item = self.fixed[self.index].take();
            self.index += 1;
            return item;
        }

        self.allocated.next().map(|(_, x)| x)
    }
}

impl<T: Eq + Hash> GEdgeCollMappable for HashSet<T> {
    type Output<U: Hash + Eq> = HashSet<U>;
    fn map<U: Hash + Eq, F: Fn(&Self::Edge) -> U>(&self, mapping: F) -> Self::Output<U> {
        self.iter().map(mapping).collect()
    }
}

impl<T: Eq + Hash> GEdgeColl for CfgEdge<T> {
    type Edge = T;
    type Iter<'a> = CfgEdgeIter<'a, Self::Edge> where Self::Edge: 'a;

    #[allow(clippy::needless_lifetimes)]
    fn iter<'a>(&'a self) -> Self::Iter<'a> {
        match self {
            Uncond(u) => CfgEdgeIter {
                fixed: [Some(u), None],
                allocated: [].iter(),
                index: 0,
            },
            Cond(cond, fallthrough) => CfgEdgeIter {
                fixed: [Some(cond), Some(fallthrough)],
                allocated: [].iter(),
                index: 0,
            },
            Switch(v) => CfgEdgeIter {
                fixed: [None, None],
                allocated: v.iter(),
                index: 2,
            },
            Terminal => CfgEdgeIter {
                fixed: [None, None],
                allocated: [].iter(),
                index: 0,
            },
        }
    }
}

impl<T: Eq + Hash> GEdgeCollMappable for CfgEdge<T> {
    type Output<U: Hash + Eq> = CfgEdge<U>;
    fn map<U: Hash + Eq, F: Fn(&Self::Edge) -> U>(&self, mapping: F) -> Self::Output<U> {
        match self {
            Self::Uncond(t) => Uncond(mapping(t)),
            Self::Cond(t, f) => Cond(mapping(t), mapping(f)),
            Self::Switch(v) => Switch(v.iter().map(|(u, x)| (*u, mapping(x))).collect()),
            Self::Terminal => Terminal,
        }
    }
}

// impl<T: Hash + Eq + Clone + GEdge<Inside = T>> Graph<T, T> for Cfg<T> {
//     type EdgeColl = CfgEdge<T>;

//     fn edges(&self) -> &HashMap<<Self::EdgeColl as GEdgeColl>::Label, Self::EdgeColl> {
//         &self.out_edges
//     }

//     fn edge(&self, label: &<Self::EdgeColl as GEdgeColl>::Label) -> &Self::EdgeColl {
//         self.out_edges
//             .get(label)
//             .expect("any node should have outgoing edges")
//     }
// }

impl<'a, T: Hash + Eq + Clone + 'a> Graph<'a, T, T> for Cfg<T> {
    type EdgeColl = CfgEdge<T>;

    fn lower_edge(edge: &T) -> &T {
        edge
    }

    fn edges(&'a self) -> &HashMap<T, Self::EdgeColl> {
        &self.out_edges
    }
}

// impl<T: Hash + Eq + Clone + GEdge<Inside = T>> GraphMappable<T, T> for Cfg<T> {
//     type Output<U: Hash + Eq + Clone, UE: GEdge<Inside = U>> = Cfg<U>;

//     fn map_label<M, U: Eq + Hash + Clone, UE: GEdge<Inside = U>>(
//         &self,
//         mapping: M,
//     ) -> Self::Output<U, UE>
//     where
//         M: Fn(&<Self::EdgeColl as GEdgeColl>::Label) -> U,
//         Self: Sized,
//     {
//         let out_edges = self
//             .edges()
//             .iter()
//             .map(|(l, edge)| (mapping(l), edge.map(&mapping)))
//             .collect();
//         Cfg {
//             entry: mapping(&self.entry),
//             out_edges,
//         }
//     }
// }

impl<'a, T: Hash + Eq + Clone + 'a> GraphMut<'a, T, T> for Cfg<T> {
    fn edge_mut(&mut self, label: &T) -> &mut Self::EdgeColl {
        self.out_edges
            .get_mut(label)
            .expect("any node should have outgoing edges")
    }

    fn add_node(&mut self, n: T) {
        let prev_edge = self.out_edges.insert(n, Terminal);
        Self::check_previous_edge(prev_edge);
    }

    fn remove_node(&mut self, n: &T) {
        self.out_edges
            .remove(n)
            .expect("cannot delete non-present node");
    }

    fn add_edge(&mut self, from: T, edge: Self::EdgeColl) {
        let out_edges = &mut self.out_edges;
        for n in edge.iter() {
            if !out_edges.contains_key(n) {
                // The clone here is required because we use `edge` again in the insert below
                out_edges.insert(n.clone(), Terminal);
            }
        }

        let prev_edge = out_edges.insert(from, edge);
        Self::check_previous_edge(prev_edge);
    }

    fn remove_edge(&mut self, from: T, edge: &Self::EdgeColl) {
        let removed_edge = self.out_edges.remove(&from);
        assert!(removed_edge.as_ref() == Some(edge));
    }

    fn add_edge_or_promote(&mut self, from: T, to: T) {
        match self.out_edges.remove(&from) {
            None | Some(Terminal) => self.out_edges.insert(from, Uncond(to)),
            Some(Uncond(uncond)) => self.out_edges.insert(from, Cond(to, uncond)),
            _ => panic!("edge (should be absent) or (shouldn't be `Cond`)"),
        };
    }
}

#[derive(Clone, Debug)]
pub struct Cfg<TLabel> {
    pub(crate) entry: TLabel,
    out_edges: HashMap<TLabel, CfgEdge<TLabel>>,
}

impl<T: Eq + Hash + Clone> Cfg<T> {
    pub fn new(entry: T) -> Cfg<T> {
        Self {
            entry,
            out_edges: Default::default(),
        }
    }

    fn check_previous_edge(edge: Option<CfgEdge<T>>) {
        match edge {
            None | Some(Terminal) => {}
            _ => panic!("adding edge over already present one"),
        }
    }

    pub fn map_label<M: Fn(&T) -> U, U: Eq + std::hash::Hash + Clone>(&self, mapping: M) -> Cfg<U> {
        let out_edges = self
            .out_edges
            .iter()
            .map(|(f, edges)| (mapping(f), edges.map(&mapping)))
            .collect();
        Cfg {
            entry: mapping(&self.entry),
            out_edges,
        }
    }
}

impl<TLabel: Eq + Hash + Copy> Cfg<TLabel> {
    pub fn from_edges(entry: TLabel, edges: HashMap<TLabel, CfgEdge<TLabel>>) -> Self {
        let mut cfg = Cfg::new(entry);
        for (from, edge) in edges.into_iter() {
            cfg.add_edge(from, edge);
        }

        cfg
    }
}

impl<TLabel: CfgLabel> Cfg<TLabel> {
    pub fn strip_unreachable(&mut self) {
        let reachable_from_start: HashSet<&TLabel> =
            Bfs::start_from(&self.entry, |label| self.children(label)).collect();
        let unreachable_nodes: HashSet<TLabel> = self
            .nodes()
            .difference(&reachable_from_start)
            .into_iter()
            .map(|n| **n)
            .collect();
        for unreachable in unreachable_nodes {
            self.remove_node(&unreachable);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfg_edge_iter() {
        test_cfg_edge_iter_inner(Vec::new());
        test_cfg_edge_iter_inner(vec![7]);
        test_cfg_edge_iter_inner(vec![123, 456]);
        test_cfg_edge_iter_inner(vec![123, 456, 789]);
    }

    fn test_cfg_edge_iter_inner(input: Vec<usize>) {
        let edge = cfg_edge_from_slice(&input);

        let reconstructed: Vec<usize> = edge.iter().copied().collect();
        assert_eq!(input, reconstructed);
    }

    fn cfg_edge_from_slice<T: Copy>(values: &[T]) -> CfgEdge<T> {
        match values {
            [] => CfgEdge::Terminal,
            [x] => CfgEdge::Uncond(*x),
            [x, y] => CfgEdge::Cond(*x, *y),
            longer => CfgEdge::Switch(longer.iter().map(|x| (0, *x)).collect()),
        }
    }
}
