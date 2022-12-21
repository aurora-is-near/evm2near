use crate::cfg::CfgEdge::*;
use crate::cfg::{Cfg, CfgLabel};
use crate::re_graph::ReBlockType::{Block, If, Loop};
use crate::re_graph::ReEdge::Next;
use crate::re_graph::ReLabel::{FromCfg, Generated};
use crate::re_graph::{ReBlock, ReBlockType, ReEdge, ReGraph, ReLabel};
use crate::traversal::graph;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct DomTree {
    pub dominates: HashMap<CfgLabel, HashSet<CfgLabel>>,
    pub dominated: HashMap<CfgLabel, CfgLabel>,
}

impl From<Vec<(CfgLabel, CfgLabel)>> for DomTree {
    fn from(edges: Vec<(CfgLabel, CfgLabel)>) -> Self {
        let dominated = HashMap::from_iter(edges.iter().copied());
        let mut dominates: HashMap<CfgLabel, HashSet<CfgLabel>> = HashMap::new();

        for (dominated, dominator) in edges {
            dominates.entry(dominator).or_default().insert(dominated);
        }

        DomTree {
            dominates,
            dominated,
        }
    }
}

impl DomTree {
    fn immediately_dominated_by(&self, label: CfgLabel) -> HashSet<CfgLabel> {
        self.dominates
            .get(&label)
            .unwrap_or(&HashSet::new())
            .to_owned()
    }
}

#[derive(Clone, Copy)]
enum Context {
    If,
    LoopHeadedBy(CfgLabel),
    BlockHeadedBy(CfgLabel),
}

struct Relooper<'a> {
    cfg: &'a Cfg,
    entry: CfgLabel,
    // reachability: HashMap<CfgLabel, HashSet<CfgLabel>>,
    postorder_rev: HashMap<CfgLabel, usize>,
    domitation: DomTree,
    last_generated_label: usize,
    ifs: HashSet<CfgLabel>,
    loops: HashSet<CfgLabel>,
    merges: HashSet<CfgLabel>,
    blocks: HashMap<ReLabel, ReBlock>,
}

impl<'a> Relooper<'a> {
    fn generate_label(&mut self) -> ReLabel {
        self.last_generated_label += 1;
        Generated(self.last_generated_label)
    }

    // fn reachable(&self, l: &CfgLabel) -> &HashSet<CfgLabel> {
    //     self.reachability
    //         .get(l)
    //         .expect("that label should be in the initial cfg")
    // }

    fn children(&self, label: CfgLabel) -> Vec<CfgLabel> {
        let mut res = self
            .domitation
            .immediately_dominated_by(label)
            .into_iter()
            .collect::<Vec<_>>();
        res.sort_by_key(|n| {
            self.postorder_rev
                .get(n)
                .expect("every node should have postorder numbering")
        });
        res
    }

    fn is_backward(&self, from: CfgLabel, to: CfgLabel) -> bool {
        self.postorder_rev
            .get(&from)
            .and_then(|&f| self.postorder_rev.get(&to).map(|&t| f < t))
            .unwrap()
    }

    fn new_block(&mut self, typ: ReBlockType, label: ReLabel, next: ReEdge) -> ReLabel {
        let block = ReBlock::new(typ, label, next);
        assert!(self.blocks.insert(label, block).is_none());
        label
    }

    fn gen_block(&mut self, typ: ReBlockType, next: ReEdge) -> ReLabel {
        let label = self.generate_label();
        self.new_block(typ, label, next)
    }

