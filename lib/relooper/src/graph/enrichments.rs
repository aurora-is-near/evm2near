use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel};
use crate::traversal::graph::bfs::Bfs;
use crate::traversal::graph::dfs::{
    Dfs, DfsPost, DfsPostReverseInstantiator, PrePostOrder, VisitAction,
};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::hash::Hash;
use std::vec::Vec;

use super::supergraph::SLabel;
use super::{GEdgeColl, GEdgeCollMappable, Graph, GraphMut};

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
            DomTree::new_ordering(&cfg, &node_ordering)
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

    pub fn dominates<'a>(&'a self, dominator: &'a T) -> HashSet<&T> {
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

    pub fn new(cfg: &Cfg<T>) -> Self {
        let node_ordering = NodeOrdering::new(cfg, cfg.entry);
        Self::new_ordering(cfg, &node_ordering)
    }

    pub fn new_ordering(cfg: &Cfg<T>, node_ordering: &NodeOrdering<T>) -> Self {
        let domination_map = Self::domination_map(cfg, node_ordering);
        DomTree::from_edges(cfg.entry, domination_map)
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

#[derive(Debug)]
struct DJSpanningTree<T>(HashMap<T, HashSet<T>>);

impl<'a, T: Eq + Hash + 'a> Graph<'a, T, T> for DJSpanningTree<T> {
    type EdgeColl = HashSet<T>;

    fn lower_edge(&'a self, edge: &'a T) -> &'a T {
        edge
    }

    fn edges(&'a self) -> &HashMap<T, Self::EdgeColl> {
        &self.0
    }
}

impl<'a, T: Eq + Hash + 'a> DJSpanningTree<T> {
    fn is_sp_back(&self, from: &T, to: &T) -> bool {
        from == to || self.is_reachable(to, from)
    }

    fn is_sp_tree(&self, from: &T, to: &T) -> bool {
        self.children(from).contains(to)
    }

    fn is_sp_forward(&self, from: &T, to: &T) -> bool {
        !self.children(from).contains(to) && self.is_reachable(from, to)
    }

    fn is_sp_cross(&self, from: &T, to: &T) -> bool {
        !self.is_reachable(from, to) && !self.is_reachable(to, from)
    }
}

#[derive(Debug)]
struct Reducer<T: CfgLabel> {
    cfg: Cfg<T>,
    dj_graph: DJGraph<T>,
    spanning_tree: DJSpanningTree<T>,
    dom_tree: DomTree<T>,
}

impl<T: CfgLabel> Reducer<T> {
    fn new(cfg: Cfg<T>) -> Reducer<T> {
        let dom_tree: DomTree<T> = DomTree::new(&cfg);

        let mut dj_graph: HashMap<T, HashSet<DJEdge<T>>> = Default::default();
        for (&from, dom_edge_set) in dom_tree.edges() {
            dj_graph.insert(from, dom_edge_set.iter().map(|&x| DJEdge::D(x)).collect());
        }

        for (f, e) in cfg.edges() {
            let d_edge = dom_tree.edge(f);
            for t in e.iter() {
                let dominated = dom_tree.dominates(t);
                if !d_edge.contains(t) {
                    let j_edge = if dominated.contains(f) {
                        JEdge::B(*t)
                    } else {
                        JEdge::C(*t)
                    };
                    dj_graph.entry(*f).or_default().insert(DJEdge::J(j_edge));
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
        })
        .count(); // only for side effect computation

        Reducer {
            cfg,
            dj_graph: DJGraph(dj_graph),
            spanning_tree: DJSpanningTree(spanning_tree),
            dom_tree,
        }
    }

    fn sp_back(&self) -> HashSet<(T, T)> {
        let mut set: HashSet<(T, T)> = Default::default();

        let pre_post_order =
            PrePostOrder::start_from(&self.cfg.entry, |x| self.spanning_tree.children(x));

        let mut path: HashSet<&T> = Default::default();

        for traverse_action in pre_post_order {
            match traverse_action {
                VisitAction::Enter(x) => {
                    path.insert(x);

                    let sp_iter = self
                        .spanning_tree
                        .children(x)
                        .into_iter()
                        .filter(|c| path.contains(c))
                        .copied()
                        .map(|c| (*x, c));
                    for sp_back in sp_iter {
                        set.insert(sp_back);
                    }
                }
                VisitAction::Leave(x) => {
                    path.remove(&x);
                }
            }
        }

        assert!(set.iter().all(|(f, t)| self.spanning_tree.is_sp_back(f, t)));

        set
    }

    fn sed_set(&self, loop_set: HashSet<T>) -> (T, HashSet<T>) {
        let mut shared_idom: HashMap<&T, HashSet<&T>> = Default::default();
        loop_set
            .iter()
            .filter_map(|n| {
                self.dom_tree.imm_dominated(n).and_then(|idom| {
                    if !loop_set.contains(idom) {
                        Some((idom, n))
                    } else {
                        None
                    }
                })
            })
            .for_each(|(idom, n)| {
                shared_idom.entry(idom).or_default().insert(n);
            });
        let (&ext_dom, sed_set) = shared_idom
            .into_iter()
            .max_by_key(|(_idom, set)| set.len())
            .unwrap();
        (ext_dom, sed_set.into_iter().copied().collect())
    }

    fn domain(&self, h: &T, sed_set: &HashSet<T>) -> HashSet<T> {
        let dominated = self.dom_tree.dominates(h);
        sed_set
            .iter()
            .filter(|n| dominated.contains(n))
            .copied()
            .collect()
    }

    fn splt_loops(&self, top: &T, set: &HashSet<T>) -> bool {
        let mut cross = false;
        for child in self.dom_tree.children(top) {
            if (set.is_empty() || set.contains(child)) && self.splt_loops(child, set) {
                cross = true;
            }
        }
        if cross {
            self.handle_ir_children(top, set)
        }
        for predecessor in self.cfg.parents(top) {
            if self.spanning_tree.is_sp_back(predecessor, top)
                && !self.dom_tree.is_dom(top, predecessor)
            {
                return true;
            }
        }
        false
    }

    fn handle_ir_children(&self, top: &T, set: &HashSet<T>) {
        todo!()
    }
}

impl<T: CfgLabel> Reducer<SLabel<T>> {
    fn split_scc(&self, header: &SLabel<T>, scc: HashSet<SLabel<T>>) -> Self {
        let header_domain = self.domain(header, &scc);
        let copied_region: HashMap<SLabel<T>, SLabel<T>> = scc
            .difference(&header_domain)
            .copied()
            .map(|copied| (copied, copied.duplicate()))
            .collect();
        let reduced_edges: HashMap<_, _> = self
            .cfg
            .edges()
            .iter()
            .flat_map(|(from, edge)| {
                let from_domain = header_domain.contains(from);
                let from_copied = copied_region.contains_key(from);

                let mut new_edges: Vec<(SLabel<T>, CfgEdge<SLabel<T>>)> = vec![];

                if from_domain {
                    let ne = edge.map(|to| match copied_region.get(to) {
                        Some(copy) => *copy,
                        None => *to,
                    });
                    new_edges.push((*from, ne));
                } else {
                    new_edges.push((*from, edge.clone()));
                }

                if from_copied {
                    let ne = edge.map(|to| match copied_region.get(to) {
                        Some(copy) => *copy,
                        None => *to,
                    });
                    new_edges.push((*from, ne));
                }

                new_edges
            })
            .collect();

        let new_cfg = Cfg::from_edges(self.cfg.entry, reduced_edges);

        Reducer::new(new_cfg)
    }
}
