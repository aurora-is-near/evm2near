use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel};
use crate::traversal::graph::bfs::Bfs;
use crate::traversal::graph::dfs::{Dfs, DfsPost, DfsPostReverseInstantiator};
use std::collections::{HashMap, HashSet, VecDeque};
use std::vec::Vec;

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

        let mut merge_nodes: HashSet<TLabel> = HashSet::new();
        let mut loop_nodes: HashSet<TLabel> = HashSet::new();
        let mut if_nodes: HashSet<TLabel> = HashSet::new();

        let in_edges = cfg.in_edges();

        for &n in cfg.nodes() {
            let in_edges_count = in_edges.get(&n).map_or(0, |v| {
                v.iter()
                    .filter(|&&from| node_ordering.is_forward(from, n))
                    .count()
            });
            if in_edges_count > 1 {
                merge_nodes.insert(n);
            }

            let reachable: HashSet<_> =
                Bfs::start_from_except(n, |&l| cfg.children(&l).into_iter().copied()).collect();
            for &c in cfg.children(&n).into_iter() {
                if node_ordering.is_backward(n, c) && reachable.contains(&c) {
                    loop_nodes.insert(c);
                }
            }

            if let CfgEdge::Cond(_, _) = cfg.edges().get(&n).unwrap() {
                if_nodes.insert(n);
            }
        }

        let domination_map = Self::domination_tree(&cfg, &node_ordering, cfg.entry);
        let domination_vec = Vec::from_iter(domination_map);
        let domination = DomTree::from(domination_vec);

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

        let reached = Dfs::start_from(origin, |&n| {
            let mut ch = cfg.children(&n);
            ch.remove(&cur_id);
            ch.into_iter().copied()
        });
        for id in reached {
            reachable_set.remove(&id);
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
pub struct DomTree<TLabel: CfgLabel> {
    dominates: HashMap<TLabel, HashSet<TLabel>>,
    pub(crate) dominated: HashMap<TLabel, TLabel>,
}

impl<TLabel: CfgLabel> From<Vec<(TLabel, TLabel)>> for DomTree<TLabel> {
    fn from(edges: Vec<(TLabel, TLabel)>) -> Self {
        let dominated = HashMap::from_iter(edges.iter().copied());
        let mut dominates: HashMap<TLabel, HashSet<TLabel>> = HashMap::new();

        for (dominated, dominator) in edges {
            dominates.entry(dominator).or_default().insert(dominated);
        }

        DomTree {
            dominates,
            dominated,
        }
    }
}

impl<TLabel: CfgLabel> DomTree<TLabel> {
    pub(crate) fn immediately_dominated_by(&self, label: TLabel) -> HashSet<TLabel> {
        self.dominates
            .get(&label)
            .unwrap_or(&HashSet::new())
            .to_owned()
    }
}

pub struct NodeOrdering<TLabel: CfgLabel> {
    pub(crate) idx: HashMap<TLabel, usize>,
    vec: Vec<TLabel>,
}

impl<TLabel: CfgLabel> NodeOrdering<TLabel> {
    pub fn new(cfg: &Cfg<TLabel>, entry: TLabel) -> Self {
        let vec =
            DfsPost::<_, _, HashSet<_>>::reverse(entry, |x| cfg.children(x).into_iter().copied());
        let idx: HashMap<TLabel, usize> = vec.iter().enumerate().map(|(i, &n)| (n, i)).collect();
        Self { vec, idx }
    }

    pub fn is_backward(&self, from: TLabel, to: TLabel) -> bool {
        self.idx
            .get(&from)
            .zip(self.idx.get(&to))
            .map(|(&f, &t)| f > t)
            .unwrap()
    }

    pub fn is_forward(&self, from: TLabel, to: TLabel) -> bool {
        !self.is_backward(from, to)
    }

    pub fn sequence(&self) -> &Vec<TLabel> {
        &self.vec
    }
}