    fn do_branch(&self, from: CfgLabel, to: CfgLabel, context: &Vec<Context>) -> Option<usize> {
        if self.is_backward(from, to) || self.merges.contains(&to) {
            let idx_coll = context
                .iter()
                .enumerate()
                .filter_map(|(i, c)| match c {
                    Context::LoopHeadedBy(label) | Context::BlockHeadedBy(label)
                        if *label == to =>
                    {
                        Some(context.len() - i - 1)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();

            assert_eq!(idx_coll.len(), 1);
            let &jump_idx = idx_coll
                .first()
                .expect("suitable jump target not found in context");
            Some(jump_idx)
        } else {
            None
        }
    }

    fn node_within(
        &mut self,
        node: CfgLabel,
        merges: &Vec<CfgLabel>,
        context: &Vec<Context>,
    ) -> ReLabel {
        let mut current_merges = merges.clone();
        match current_merges.pop() {
            Some(merge) => {
                let mut new_ctx = context.clone();
                new_ctx.push(Context::BlockHeadedBy(merge));
                let inner = self.node_within(node, &current_merges, &new_ctx);
                let curr = self.do_tree(merge, context);

                // let c = self.gen_block(Block, )
                todo!("concat inner & curr, so returning value needed to be changed to smth")
            }
            None => {
                match self.cfg.edge(node) {
                    Uncond(u) => match self.do_branch(node, *u, context) {
                        Some(br) => self.new_block(Block, FromCfg(node), ReEdge::Uncond(br)),
                        None => {
                            let next_block = self.do_tree(*u, context);
                            self.new_block(Block, FromCfg(node), Next(next_block))
                        }
                    },
                    Cond(true_label, false_label) => {
                        let mut if_context = context.clone();
                        if_context.push(Context::If);

                        let true_branch = match self.do_branch(node, *true_label, &if_context) {
                            Some(br) => todo!(),
                            None => todo!(),
                        };
                        let false_branch = self.do_branch(node, *false_label, &if_context);

                        // ReBlock::new(ReBlockType::If(), )
                        todo!()
                    }
                    Terminal => todo!(),
                }
            }
        }
    }

    fn gen_node(&mut self, node: CfgLabel, context: &Vec<Context>) -> ReLabel {
        let merge_children: Vec<CfgLabel> = self
            .children(node)
            .into_iter()
            .filter(|n| self.merges.contains(n))
            .collect();
        self.node_within(node, &merge_children, context)
    }

    fn do_tree(&mut self, node: CfgLabel, context: &Vec<Context>) -> ReLabel {
        if self.loops.contains(&node) {
            let mut ctx = context.clone();
            ctx.push(Context::LoopHeadedBy(node));
            let next_block = self.gen_node(node, context);
            let re_label = FromCfg(node);
            let block = ReBlock::new(Loop, re_label, Next(next_block));
            re_label
        } else {
            self.gen_node(node, context)
        }
    }

    // fn dummy() -> ReGraph {
    //     let mut m = HashMap::new();
    //     m.insert(
    //         FromCfg(0),
    //         ReBlock::new(
    //             If(FromCfg(1)),
    //             FromCfg(0),
    //             ReEdge::Cond(FromCfg(0), FromCfg(2)),
    //         ),
    //     );
    //     m.insert(
    //         FromCfg(1),
    //         ReBlock::new(Block, FromCfg(1), ReEdge::Uncond(FromCfg(3))),
    //     );
    //     m.insert(
    //         FromCfg(2),
    //         ReBlock::new(Block, FromCfg(2), ReEdge::Uncond(FromCfg(3))),
    //     );
    //     m.insert(
    //         FromCfg(3),
    //         ReBlock::new(Block, FromCfg(3), ReEdge::Uncond(FromCfg(4))),
    //     );
    //     m.insert(
    //         FromCfg(4),
    //         ReBlock::new(Loop, FromCfg(4), ReEdge::Uncond(FromCfg(0))),
    //     );
    //
    //     ReGraph(m)
    // }
}

pub fn reloop(cfg: &Cfg, entry: CfgLabel) -> ReGraph {
    let nodes = cfg.nodes();

    let reachability: HashMap<CfgLabel, HashSet<CfgLabel>> = nodes
        .into_iter()
        .map(|l| {
            let reachable: HashSet<_> =
                graph::bfs::Bfs::start_from_except(l, |&l| cfg.children(l).into_iter()).collect();
            (l, reachable)
        })
        .collect();
    println!("n{:?}", reachability);

    let mut relooper = Relooper {
        cfg,
        entry,
        postorder_rev: Default::default(), //TODO
        domitation: Default::default(),    //TODO
        last_generated_label: 0,
        ifs: Default::default(),
        loops: Default::default(),
        merges: Default::default(),
        blocks: Default::default(),
    };

    let re_entry = relooper.do_tree(entry, &Vec::new());

    ReGraph {
        start: re_entry,
        blocks: relooper.blocks,
    }
}
