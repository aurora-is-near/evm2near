pub mod dj_graph;
pub mod dj_spanning_tree;

use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Display, Formatter},
};

use crate::graph::{reduction::dj_graph::DJEdge, Graph, GraphCopy};

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
    pub version: SVersion,
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

    pub fn duplicate(&self, version: SVersion) -> Self {
        Self {
            origin: self.origin,
            version,
        }
    }
}

#[derive(Debug)]
struct Reducer<T: CfgLabel> {
    cfg: Cfg<T>,
    dj_graph: DJGraph<T>,
    dj_spanning_tree: DJSpanningTree<T>,
    dom_tree: DomTree<T>,
    last_version: SVersion,
}

impl<T: CfgLabel> Reducer<T> {
    fn new(cfg: Cfg<T>, last_version: SVersion) -> Reducer<T> {
        let dom_tree: DomTree<T> = DomTree::new(&cfg);

        let dj_graph = DJGraph::new(&cfg, &dom_tree);

        let dj_spanning_tree = DJSpanningTree::new(cfg.entry, &dj_graph);

        Reducer {
            cfg,
            dj_graph,
            dj_spanning_tree,
            dom_tree,
            last_version,
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
        let mut copies_counter: SVersion = 0;
        let scc_refs: HashSet<SLabelRef<T>> = scc.iter().copied().collect();
        let header_domain: HashSet<SLabelRef<T>> = self.domain(header, &scc_refs);
        let copied_region: HashMap<SLabelRef<T>, SLabel<T>> = scc_refs
            .difference(&header_domain)
            .copied()
            .map(|copied| {
                copies_counter += 1;
                (copied, copied.duplicate(self.last_version + copies_counter))
            })
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

        let red = Reducer::new(new_cfg, self.last_version + copies_counter);
        println!("reducer: {:?}", red);

        for n in red.cfg.nodes() {
            if !red.cfg.is_reachable(&red.cfg.entry, n) {
                panic!("node {:?} is not reachable from cfg root", n);
            }
        }

        red
    }

    fn reduce(self) -> Self {
        let max_level = self.dom_tree.max_level();
        println!("max level: {}", max_level);
        (0..(max_level + 1)).rev().fold(self, |reducer, level| {
            let mut reducer = reducer;
            loop {
                let by_level = reducer.dom_tree.by_level();
                let level_nodes = by_level.get(&level).unwrap();

                let mut irreduceible_loop = false; // todo move irr actions directly into match?

                let sp_back = reducer.dj_spanning_tree.sp_back(&reducer.cfg.entry);

                println!("|||| level {}, sp_back: {:?}", level, sp_back);

                let transposed: HashMap<SLabelRef<T>, HashSet<SLabelRef<T>>> =
                    reducer.dj_graph.in_edges();

                for n in level_nodes {
                    for &m in transposed.get(&n).into_iter().flatten() {
                        println!("dj_to {:?} -> {:?}", m, n);
                        if sp_back.contains(&(m, n)) {
                            print!("sp-back, ");
                            let m_dj_edges = reducer.dj_graph.edge(m);
                            if m_dj_edges.contains(&DJEdge::JC(*n)) {
                                println!("CJ IRRRRRRRRRRRRRRR");
                                irreduceible_loop = true;
                            } else {
                                println!("BJ");
                            }
                        } else {
                            println!("no sp-back");
                        }
                        // for dj_to in reducer.dj_graph.edge(m) {
                        //     println!("dj_to {:?} -> {:?}", m, n);
                        //     match dj_to {
                        //         // m !dom n
                        //         DJEdge::J(JEdge::C(to)) if to == n && sp_back.contains(&(m, n)) => {
                        //             println!("irr found!");
                        //             irreduceible_loop = true;
                        //         }
                        //         // m dom n
                        //         DJEdge::J(JEdge::B(to)) if to == n => {

                        //             // reach under & collapse
                        //             // todo is it really needed there?
                        //             // now we are filtering only irreducible loops below, but it may be so that reducible loop will somehow blend with irreducible?
                        //             // it should not be possible (if reducible loop is somehow connected to reducible one, it will be single irr liio),
                        //             // so it is safe to skip `collapse` altogether?
                        //         }
                        //         _ => {}
                        //     }
                        // }
                    }
                }
                if irreduceible_loop {
                    // subgraph by level & every scc simplification
                    let below_nodes = reducer
                        .dom_tree
                        .by_level()
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
                        // filter out all `trivial` scc (only loops of len > 1 will stay)
                        .filter(|scc| scc.len() > 1)
                        .filter_map(|scc| {
                            let headers: Vec<&&SLabel<T>> = scc // get all headers of given scc/loop (nodes on `level`)
                                .iter()
                                .filter(|n| *reducer.dom_tree.levels().get(n).unwrap() == level)
                                .collect();
                            // ensure that that given loop is irreducible (have at least two header nodes)
                            if headers.len() > 1 {
                                // let (&&&header, domain) = headers
                                //     .iter()
                                //     .map(|h| (h, reducer.domain(h, &scc)))
                                //     .max_by_key(|(_, domain)| domain.len())
                                //     .unwrap();
                                // println!("h: {:?}\n\td: {:#?}", header, domain);
                                let header = **headers[0];
                                Some((header, scc))
                            } else {
                                None
                            }
                        })
                        .collect();
                    // println!("-----");

                    println!("irr sccs count: {}", irr_sccs.len());
                    reducer = irr_sccs
                        .into_iter()
                        .fold(reducer, |reducer: Reducer<_>, (header, scc)| {
                            reducer.split_scc(&header, scc)
                        })
                } else {
                    println!("---- level {} finished", level);
                    break;
                }
            }
            reducer
        })
    }
}

fn check_reduced_edges<TLabel: CfgLabel>(
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

pub fn reduce<T: CfgLabel>(cfg: &Cfg<T>) -> Cfg<SLabel<T>> {
    let slabel_cfg = cfg.map_label(|&n| SLabel::new(n, 0));
    let reducer = Reducer::new(slabel_cfg, 0);
    let reduced_cfg = reducer.reduce().cfg;
    assert!(check_reduced_edges(cfg, &reduced_cfg));
    // assert!(check_reduced_loop_headers(&reduced_cfg));
    reduced_cfg
}

#[cfg(test)]
mod tests {
    use crate::graph::cfg::Cfg;
    use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
    use crate::graph::enrichments::EnrichedCfg;
    use crate::graph::reduction::{check_reduced_edges, reduce};
    use crate::graph::Graph;

    #[test]
    fn simplest() {
        let cfg = Cfg::from_edges(
            0,
            vec![(0, Cond(1, 2)), (1, Uncond(2)), (2, Cond(3, 1))]
                .into_iter()
                .collect(),
        );
        let reduced = reduce(&cfg);

        assert!(check_reduced_edges(&cfg, &reduced));
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

        assert!(check_reduced_edges(&cfg, &reduced));
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

        assert!(check_reduced_edges(&cfg, &reduced));
    }

    #[test]
    fn same_level_irr() {
        let cfg = Cfg::from_edges(
            0,
            vec![
                (0, Cond(1, 2)),
                (1, Cond(3, 4)),
                (2, Cond(5, 6)),
                (3, Cond(4, 7)),
                (4, Cond(5, 3)),
                (5, Cond(6, 4)),
                (6, Uncond(5)),
                (7, Uncond(8)),
                (8, Terminal),
            ]
            .into_iter()
            .collect(),
        );
        let reduced = reduce(&cfg);

        println!("reduced node count: {}", reduced.nodes().len());

        assert!(check_reduced_edges(&cfg, &reduced));

        let enriched = EnrichedCfg::new(reduced);
        enriched.reloop();
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

        assert!(check_reduced_edges(&cfg, &reduced));
    }

    #[test]
    fn nested_irr() {
        let cfg = Cfg::from_edges(
            0,
            vec![
                (0, Cond(1, 4)),
                (1, Cond(2, 3)),
                (2, Uncond(3)),
                (3, Cond(4, 2)),
                (4, Cond(5, 1)),
                (5, Terminal),
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
        assert!(check_reduced_edges(&cfg, &reduced));

        let enriched = EnrichedCfg::new(reduced);
        enriched.reloop();
    }
}
