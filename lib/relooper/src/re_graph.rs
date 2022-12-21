use crate::cfg::CfgLabel;

pub struct ReSeq(pub Vec<ReBlock>);

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

    fn concat(mut self, other: ReSeq) {
        self.0.extend(other.0);
    }
}
