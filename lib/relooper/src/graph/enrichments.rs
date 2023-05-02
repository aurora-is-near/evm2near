use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel};
use crate::traversal::graph::bfs::Bfs;
use crate::traversal::graph::dfs::{Dfs, DfsPost, DfsPostReverseInstantiator};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::vec::Vec;

use super::cfg::{GEdge, Graph};

struct Lazy<T, F> {
    init: Option<F>,
    value: Option<T>,
}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    fn new(init: F) -> Self {
        Self {
            init: Some(init),
            value: None,
        }
    }

    fn force(&mut self) -> &T {
        self.value
            .get_or_insert_with(|| (self.init.take().unwrap())())
    }
}

pub struct EnrichedCfg<TLabel: CfgLabel> {
    pub cfg: Cfg<TLabel>,
    pub node_ordering: NodeOrdering<TLabel>,
    pub domination: DomTree<TLabel>,
    pub merge_nodes: HashSet<TLabel>,
    pub loop_nodes: HashSet<TLabel>,
    pub if_nodes: HashSet<TLabel>,
}

impl<TLabel: CfgLabel> EnrichedCfg<TLabel> {
    pub fn new(cfg: Cfg<TLabel>) -> Self {
        let node_ordering = NodeOrdering::new(&cfg, cfg.entry);

        let mut merge_nodes: HashSet<TLabel> = Default::default();
        let mut loop_nodes: HashSet<TLabel> = Default::default();
        let mut if_nodes: HashSet<TLabel> = Default::default();

        let in_edges = cfg.in_edges();

        flame::span_of("marking nodes", || {
            for n in cfg.nodes() {
                flame::span_of("marking node", || {
                    let in_edges_count = in_edges.get(n).map_or(0, |v| {
                        v.iter()
                            .filter(|&from| node_ordering.is_forward(from, n))
                            .count()
                    });
                    if in_edges_count > 1 {
                        merge_nodes.insert(*n);
                    }

                    let mut reachable: Lazy<HashSet<&TLabel>, _> =
                        Lazy::new(|| Bfs::start_from_except(n, |l| cfg.children(l)).collect());
                    for c in cfg.children(n).into_iter() {
                        if node_ordering.is_backward(n, c) && reachable.force().contains(&c) {
                            loop_nodes.insert(*c);
                        }
                    }

                    if let CfgEdge::Cond(_, _) = cfg.edges().get(n).unwrap() {
                        if_nodes.insert(*n);
                    }
                });
            }
        });
        let domination = flame::span_of("building domination", || {
            let domination_map = Self::domination_tree(&cfg, &node_ordering, cfg.entry);
            let domination_vec = Vec::from_iter(domination_map);
            DomTree::from(domination_vec)
        });

        Self {
            cfg,
            node_ordering,
            domination,
            merge_nodes,
            loop_nodes,
            if_nodes,
        }
    }

    pub fn domination_tree(
        cfg: &Cfg<TLabel>,
        node_ordering: &NodeOrdering<TLabel>,
        begin: TLabel,
    ) -> HashMap<TLabel, TLabel> /* map points from node id to id of its dominator */ {
        let mut result = HashMap::<TLabel, TLabel>::new();
        let mut bfs = VecDeque::<TLabel>::new();
        let mut visited = HashSet::<TLabel>::new();
        for &n in node_ordering.sequence() {
            result.insert(n, begin);
        }
        bfs.push_back(begin); // should be next. upd: i dont think so
        visited.insert(begin);
        loop {
            if bfs.is_empty() {
                break;
            }
            let &cur_id = bfs.front().unwrap();
            visited.insert(cur_id);
            bfs.pop_front().unwrap();
            Self::update_dominators(cfg, node_ordering, cur_id, begin, &mut result);
            for &id in cfg.children(&cur_id) {
                if !visited.contains(&id) {
                    bfs.push_back(id);
                }
            }
        }
        result
    }

    fn update_dominators(
        cfg: &Cfg<TLabel>,
        node_ordering: &NodeOrdering<TLabel>,
        cur_id: TLabel,
        origin: TLabel,
        result: &mut HashMap<TLabel, TLabel>,
    ) {
        let mut reachable_set = HashSet::<TLabel>::default();
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
}

#[derive(Default)]
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
pub struct DomTree<TLabel: Hash + Eq + Clone> {
    dominates: HashMap<TLabel, HashSet<TLabel>>,
}

impl<TLabel: Hash + Eq + Clone> Graph for DomTree<TLabel> {
    type Edge = HashSet<TLabel>;
    type Output<U: std::hash::Hash + Eq + Clone> = DomTree<U>;

    fn edges(&self) -> &HashMap<<Self::Edge as super::cfg::GEdge>::Label, Self::Edge> {
        &self.dominates
    }

