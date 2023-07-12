use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::graph::Graph;
use crate::traversal::graph::dfs::{Dfs, PrePostOrder, VisitAction};

use super::dj_graph::DJGraph;

#[derive(Debug)]
pub struct DJSpanningTree<T>(pub HashMap<T, HashSet<T>>);

impl<'a, T: Eq + Hash + 'a> Graph<'a, T, T> for DJSpanningTree<T> {
    type EdgeColl = HashSet<T>;

    fn lower_edge(&'a self, edge: &'a T) -> &'a T {
        edge
    }

    fn edges(&'a self) -> &HashMap<T, Self::EdgeColl> {
        &self.0
    }
}

impl<'a, T: Eq + Hash + Copy + std::fmt::Debug + 'a> DJSpanningTree<T> {
    pub fn new(entry: T, dj_graph: &DJGraph<T>) -> Self {
        let mut spanning_tree: HashMap<T, HashSet<T>> = Default::default();
        Dfs::start_from(entry, |x| {
            let children: HashSet<T> = dj_graph
                .edge(&x)
                .iter()
                .map(|c| c.label())
                .copied()
                .collect();
            spanning_tree.insert(x, children.clone());
            children
        })
        .count(); // only for side effect computation

        DJSpanningTree(spanning_tree)
    }

    fn is_sp_back(&self, from: &T, to: &T) -> bool {
        from == to || self.is_reachable(to, from)
    }

    // fn is_sp_tree(&self, from: &T, to: &T) -> bool {
    //     self.children(from).contains(to)
    // }

    // fn is_sp_forward(&self, from: &T, to: &T) -> bool {
    //     !self.children(from).contains(to) && self.is_reachable(from, to)
    // }

    // fn is_sp_cross(&self, from: &T, to: &T) -> bool {
    //     !self.is_reachable(from, to) && !self.is_reachable(to, from)
    // }

    pub fn sp_back(&'a self, entry: &'a T) -> HashSet<(&'a T, &'a T)> {
        let mut set: HashSet<(&T, &T)> = Default::default();

        let pre_post_order =
            PrePostOrder::start_from(entry, |x| self.children(x)).collect::<Vec<_>>();

        println!("prepost -----------\n{:?}", pre_post_order);

        let mut path: HashSet<&T> = Default::default();

        for traverse_action in pre_post_order {
            match traverse_action {
                VisitAction::Enter(e) => {
                    path.insert(e);

                    let ch = self.children(e);
                    println!("{:?} ch: {:?}", e, ch);

                    let sp_iter = ch.into_iter().filter(|c| path.contains(c)).map(|c| (e, c));
                    for sp_back in sp_iter {
                        set.insert(sp_back);
                    }
                }
                VisitAction::Leave(l) => {
                    path.remove(&l);
                }
            }
        }

        assert!(set.iter().all(|(f, t)| self.is_sp_back(f, t)));

        set
    }
}
