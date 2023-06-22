pub mod dj_graph;
pub mod dj_spanning_tree;

use core::panic;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::{Debug, Display, Formatter},
};

use crate::{
    graph::{
        reduction::dj_graph::{DJEdge, JEdge},
        Graph, GraphCopy,
    },
    traversal::graph::dfs::Dfs,
};

use self::{dj_graph::DJGraph, dj_spanning_tree::DJSpanningTree};

use super::{
    cfg::{Cfg, CfgEdge, CfgLabel},
    domtree::DomTree,
    GEdgeColl, GEdgeCollMappable,
};
pub type SVersion = usize;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct SLabel<TLabel: CfgLabel> {
    pub origin: TLabel,
    version: SVersion,
}

impl<TLabel: CfgLabel + Display> Display for SLabel<TLabel> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.origin, self.version)
    }
}

impl<TLabel: CfgLabel> Debug for SLabel<TLabel> {
    // why debug isnt automatically derived from display?
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}_{}", self.origin, self.version)
    }
}

impl<TLabel: CfgLabel> From<TLabel> for SLabel<TLabel> {
    fn from(origin: TLabel) -> Self {
        Self { origin, version: 0 }
    }
}

impl<TLabel: CfgLabel> SLabel<TLabel> {
    pub fn new(origin: TLabel, version: SVersion) -> Self {
        Self { origin, version }
    }

