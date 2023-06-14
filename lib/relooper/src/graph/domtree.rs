use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
    iter::successors,
};

use crate::traversal::graph::dfs::Dfs;

use super::{
    cfg::{Cfg, CfgLabel},
    node_ordering::NodeOrdering,
    Graph,
};

/// Node A dominate node B if you can't reach B without visiting A. For example, entry node dominates all nodes.
/// Each node have set of dominators. If B_set is set of node B dominators, node A will called Immediate Dominator of B
/// if it is in B_set AND NOT dominate any other nodes from B_set.
/// Each node have exactly one immediate dominator. Each node can be immediate dominator for any amount of nodes.
///  
/// Domination tree is a graph with nodes of CFG, but edges only from dominator to dominated nodes.
/// Domination tree uniquely specified by given CFG
///
/// We build domination tree next way:
/// 1) make an array of results (hash_map (dominated -> dominator)) and initialize it with entry node as dominator for every node.
/// 2) Than we iterate in nodes in reverse postorder(?) and make next operation for each node:
///   2.1) remove this node and all its edges from graph, go throw graph with dfs, and find all nodes unreachable without this nodes
///   2.2) update immediate dominator for all unreachable nodes
///
/// Thanks to reverse postorder we will find immediate dominator for all nodes.
///
#[derive(Debug, Default)]
pub struct DomTree<T: Hash + Eq> {
    dominates: HashMap<T, HashSet<T>>,
    levels: HashMap<T, usize>,
}

impl<'a, T: Hash + Eq + Clone + 'a> Graph<'a, T, T> for DomTree<T> {
    type EdgeColl = HashSet<T>;

    fn lower_edge(&'a self, edge: &'a T) -> &'a T {
        edge
    }

    fn edges(&'a self) -> &HashMap<T, Self::EdgeColl> {
        &self.dominates
    }
}

impl<T: CfgLabel> DomTree<T> {
    pub fn is_dom(&self, dominator: &T, dominated: &T) -> bool {
        self.is_reachable(dominator, dominated)
    }

    pub fn dom<'a>(&'a self, dominator: &'a T) -> HashSet<&T> {
        self.reachable(dominator)
    }

    pub fn is_stdom(&self, dominator: &T, dominated: &T) -> bool {
        dominator != dominated && self.is_reachable(dominator, dominated)
    }

    pub fn imm_dominated(&self, n: &T) -> Option<&T> {
        for (from, to) in self.edges() {
            if to.contains(n) {
                return Some(from);
            }
        }
        None
    }

    pub fn is_idom(&self, dominator: &T, dominated: &T) -> bool {
        self.children(dominator).contains(dominated)
    }

    pub fn level(&self, node: &T) -> usize {
        *self
            .levels
            .get(node)
            .expect("node should be present in domination tree in order to acquire its level")
    }

    pub fn levels(&self) -> &HashMap<T, usize> {
        &self.levels
    }

    fn update_dominators(
        cfg: &Cfg<T>,
        node_ordering: &NodeOrdering<T>,
        cur_id: T,
        origin: T,
        result: &mut HashMap<T, T>,
    ) {
        let mut reachable_set = HashSet::<T>::default();
        for &node in node_ordering.sequence() {
            reachable_set.insert(node);
        }

        let reached = Dfs::start_from(&origin, |&n| {
            let mut ch = cfg.children(&n);
            ch.remove(&cur_id);
            ch
        });
        for id in reached {
            reachable_set.remove(id);
        }
        reachable_set.remove(&cur_id);
        for id in reachable_set {
            result.insert(id, cur_id);
        }
    }

    pub fn domination_map(cfg: &Cfg<T>, node_ordering: &NodeOrdering<T>) -> HashMap<T, T> /* map points from node id to id of its dominator */
    {
        let mut result = HashMap::<T, T>::new();
        let mut bfs = VecDeque::<T>::new();
        let mut visited = HashSet::<T>::new();
        for &n in node_ordering.sequence() {
            result.insert(n, cfg.entry);
        }
        bfs.push_back(cfg.entry); // should be next. upd: i dont think so
        visited.insert(cfg.entry);
        loop {
            if bfs.is_empty() {
                break;
            }
            let &cur_id = bfs.front().unwrap();
            visited.insert(cur_id);
            bfs.pop_front().unwrap();
            Self::update_dominators(cfg, node_ordering, cur_id, cfg.entry, &mut result);
            for &id in cfg.children(&cur_id) {
                if !visited.contains(&id) {
                    bfs.push_back(id);
                }
            }
        }
        result
    }

    //todo remove with new dominance building
    fn from_edges(entry: T, domination_map: HashMap<T, T>) -> Self {
        let mut dominates: HashMap<T, HashSet<T>> = HashMap::new();

        for (dominated, dominator) in domination_map {
            dominates.entry(dominator).or_default().insert(dominated);
            dominates.entry(dominated).or_default();
        }

        let levels = successors(
            Some((HashSet::from_iter(Some(entry)), 0_usize)),
            |(nodes, level)| {
                let next_nodes: HashSet<T> = nodes
                    .iter()
                    .flat_map(|n| {
                        let mut all_dom = dominates.children(n);
                        all_dom.remove(n);
                        all_dom
                    })
                    .copied()
                    .collect();
                if next_nodes.is_empty() {
                    None
                } else {
                    Some((next_nodes, level + 1))
                }
            },
        )
        .flat_map(|(nodes_set, level)| nodes_set.into_iter().map(move |n| (n, level)))
        .collect();

        DomTree { dominates, levels }
    }

    pub fn new(cfg: &Cfg<T>) -> Self {
        let node_ordering = NodeOrdering::new(cfg, cfg.entry);
        Self::new_ordering(cfg, &node_ordering)
    }

    pub fn new_ordering(cfg: &Cfg<T>, node_ordering: &NodeOrdering<T>) -> Self {
        let domination_map = Self::domination_map(cfg, node_ordering);
        DomTree::from_edges(cfg.entry, domination_map)
    }
}
