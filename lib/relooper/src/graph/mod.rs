use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::traversal::graph::bfs::Bfs;

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

// pub trait GraphSameTypes<T: Eq + Hash> {}

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
}
