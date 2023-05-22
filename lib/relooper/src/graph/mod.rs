use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::traversal::graph::bfs::Bfs;
use crate::traversal::graph::dfs::{PrePostOrder, VisitAction};

pub mod cfg;
pub mod dot_debug;
pub mod enrichments;
pub mod relooper;
pub mod supergraph;

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
    type EdgeColl: GEdgeColl<Edge = TE>; // todo try changing to IntoIter for a ref? cant do that right now due to 'CfgEdgeIter' reference lifetime requirement

    fn lower_edge(&'a self, edge: &'a TE) -> &'a T; //todo rename

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
            .flat_map(|edge_coll| edge_coll.iter().map(|edge| self.lower_edge(edge)))
            .collect()
    }

    fn parents(&'a self, label: &T) -> HashSet<&'a T> {
        self.edges()
            .iter()
            .filter_map(|(from, edge_coll)| {
                edge_coll.iter().find_map(|x| {
                    if self.lower_edge(x) == label {
                        Some(from)
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    fn reachable(&'a self, node: &'a T) -> HashSet<&T> {
        Bfs::start_from(node, |x| self.children(x)).collect()
    }

    fn is_reachable(&'a self, ancestor: &T, descendant: &T) -> bool {
        let mut descendants = Bfs::start_from(ancestor, |x| self.children(x));
        descendants.any(|x| x == descendant)
    }

    // todo rename to 'transpose'?
    fn in_edges(&'a self) -> HashMap<&'a T, HashSet<&'a T>> {
        let mut in_edges: HashMap<&T, HashSet<&T>> = HashMap::default();

        for (from, edge_coll) in self.edges() {
            for to in edge_coll.iter() {
                in_edges
                    .entry(self.lower_edge(to))
                    .or_default()
                    .insert(from);
            }
        }

        in_edges
    }
}

pub trait GraphCopy<'a, T: Eq + Hash + Copy + 'a>: Graph<'a, T, T> {
    fn kosaraju_scc(&'a self, start: &T) -> Vec<HashSet<T>> {
        let mut components: Vec<HashSet<T>> = Default::default();
        let mut visited: HashSet<T> = Default::default();

        let mut transposed = self.in_edges();

        let mut order: Vec<_> = PrePostOrder::start_from(start, |x| self.children(x))
            .filter_map(|visit_action| match visit_action {
                VisitAction::Enter(_) => None,
                VisitAction::Leave(x) => Some(x),
            })
            .collect();
        order.reverse();

        for x in order {
            if visited.contains(x) {
                continue;
            }
            let reachable: HashSet<T> = transposed.reachable(&x).into_iter().map(|&&x| x).collect();
            for r in reachable.clone() {
                visited.insert(r);
                transposed.remove_node(&r);
            }
            components.push(reachable);
        }

        components
    }
}

impl<'a, T: Eq + Hash + Copy + 'a, TG: Graph<'a, T, T>> GraphCopy<'a, T> for TG {}

pub trait GEdge {
    type Inside;
    fn lower(&self) -> &Self::Inside;
}

pub trait GraphMut<'a, T: Eq + Hash + 'a, TE: 'a>: Graph<'a, T, TE> {
    fn edge_mut(&mut self, label: &T) -> &mut Self::EdgeColl;

    fn add_node(&mut self, n: T);

    fn remove_node<Q: ?Sized>(&mut self, n: &Q)
    where
        T: Borrow<Q>,
        Q: Hash + Eq;

    fn add_edge(&mut self, from: T, edge: Self::EdgeColl);

    fn remove_edge(&mut self, from: T, edge: &Self::EdgeColl);
}

//todo cant do that until we redo GEdgeColl to IntoIterator
// impl<'a, T: Hash + Eq + 'a, TI: IntoIterator<Item = T>> Graph<'a, T, T> for HashMap<T, TI> {
//     type EdgeColl = TI;

//     fn lower_edge(edge: &T) -> &T {
//         edge
//     }

//     fn edges(&'a self) -> &HashMap<T, Self::EdgeColl> {
//         &self
//     }
// }

impl<'a, T: Eq + Hash + 'a> Graph<'a, T, T> for HashMap<T, HashSet<T>> {
    type EdgeColl = HashSet<T>;

    fn lower_edge(&'a self, edge: &'a T) -> &'a T {
        edge
    }

    fn edges(&'a self) -> &HashMap<T, Self::EdgeColl> {
        self
    }
}

impl<'a, T: Eq + Hash + 'a> GraphMut<'a, T, T> for HashMap<T, HashSet<T>> {
    fn edge_mut(&mut self, label: &T) -> &mut Self::EdgeColl {
        self.get_mut(label)
            .expect("node should be present in graph")
    }

    fn add_node(&mut self, _n: T) {}

    fn remove_node<Q: ?Sized>(&mut self, n: &Q)
    where
        T: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.remove(n).expect("node should be present in graph");
        for to_edges in self.values_mut() {
            to_edges.remove(n);
        }
    }

    fn add_edge(&mut self, from: T, edge: Self::EdgeColl) {
        assert!(self.insert(from, edge).is_none());
    }

    fn remove_edge(&mut self, from: T, edge: &Self::EdgeColl) {
        assert!(&self.remove(&from).expect("node should be present in graph") == edge)
    }
}