    pub fn duplicate(&self) -> Self {
        Self {
            origin: self.origin,
            version: self.version + 1,
        }
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

        //todo to .map_label
        let mut dj_graph: HashMap<T, HashSet<DJEdge<T>>> = Default::default();
        for (&from, dom_edge_set) in dom_tree.edges() {
            dj_graph.insert(from, dom_edge_set.iter().map(|&x| DJEdge::D(x)).collect());
        }

        for (f, e) in cfg.edges() {
            let d_edge = dom_tree.edge(f);
            for t in e.iter() {
                if !d_edge.contains(t) {
                    let j_edge = if dom_tree.is_dom(t, f) {
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

    fn domain<'a>(&'a self, h: &T, loop_set: &HashSet<&'a T>) -> HashSet<&T> {
        let dominated = self.dom_tree.dom(h);
        loop_set
            .iter()
            .filter(|&&n| dominated.contains(n))
            .copied()
            .collect()
    }
}

type SLabelRef<'a, T> = &'a SLabel<T>;

impl<T: CfgLabel> Reducer<SLabel<T>> {
    pub fn split_scc(&self, header: SLabelRef<T>, scc: HashSet<SLabelRef<T>>) -> Self {
        let scc_refs: HashSet<SLabelRef<T>> = scc.iter().copied().collect();
        let header_domain: HashSet<SLabelRef<T>> = self.domain(header, &scc_refs);
        let copied_region: HashMap<SLabelRef<T>, SLabel<T>> = scc_refs
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

                let ne = edge.map(|to| match copied_region.get(to) {
                    Some(copy) => *copy,
                    None => *to,
                });

                if from_domain {
                    new_edges.push((*from, ne.clone()));
                } else {
                    new_edges.push((*from, edge.clone()));
                }

                if from_copied {
                    new_edges.push((*copied_region.get(from).unwrap(), ne));
                }

                new_edges
            })
            .collect();

        let new_cfg = Cfg::from_edges(self.cfg.entry, reduced_edges);

        for n in new_cfg.nodes() {
            if !new_cfg.is_reachable(&new_cfg.entry, n) {
                panic!("node {:?} is not reachable from cfg root", n);
            }
        }

        Reducer::new(new_cfg)
    }

    #[allow(dead_code)]
    fn reduce(self) -> Self {
        let levels = self.dom_tree.levels().clone();

        let mut by_level: BTreeMap<usize, HashSet<SLabelRef<T>>> = Default::default();
        let mut max_level = 0;
        for (sl, &level) in &levels {
            max_level = usize::max(max_level, level);
            by_level.entry(level).or_default().insert(sl);
        }

        by_level
            .clone()
            .into_iter()
            .rev()
            .fold(self, |reducer, (level, slabels)| {
                let mut irreduceible_loop = false; // todo move irr actions directly into match?

                let sp_back = reducer.spanning_tree.sp_back(&reducer.cfg.entry);

                let transposed: HashMap<SLabelRef<T>, HashSet<SLabelRef<T>>> =
                    reducer.dj_graph.in_edges();

                for n in slabels {
                    for &m in transposed.get(&n).into_iter().flatten() {
                        for dj_to in reducer.dj_graph.edge(m) {
                            match dj_to {
                                // m !dom n
                                DJEdge::J(JEdge::C(to)) if to == n && sp_back.contains(&(m, n)) => {
                                    irreduceible_loop = true;
                                }
                                // m dom n
                                DJEdge::J(JEdge::B(to)) if to == n => {

                                    // reach under & collapse
                                    // todo is it really needed there?
                                    // now we are filtering only irreducible loops below, but it may be so that reducible loop will somehow blend with irreducible?
                                    // it should not be possible (if reducible loop is somehow connected to reducible one, it will be single irr liio),
                                    // so it is safe to skip `collapse` altogether?
                                }
                                _ => {}
                            }
                        }
                    }
                }
                if irreduceible_loop {
                    // subgraph by level & every scc simplification
                    let below_nodes = by_level
                        .clone()
                        .into_iter()
                        .flat_map(|(l, level_snodes)| {
                            if l >= level {
                                level_snodes
                            } else {
                                HashSet::new()
                            }
                        })
                        .collect::<HashSet<_>>();
                    let edges = reducer.cfg.edges().clone();
                    let graph_below_level: HashMap<SLabelRef<T>, HashSet<SLabelRef<T>>> = edges
                        .iter()
                        .filter_map(|(from, edges)| {
                            if below_nodes.contains(from) {
                                Some((
                                    from,
                                    edges
                                        .iter()
                                        .filter(|&a| below_nodes.contains(a))
                                        .collect::<HashSet<_>>(),
                                ))
                            } else {
                                None
                            }
                        })
                        .collect::<HashMap<_, _>>();

                    let irr_sccs: Vec<(SLabel<T>, HashSet<SLabelRef<T>>)> = graph_below_level
                        .kosaraju_scc()
                        .into_iter()
                        .filter(|scc| scc.len() > 1)
                        .filter_map(|scc| {
                            let headers: Vec<&&SLabel<T>> = scc // get all headers of given scc/loop (nodes on `level + 1`)
                                .iter()
                                .filter(|n| *levels.get(n).unwrap() == level)
                                .collect();
                            if headers.len() > 1 {
                                // ensure that that given loop is irreducible (have at least two header nodes)
                                let header = **headers[0]; // todo select header by weight of its entire domain
                                Some((header, scc))
                            } else {
                                None
                            }
                        })
                        .collect();

                    irr_sccs
                        .into_iter()
                        .fold(reducer, |reducer: Reducer<_>, (header, scc)| {
                            reducer.split_scc(&header, scc)
                        })
                } else {
                    reducer
                }
            })
    }
}

pub fn reduce<T: CfgLabel>(cfg: &Cfg<T>) -> Cfg<SLabel<T>> {
    let slabel_cfg = cfg.map_label(|&n| SLabel::new(n, 0));
    let reducer = Reducer::new(slabel_cfg);
    let cfg = reducer.reduce().cfg;
    cfg
}

pub fn check_reduction<TLabel: CfgLabel>(
    origin_cfg: &Cfg<TLabel>,
    reduced_cfg: &Cfg<SLabel<TLabel>>,
) -> bool {
    let reduced_nodes = reduced_cfg.nodes();
    let mut origin_mapping: HashMap<TLabel, HashSet<SLabel<TLabel>>> = Default::default();
    for &x in reduced_nodes.iter() {
        origin_mapping.entry(x.origin).or_default().insert(*x);
    }

    origin_cfg.edges().iter().all(|(from, e)| {
        origin_mapping
            .get(from)
            .unwrap()
            .iter()
            .all(|&r_from| &reduced_cfg.edge(&r_from).map(|x| x.origin) == e)
    })
}

#[cfg(test)]
mod tests {
    use crate::graph::cfg::Cfg;
    use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
    use crate::graph::reduction::{check_reduction, reduce};

    #[test]
    fn simplest() {
        let cfg = Cfg::from_edges(
            0,
            vec![(0, Cond(1, 2)), (1, Uncond(2)), (2, Cond(3, 1))]
                .into_iter()
                .collect(),
        );
        let reduced = reduce(&cfg);

        assert!(check_reduction(&cfg, &reduced));
    }

    #[test]
    fn irreducible() {
        let cfg = Cfg::from_edges(
            0,
            vec![
                (0, Cond(1, 2)),
                (1, Uncond(4)),
                (4, Uncond(2)),
                (2, Cond(3, 1)),
            ]
            .into_iter()
            .collect(),
        );
        let reduced = reduce(&cfg);

        assert!(check_reduction(&cfg, &reduced));
    }

    #[test]
    fn moderate() {
        let cfg = Cfg::from_edges(
            0,
            vec![
                (0, Cond(1, 2)),
                (1, Cond(3, 4)),
                (2, Cond(3, 5)),
                (3, Uncond(4)),
                (4, Cond(2, 5)),
            ]
            .into_iter()
            .collect(),
        );
        let reduced = reduce(&cfg);

        assert!(check_reduction(&cfg, &reduced));
    }

    #[test]
    fn new() {
        let cfg = Cfg::from_edges(
            0,
            vec![
                (0, Cond(1, 3)),
                (1, Uncond(2)),
                (2, Cond(5, 1)),
                (3, Uncond(4)),
                (4, Cond(5, 3)),
                (5, Cond(6, 7)),
                (6, Terminal),
                (7, Cond(1, 3)),
            ]
            .into_iter()
            .collect(),
        );
        let reduced = reduce(&cfg);

        std::fs::write(
            "irr_new_cfg.dot",
            format!("digraph {{{}}}", cfg.cfg_to_dot("irr_new_cfg")),
        )
        .expect("fs error");

        std::fs::write(
            "irr_new_cfg_reduced.dot",
            format!("digraph {{{}}}", reduced.cfg_to_dot("irr_new_cfg_reduced")),
        )
        .expect("fs error");

        assert!(check_reduction(&cfg, &reduced));
    }
}
