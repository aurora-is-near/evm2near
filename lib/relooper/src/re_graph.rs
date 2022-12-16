use crate::cfg::CfgLabel;
use crate::re_graph::ReBlockType::{Block, If, Loop};
use std::collections::BTreeMap;

pub type ReGenLabel = usize;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReLabel {
    FromCfg(CfgLabel),
    Generated(ReGenLabel),
}

#[derive(Debug, Clone, Copy)]
pub enum ReBlockType {
    Block,
    Loop,
    If(ReLabel),
}

#[derive(Debug, Clone, Copy)]
pub struct ReBlock {
    block_type: ReBlockType,
    curr: ReLabel,
    next: ReLabel,
    branch: Option<usize>,
}

pub struct ReGraph(pub(crate) BTreeMap<ReLabel, ReBlock>);

impl ReBlock {
    pub fn new(typ: ReBlockType, curr: ReLabel, next: ReLabel) -> ReBlock {
        ReBlock {
            block_type: typ,
            curr,
            next,
            branch: None,
        }
    }

    pub fn label(&self) -> ReLabel {
        self.curr
    }
}