    fn map_label<M, U: Eq + std::hash::Hash + Clone>(&self, mapping: M) -> Self::Output<U>
    where
        M: Fn(&<Self::Edge as super::cfg::GEdge>::Label) -> U,
        Self: Sized,
    {
        todo!()
    }

    fn add_node(&mut self, n: <Self::Edge as super::cfg::GEdge>::Label) {
        // self.do
        todo!()
    }

    fn remove_node(&mut self, n: &<Self::Edge as super::cfg::GEdge>::Label) {
        todo!()
    }

    fn add_edge(&mut self, from: <Self::Edge as super::cfg::GEdge>::Label, edge: Self::Edge) {
        todo!()
    }

    fn remove_edge(&mut self, from: <Self::Edge as super::cfg::GEdge>::Label, edge: &Self::Edge) {
        todo!()
    }

    fn add_edge_or_promote(
        &mut self,
        from: <Self::Edge as super::cfg::GEdge>::Label,
        to: <Self::Edge as super::cfg::GEdge>::Label,
    ) {
        todo!()
    }

    fn edge(&self, label: &<Self::Edge as super::cfg::GEdge>::Label) -> &Self::Edge {
        todo!()
    }

    fn edge_mut(&mut self, label: &<Self::Edge as super::cfg::GEdge>::Label) -> &mut Self::Edge {
        todo!()
    }

    fn nodes(&self) -> HashSet<&<Self::Edge as super::cfg::GEdge>::Label> {
        self.edges().keys().collect()
    }
}

impl<TLabel: CfgLabel> From<Vec<(TLabel, TLabel)>> for DomTree<TLabel> {
    fn from(edges: Vec<(TLabel, TLabel)>) -> Self {
        let dominated: HashMap<TLabel, TLabel> = HashMap::from_iter(edges.iter().copied());
        let mut dominates: HashMap<TLabel, HashSet<TLabel>> = HashMap::new();

        for (dominated, dominator) in edges {
            dominates.entry(dominator).or_default().insert(dominated);
        }

        DomTree {
            dominates,
            // dominated,
        }
    }
}

impl<TLabel: CfgLabel> DomTree<TLabel> {
    pub(crate) fn immediately_dominated_by(&self, label: &TLabel) -> HashSet<&TLabel> {
        self.dominates.get(label).into_iter().flatten().collect()
    }

    pub fn dominates(&self, dominator: &TLabel, dominated: &TLabel) -> bool {
        let mut dom_iter = Bfs::start_from(dominator, |x| self.immediately_dominated_by(x));
        dom_iter.any(|x| x == dominated)
    }
}

pub struct NodeOrdering<TLabel: CfgLabel> {
    pub(crate) idx: HashMap<TLabel, usize>,
    vec: Vec<TLabel>,
}

impl<TLabel: CfgLabel> NodeOrdering<TLabel> {
    pub fn new(cfg: &Cfg<TLabel>, entry: TLabel) -> Self {
        let vec: Vec<TLabel> = DfsPost::<_, _, HashSet<_>>::reverse(&entry, |x| cfg.children(x))
            .into_iter()
            .copied()
            .collect();
        let idx: HashMap<TLabel, usize> = vec.iter().enumerate().map(|(i, &n)| (n, i)).collect();
        Self { vec, idx }
    }

    pub fn is_backward(&self, from: &TLabel, to: &TLabel) -> bool {
        self.idx
            .get(from)
            .zip(self.idx.get(to))
            .map(|(&f, &t)| f > t)
            .unwrap()
    }

    pub fn is_forward(&self, from: &TLabel, to: &TLabel) -> bool {
        !self.is_backward(from, to)
    }

    pub fn sequence(&self) -> &Vec<TLabel> {
        &self.vec
    }
}

impl<T: CfgLabel> EnrichedCfg<T> {
    // fn is_sp_back(&self, from: &T, to: &T) -> bool {
    //     todo!()
    // }

    // fn splt_loops(&self, top: &T, set: &HashSet<T>) -> bool {
    //     let mut cross = false;
    //     for child in self.domination.immediately_dominated_by(top) {
    //         if (set.is_empty() || set.contains(child)) && self.splt_loops(child, set) {
    //             cross = true;
    //         }
    //     }
    //     if cross {
    //         self.handle_ir_children(top, set)
    //     }
    //     for predecessor in self.cfg.parents(top) {
    //         if self.is_sp_back(predecessor, top) && self.domination.dominates(top, predecessor) {
    //             return true;
    //         }
    //     }
    //     false
    // }

    // fn handle_ir_children(&self, top: &T, set: &HashSet<T>) {
    //     todo!()
    // }
}
