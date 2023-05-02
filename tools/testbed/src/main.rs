use graph_samples::*;
use relooper::graph::cfg::Graph;
use relooper::graph::enrichments::EnrichedCfg;
use relooper::graph::supergraph::reduce;

#[allow(dead_code)]
mod graph_samples {
    use std::collections::HashMap;

    use relooper::graph::cfg::Cfg;
    use relooper::graph::cfg::CfgEdge;
    use relooper::graph::cfg::Graph;

    pub fn irreducible() -> Cfg<i32> {
        let mut cfg = Cfg::new(0);
        let d = 2;
        let exit = 3;
        cfg.add_edge(0, CfgEdge::Uncond(1));
        cfg.add_edge(1, CfgEdge::Uncond(d));
        let dyn_outs: Vec<_> = (0..10).into_iter().map(|x| x + 5).collect();
        cfg.add_edge(
            d,
            CfgEdge::Switch(dyn_outs.clone().into_iter().enumerate().collect()),
        );
        for i in dyn_outs {
            if i % 2 == 0 {
                cfg.add_edge(i, CfgEdge::Uncond(exit));
            } else {
                cfg.add_edge(i, CfgEdge::Cond(exit, 1));
            }
        }
        cfg.add_edge(exit, CfgEdge::Cond(d, 4));
        cfg
    }

    pub fn irreducible1() -> Cfg<i32> {
        Cfg::from_edges(
            0,
            HashMap::from_iter(vec![
                (0, CfgEdge::Uncond(1)),
                (1, CfgEdge::Switch(vec![(2, 2), (3, 3), (4, 4)])),
                (2, CfgEdge::Cond(5, 3)),
                (3, CfgEdge::Cond(5, 4)),
                (4, CfgEdge::Cond(5, 2)),
            ]),
        )
    }

    pub fn irreducible2() -> Cfg<i32> {
        Cfg::from_edges(
            0,
            HashMap::from_iter(vec![
                (0, CfgEdge::Uncond(1)),
                (1, CfgEdge::Switch(vec![(2, 2), (3, 3), (4, 4)])),
                (2, CfgEdge::Cond(5, 3)),
                (3, CfgEdge::Switch(vec![(2, 2), (4, 4), (5, 5)])),
                (4, CfgEdge::Switch(vec![(2, 2), (3, 3), (5, 5)])),
            ]),
        )
    }

    pub fn irreducible_tr() -> Cfg<&'static str> {
        Cfg::from_edges(
            "e",
            HashMap::from_iter(vec![
                ("e", CfgEdge::Switch(vec![(0, "b"), (1, "a"), (2, "f")])),
                ("b", CfgEdge::Uncond("a")),
                ("a", CfgEdge::Cond("c", "b")),
                ("f", CfgEdge::Uncond("a")),
                ("c", CfgEdge::Cond("d", "b")),
                ("d", CfgEdge::Uncond("f")),
            ]),
        )
    }
}

fn main() {
    let cfg = irreducible_tr();

    std::fs::write(
        "base.dot",
        format!("digraph {{{}}}", cfg.cfg_to_dot("base")),
    )
    .expect("fs error");

    let reduced = reduce(&cfg);
    std::fs::write(
        "reduced.dot",
        format!("digraph {{{}}}", reduced.cfg_to_dot("reduced")),
    )
    .expect("fs error");

    println!("{:?}", reduced.nodes().len());

    let enriched = EnrichedCfg::new(reduced);
    std::fs::write(
        "enriched.dot",
        format!("digraph {{{}}}", enriched.cfg_to_dot("enriched")),
    )
    .expect("fs error");

    let relooped = enriched.reloop();
    std::fs::write("relooped.dot", format!("digraph {{{}}}", relooped.to_dot())).expect("fs error");
}
