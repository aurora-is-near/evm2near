use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    hash::Hash,
};

use crate::graph::{Graph, GraphMut};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum JEdge<T> {
    B(T),
    C(T),
}
impl<T> JEdge<T> {
    fn label(&self) -> &T {
        match self {
            Self::B(x) => x,
            Self::C(x) => x,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DJEdge<T> {
    D(T),
    J(JEdge<T>),
}

impl<T> DJEdge<T> {
    pub fn label(&self) -> &T {
        match self {
            Self::D(x) => x,
            Self::J(jedge) => jedge.label(),
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

// todo remove if not needed
impl<'a, T: Eq + Hash + Clone + 'a> GraphMut<'a, T, DJEdge<T>> for DJGraph<T> {
    fn edge_mut(&mut self, label: &T) -> &mut Self::EdgeColl {
        self.0.get_mut(label).expect("node should be present")
    }

    fn add_node(&mut self, _n: T) {
        unreachable!()
    }

    fn remove_node<Q: ?Sized>(&mut self, n: &Q)
    where
        T: Borrow<Q>,
        Q: Hash + Eq,
    {
        assert!(self.0.remove(n).is_some());
    }

    fn add_edge(&mut self, from: T, edge: Self::EdgeColl) {
        assert!(self.0.insert(from, edge).is_none());
    }

    fn remove_edge(&mut self, from: T, _edge: &Self::EdgeColl) {
        let _prev = self.0.remove(&from).expect("node should be present");
        // assert_eq!(prev, edge); // todo
    }
}
