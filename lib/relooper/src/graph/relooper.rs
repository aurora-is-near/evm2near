use crate::graph::cfg::{CfgEdge::*, CfgLabel};
use crate::graph::relooper::ReBlock::*;
use crate::graph::EnrichedCfg;

#[derive(Debug)]
pub struct ReSeq<TLabel: CfgLabel>(pub Vec<ReBlock<TLabel>>);

/// describes relooped graph structure
/// consists of three "container" variants and several "actions" variants
/// containers defines tree structure, actions denotes runtime control flow behaviour
#[derive(Debug)]
pub enum ReBlock<TLabel: CfgLabel> {
    Block(ReSeq<TLabel>),
    Loop(ReSeq<TLabel>),
    If(ReSeq<TLabel>, ReSeq<TLabel>),

    Actions(TLabel),
    Br(usize),
    Return,
}

impl<TLabel: CfgLabel> ReBlock<TLabel> {
    pub(crate) fn concat(self, other: ReSeq<TLabel>) -> ReSeq<TLabel> {
        let mut blocks = vec![self];
        blocks.extend(other.0);
        ReSeq(blocks)
    }
}

impl<TLabel: CfgLabel> ReSeq<TLabel> {
    pub(crate) fn single(block: ReBlock<TLabel>) -> ReSeq<TLabel> {
        ReSeq(vec![block])
    }
}

#[derive(Clone, Copy)]
enum Context<TLabel: CfgLabel> {
    If,
    LoopHeadedBy(TLabel),
    BlockHeadedBy(TLabel),
}

impl<TLabel: CfgLabel> EnrichedCfg<TLabel> {
    /// that defines order of graph traversal for nested nodes generation
    /// returns vector of immediately dominated nodes ordered according to reversed postorder traversal (in cfg graph)
    fn children_ord(&self, label: TLabel) -> Vec<TLabel> {
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

    /// either generates branch node or "fallthrough" next node
    fn do_branch(&self, from: TLabel, to: TLabel, context: &Vec<Context<TLabel>>) -> ReSeq<TLabel> {
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
            ReSeq(vec![Br(jump_idx)])
        } else {
            self.do_tree(to, context)
        }
    }

    /// in case of multiple merge nodes beneath current node, lays down merge nodes first
    /// otherwise, generates current node and branches to merge nodes generated on previous step (and above in tree structure)
    fn node_within(
        &self,
        node: TLabel,
        merges: &[TLabel],
        context: &Vec<Context<TLabel>>,
    ) -> ReSeq<TLabel> {
        let mut current_merges = Vec::from(merges);
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
                let other = match *self.cfg.edge(&node) {
                    Uncond(u) => self.do_branch(node, u, context),
                    Cond(true_label, false_label) => {
                        let mut if_context = context.clone();
                        if_context.push(Context::If);

                        let true_branch = self.do_branch(node, true_label, &if_context);
                        let false_branch = self.do_branch(node, false_label, &if_context);

                        ReSeq(vec![If(true_branch, false_branch)])
                    }
                    Terminal => ReSeq(vec![Return]),
                };
                actions.concat(other)
            }
        }
    }

    /// helper function for finding all the merge nodes depending on current node
    fn gen_node(&self, node: TLabel, context: &Vec<Context<TLabel>>) -> ReSeq<TLabel> {
        let merge_children: Vec<TLabel> = self
            .children_ord(node)
            .into_iter()
            .filter(|n| self.merge_nodes.contains(n))
            .collect();
        self.node_within(node, &merge_children, context)
    }

    /// main function for node generating, handles loop nodes separately
    fn do_tree(&self, node: TLabel, context: &Vec<Context<TLabel>>) -> ReSeq<TLabel> {
        if self.loop_nodes.contains(&node) {
            let mut ctx = context.clone();
            ctx.push(Context::LoopHeadedBy(node));
            ReSeq::single(Loop(self.gen_node(node, &ctx)))
        } else {
            self.gen_node(node, context)
        }
    }

    pub fn reloop(&self) -> ReSeq<TLabel> {
        self.do_tree(self.cfg.entry, &Vec::new())
    }
}
