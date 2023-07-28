use crate::graph::cfg::{Cfg, CfgEdge, CfgLabel};
use crate::traversal::graph::bfs::Bfs;
use std::collections::HashSet;

use super::{domtree::DomTree, node_ordering::NodeOrdering, Graph};

/// 'Option' in 'Thunk' version serves the only purpose:
/// to overcome 'Default' requirement of 'mem::...' operations
enum Lazy<T, F> {
    Resolved(T),
    Thunk(Option<F>),
}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    fn new(init: F) -> Self {
        Self::Thunk(Some(init))
    }

    pub fn force(&mut self) -> &T {
        if let Self::Resolved(value) = self {
            return value;
        }

        let mut thunk: Lazy<T, F> = Self::Thunk(None);
        std::mem::swap(&mut thunk, self);

        match thunk {
            Self::Thunk(Some(f)) => {
                let value = f();
                *self = Self::Resolved(value);
                match self {
                    Self::Resolved(value) => value,
                    Self::Thunk(_) => unreachable!("self re-assigned to Resolved"),
                }
            }
            Self::Thunk(None) => unreachable!("Lazy was created with empty Thunk"),
            Self::Resolved(_) => unreachable!("Handled by early return"),
        }
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
