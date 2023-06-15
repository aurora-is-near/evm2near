use crate::graph::cfg::CfgLabel;
use std::collections::HashMap;

pub struct ELUForest<TLabel: CfgLabel, Operation: TwoArgLambdaTrait<TLabel>> {
    parent: HashMap<TLabel, TLabel>,
    operation: Operation,
}

pub trait TwoArgLambdaTrait<TLabel> {
    fn apply(&self, arg1: TLabel, arg2: TLabel) -> TLabel;
}

impl<TLabel: CfgLabel, Operation: TwoArgLambdaTrait<TLabel>> ELUForest<TLabel, Operation> {
    /// returns elu forest where each node represents a separate tree
    pub fn new(nodes: Vec<TLabel>, operation: Operation) -> ELUForest<TLabel, Operation> {
        todo!()
    }

    pub fn link(&self, parent: TLabel, child: TLabel) -> () {
        todo!()
    }

    pub fn eval(&self, node: TLabel) -> TLabel {
        todo!()
    }
}
