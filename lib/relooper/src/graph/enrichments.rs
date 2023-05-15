use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel, GEdgeColl};
use crate::traversal::graph::bfs::Bfs;
use crate::traversal::graph::dfs::{Dfs, DfsPost, DfsPostReverseInstantiator};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::hash::Hash;
use std::vec::Vec;

use super::cfg::{Graph, GraphMut};

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
            let domination_map = Self::domination_tree(&cfg, &node_ordering);
            DomTree::from_edges(cfg.entry, domination_map)
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
    ) -> HashMap<TLabel, TLabel> /* map points from node id to id of its dominator */ {
        let mut result = HashMap::<TLabel, TLabel>::new();
        let mut bfs = VecDeque::<TLabel>::new();
        let mut visited = HashSet::<TLabel>::new();
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
#[derive(Default)]
pub struct DomTree<T: Hash + Eq> {
    dominates: HashMap<T, HashSet<T>>,
    levels: HashMap<T, usize>,
}

impl<'a, T: Hash + Eq + Clone + 'a> Graph<'a, T, T> for DomTree<T> {
    type EdgeColl = HashSet<T>;

    fn lower_edge(edge: &T) -> &T {
        edge
    }

    fn edges(&'a self) -> &HashMap<T, Self::EdgeColl> {
        &self.dominates
    }
}

impl<T: CfgLabel> DomTree<T> {
    pub fn dom(&self, dominator: &T, dominated: &T) -> bool {
        self.is_reachable(dominator, dominated)
    }

    pub fn stdom(&self, dominator: &T, dominated: &T) -> bool {
        dominator != dominated && self.is_reachable(dominator, dominated)
    }

    pub fn idom(&self, dominator: &T, dominated: &T) -> bool {
        self.children(dominator).contains(dominated)
    }

    pub fn level(&self, node: &T) -> usize {
        *self
            .levels
            .get(node)
            .expect("node should be present in domination tree in order to acquire its level")
    }

    pub fn levels(&self) -> HashMap<T, usize> {
        self.levels.clone()
    }

    //todo remove with new dominance building
    fn from_edges(entry: T, edges: HashMap<T, T>) -> Self {
        let mut dominates: HashMap<T, HashSet<T>> = HashMap::new();

        for (dominated, dominator) in edges {
            dominates.entry(dominator).or_default().insert(dominated);
        }

        let levels = Bfs::start_from((entry, 0), |(n, level)| {
            let next_level = level + 1;
            dominates
                .get(&n)
                .into_iter()
                .flatten()
                .map(move |&x| (x, next_level))
        })
        .collect();

        DomTree { dominates, levels }
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

// impl<T: CfgLabel> EnrichedCfg<T> {
//     fn is_sp_back(&self, from: &T, to: &T) -> bool {
//         todo!()
//     }

//     fn splt_loops(&self, top: &T, set: &HashSet<T>) -> bool {
//         let mut cross = false;
//         for child in self.domination.immediately_dominated_by(top) {
//             if (set.is_empty() || set.contains(child)) && self.splt_loops(child, set) {
//                 cross = true;
//             }
//         }
//         if cross {
//             self.handle_ir_children(top, set)
//         }
//         for predecessor in self.cfg.parents(top) {
//             if self.is_sp_back(predecessor, top) && self.domination.dominates(top, predecessor) {
//                 return true;
//             }
//         }
//         false
//     }

//     fn handle_ir_children(&self, top: &T, set: &HashSet<T>) {
//         todo!()
//     }
// }

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
    fn label(&self) -> &T {
        match self {
            Self::D(x) => x,
            Self::J(jedge) => jedge.label(),
        }
    }
}

#[derive(Debug)]
pub struct DJGraph<T>(HashMap<T, HashSet<DJEdge<T>>>);

impl<'a, T: Eq + Hash + Clone + 'a> Graph<'a, T, DJEdge<T>> for DJGraph<T> {
    type EdgeColl = HashSet<DJEdge<T>>;

    fn lower_edge(edge: &DJEdge<T>) -> &T {
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

    fn remove_node(&mut self, n: &T) {
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

struct DJSpanningTree<T>(HashMap<T, HashSet<T>>);

impl<'a, T: Eq + Hash + 'a> Graph<'a, T, T> for DJSpanningTree<T> {
    type EdgeColl = HashSet<T>;

    fn lower_edge(edge: &T) -> &T {
        edge
    }

    fn edges(&'a self) -> &HashMap<T, Self::EdgeColl> {
        &self.0
    }
}

impl<T> DJSpanningTree<T> {
    fn sp_back(&self, from: &T, to: &T) -> bool {
        false
    }
}

fn dj_spanning<T: CfgLabel>(
    cfg: &Cfg<T>,
    dom_tree: &DomTree<T>,
) -> (DJGraph<T>, DJSpanningTree<T>) {
    let mut dj_graph: HashMap<T, HashSet<DJEdge<T>>> = Default::default();
    for (&from, dom_edge_set) in dom_tree.edges() {
        dj_graph.insert(from, dom_edge_set.iter().map(|&x| DJEdge::D(x)).collect());
    }

    for (f, e) in cfg.edges() {
        let d_edge = dom_tree.edge(f);
        for t in e.iter() {
            if !d_edge.contains(t) {
                let j_edge = if dom_tree.dom(t, f) {
                    JEdge::B(*t)
                } else {
                    JEdge::C(*t)
                };
                dj_graph.entry(*f).or_default().insert(DJEdge::J(j_edge));
                // dj_graph.edge_mut(f).insert(DJEdge::J(j_edge));
            }
        }
    }

    let mut spanning_tree: HashMap<T, HashSet<T>> = Default::default();
    Dfs::start_from(cfg.entry, |x| {
        let children: Vec<T> = dj_graph
            .get(&x)
            .into_iter()
            .flatten()
            .map(|c| c.label())
            .copied()
            .collect();
        for &c in children.iter() {
            spanning_tree.entry(x).or_default().insert(c);
        }
        children
    });

    (DJGraph(dj_graph), DJSpanningTree(spanning_tree))
}
