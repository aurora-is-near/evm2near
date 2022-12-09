use crate::cfg::CfgLabel;
use crate::re_graph::ReBlockType::{Block, If, Loop};
use std::collections::BTreeMap;

pub type ReGenLabel = isize;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReLabel {
    FromCfg(CfgLabel),
    Generated(ReGenLabel),
}

#[derive(Debug, Clone, Copy)]
pub enum ReBlockType {
    Block,
    Loop,
    If,
}

#[derive(Debug, Clone, Copy)]
pub struct ReBlock {
    pub(crate) block_type: ReBlockType,
    pub(crate) curr: ReLabel,
    //TODO change to branch?
    pub(crate) inner: ReLabel,
    pub(crate) next: ReLabel,
}

pub struct ReGraph(pub(crate) BTreeMap<ReLabel, ReBlock>);

impl ReBlock {
    pub fn new(typ: ReBlockType, curr: ReLabel, inner: ReLabel, next: ReLabel) -> ReBlock {
        ReBlock {
            block_type: typ,
            curr,
            inner,
            next,
        }
    }

    pub fn block(curr: ReLabel, next: ReLabel) -> ReBlock {
        ReBlock::new(Block, curr, curr, next)
    }

    pub fn looop(curr: ReLabel, next: ReLabel) -> ReBlock {
        ReBlock::new(Loop, curr, curr, next)
    }

    pub fn iff(curr: ReLabel, tru: ReLabel, fal: ReLabel) -> ReBlock {
        ReBlock::new(If, curr, tru, fal)
    }
}
