use crate::cfg::CfgLabel;
use std::collections::HashMap;

pub type ReGenLabel = usize;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReLabel {
    FromCfg(CfgLabel),
    Generated(ReGenLabel),
}

#[derive(Debug, Clone, Copy)]
pub enum ReEdge {
    Next(ReLabel),
    Uncond(usize),
    Cond(usize, Option<usize>),
    // Terminal
}

#[derive(Debug, Clone, Copy)]
pub enum ReBlockNew {
    Block(ReLabel, ReEdge),
    Loop(ReLabel, ReEdge),
    If(ReLabel, ReEdge, ReEdge),
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
    next: ReEdge,
    branch: Option<usize>,
}

pub struct ReGraph {
    pub start: ReLabel,
    pub blocks: HashMap<ReLabel, ReBlock>,
}

impl ReBlock {
    pub fn new(typ: ReBlockType, curr: ReLabel, next: ReEdge) -> ReBlock {
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
