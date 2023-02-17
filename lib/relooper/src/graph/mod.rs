use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel};
use crate::graph::enrichments::{DomTree, NodeOrdering};
use crate::traversal::graph::bfs::Bfs;
use crate::traversal::graph::dfs::Dfs;
use std::collections::{HashMap, HashSet, VecDeque};
use std::vec::Vec;

pub mod caterpillar;
pub mod cfg;
pub mod dot_debug;
pub mod enrichments;
pub mod relooper;
pub mod supergraph;

pub struct EnrichedCfg<TLabel: CfgLabel> {
    cfg: Cfg<TLabel>,
    node_ordering: NodeOrdering<TLabel>,
    domination: DomTree<TLabel>,
    merge_nodes: HashSet<TLabel>,
    loop_nodes: HashSet<TLabel>,
    if_nodes: HashSet<TLabel>,
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

    //TODO move to enrichments' DomTree constructor
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
