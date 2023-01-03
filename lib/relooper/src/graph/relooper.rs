use crate::graph::cfg::{CfgEdge::*, CfgLabel};
use crate::graph::relooper::ReBlock::*;
use crate::graph::EnrichedCfg;

#[derive(Debug)]
pub struct ReSeq(pub Vec<ReBlock>);

#[derive(Debug)]
pub enum ReBlock {
    Block(ReSeq),
    Loop(ReSeq),
    If(ReSeq, ReSeq),

    Actions(CfgLabel),
    Br(usize),
    Return,
}

impl ReBlock {
    pub(crate) fn concat(self, other: ReSeq) -> ReSeq {
        let mut blocks = vec![self];
        blocks.extend(other.0);
        ReSeq(blocks)
    }
}

impl ReSeq {
    pub(crate) fn single(block: ReBlock) -> ReSeq {
        ReSeq(vec![block])
    }
}

#[derive(Clone, Copy)]
enum Context {
    If,
    LoopHeadedBy(CfgLabel),
    BlockHeadedBy(CfgLabel),
}

impl EnrichedCfg {
    fn children_ord(&self, label: CfgLabel) -> Vec<CfgLabel> {
        let mut res = self
            .domination
            .immediately_dominated_by(label)
            .into_iter()
            .collect::<Vec<_>>();
        res.sort_by_key(|n| {
            self.node_ordering
                .idx
                .get(n)
                .expect("every node should have postorder numbering")
        });
        res
    }

    fn do_branch(&self, from: CfgLabel, to: CfgLabel, context: &Vec<Context>) -> ReSeq {
        if self.node_ordering.is_backward(from, to) || self.merge_nodes.contains(&to) {
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
            ReSeq(vec![Br(jump_idx)]) //TODO is seq really necessary there?
        } else {
            self.do_tree(to, context)
        }
    }

    fn node_within(&self, node: CfgLabel, merges: &Vec<CfgLabel>, context: &Vec<Context>) -> ReSeq {
        let mut current_merges = merges.clone();
        match current_merges.pop() {
            Some(merge) => {
                let mut new_ctx = context.clone();
                new_ctx.push(Context::BlockHeadedBy(merge));
                let inner = self.node_within(node, &current_merges, &new_ctx);
                let merge_block = self.do_tree(merge, context);

                Block(inner).concat(merge_block)
            }
            None => {
                let actions = Actions(node);
                match *self.cfg.edge(node) {
                    Uncond(u) => actions.concat(self.do_branch(node, u, context)),
                    Cond(true_label, false_label) => {
                        let mut if_context = context.clone();
                        if_context.push(Context::If);

                        let true_branch = self.do_branch(node, true_label, &if_context);
                        let false_branch = self.do_branch(node, false_label, &if_context);

                        ReSeq(vec![If(true_branch, false_branch)])
                    }
                    Terminal => ReSeq(vec![Return]),
                }
            }
        }
    }

    fn gen_node(&self, node: CfgLabel, context: &Vec<Context>) -> ReSeq {
        let merge_children: Vec<CfgLabel> = self
            .children_ord(node)
            .into_iter()
            .filter(|n| self.merge_nodes.contains(n))
            .collect();
        self.node_within(node, &merge_children, context)
    }

    fn do_tree(&self, node: CfgLabel, context: &Vec<Context>) -> ReSeq {
        if self.loop_nodes.contains(&node) {
            let mut ctx = context.clone();
            ctx.push(Context::LoopHeadedBy(node));
            ReSeq::single(Loop(self.gen_node(node, &ctx)))
        } else {
            self.gen_node(node, context)
        }
    }

    pub fn reloop(&self) -> ReSeq {
        self.do_tree(self.cfg.entry, &Vec::new())
    }
}
