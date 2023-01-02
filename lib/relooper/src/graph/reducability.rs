use crate::graph::cfg::CfgEdge;
use crate::graph::cfg::CfgLabel;
use crate::Cfg;
use std::collections::{HashMap, HashSet};
pub type SuperNodeId = usize;

#[derive(Eq, Hash, PartialEq, Copy, Clone)]
/// (from, to)
pub enum ProperEdge {
    Uncond(CfgLabel, CfgLabel),
    Cond(CfgLabel, CfgLabel),
    Terminal,
}

#[derive(Default, Clone)]
struct Node {
    pub id: CfgLabel,
    pub succ: HashSet<ProperEdge>,
    pub prec: HashSet<ProperEdge>,
}

impl Node {
    pub fn new(id_: CfgLabel) -> Node {
        return Node {
            id: id_,
            succ: HashSet::default(),
            prec: HashSet::default(),
        };
    }
}

// TODO: set begin = 0 as default
#[derive(Default)]
pub struct Graph {
    id2node: HashMap<CfgLabel, Node>,
    next_id: CfgLabel,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            id2node: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn build_from(&mut self, cfg: &Cfg) -> &Graph {
        for (label, edge) in cfg.out_edges {
            if label >= self.next_id {
                self.next_id = label;
            }
            self.id2node.insert(label, Node::new(label));
            match edge {
                CfgEdge::Cond(to_cond, to_ucond) => {
                    self.id2node
                        .get_mut(&label)
                        .unwrap()
                        .succ
                        .insert(ProperEdge::Cond(label, to_cond));
                    self.id2node
                        .get_mut(&label)
                        .unwrap()
                        .succ
                        .insert(ProperEdge::Uncond(label, to_ucond));
                    self.id2node
                        .get_mut(&to_cond)
                        .unwrap()
                        .prec
                        .insert(ProperEdge::Cond(label, to_cond));
                    self.id2node
                        .get_mut(&to_ucond)
                        .unwrap()
                        .prec
                        .insert(ProperEdge::Uncond(label, to_ucond));
                }
                CfgEdge::Uncond(to_uncond) => {
                    self.id2node
                        .get_mut(&label)
                        .unwrap()
                        .succ
                        .insert(ProperEdge::Uncond(label, to_uncond));
                    self.id2node
                        .get_mut(&to_uncond)
                        .unwrap()
                        .prec
                        .insert(ProperEdge::Uncond(label, to_uncond));
                }
                CfgEdge::Terminal => {
                    self.id2node
                        .get_mut(&label)
                        .unwrap()
                        .succ
                        .insert(ProperEdge::Terminal);
                }
            }
        }
        return self;
    }

    pub fn cfg(&self) -> Cfg {
        let mut out_edges: HashMap<CfgLabel, CfgEdge> = HashMap::default();
        for (id, node) in self.id2node {
            if node.succ.len() == 1 {
                match &node.succ.iter().next().unwrap() {
                    ProperEdge::Uncond(from, to) => {
                        out_edges.insert(id, CfgEdge::Uncond(*to));
                    }
                    ProperEdge::Terminal => {
                        out_edges.insert(id, CfgEdge::Terminal);
                    }
                }
            } else {
                let mut cond_to: CfgLabel;
                let mut ucond_to: CfgLabel;
                for edge in node.succ {
                    match edge {
                        ProperEdge::Cond(from, to) => {
                            cond_to = to;
                        }
                        ProperEdge::Uncond(from, to) => {
                            ucond_to = to;
                        }
                        ProperEdge::Terminal => {
                            panic!("Here should not be terminal")
                        }
                    }
                }
                out_edges.insert(id, CfgEdge::Cond(cond_to, ucond_to));
            }
        }
        return Cfg { out_edges };
    }

    pub fn add_vertex(&mut self, id: CfgLabel) -> () {
        self.id2node.insert(id, Node::new(id));
        if self.next_id <= id {
            self.next_id = id + 1;
        }
    }

    pub fn add_edge(&mut self, edge: ProperEdge) -> () {
        match edge {
            ProperEdge::Uncond(from, to) => {
                self.id2node
                    .get_mut(&from)
                    .unwrap()
                    .succ
                    .insert(edge.clone());
                self.id2node.get_mut(&to).unwrap().prec.insert(edge.clone());
            }
            ProperEdge::Cond(from, to) => {
                self.id2node
                    .get_mut(&from)
                    .unwrap()
                    .succ
                    .insert(edge.clone());
                self.id2node.get_mut(&to).unwrap().prec.insert(edge.clone());
            }
            ProperEdge::Terminal => {}
        }
    }
}

pub fn reducable(g: &Cfg) -> Cfg {
    return Supergraph::default().build(&g).run().cfg();
}

#[derive(Default, Clone)]
struct SuperNode {
    super_id: SuperNodeId,
    cfg_ids: HashSet<CfgLabel>, // which cfg nodes in this supernode
    cfg_ids2cfg_nodes: HashMap<CfgLabel, Node>,
}

impl SuperNode {
    pub fn build(mut self, cfg_node: Node, id: SuperNodeId) -> SuperNode {
        self.super_id = id;
        self.cfg_ids.insert(cfg_node.id);
        self.cfg_ids2cfg_nodes.insert(cfg_node.id, cfg_node);
        return self;
    }
}

#[derive(Default)]
struct Supergraph {
    id2node: HashMap<SuperNodeId, SuperNode>,
    clone2origin: HashMap<CfgLabel, CfgLabel>, // clone2origin[x] = y means that cfg node with id=x was clonned from cfg node with id=y;
    next_id: SuperNodeId,
    next_cfg_id: CfgLabel,
}

impl Supergraph {
    // i think here is a mistake
    pub fn build(mut self, c: &Cfg) -> Supergraph {
        let g = Graph::new().build_from(c);
        println!("len g.id2node = {}", g.id2node.len());

        for (_id, cfg_node) in &g.id2node {
            let tmp = SuperNode::default();
            self.id2node
                .insert(self.next_id, tmp.build(cfg_node.clone(), self.next_id));
            self.next_id += 1;
        }
        self.next_cfg_id = g.next_id;
        // TODO: make superedges;
        return self;
    }

    pub fn run(mut self) -> Supergraph {
        loop {
            if self.can_merge() {
                self.merge(self.mergeble_nodes());
                continue;
            }
            if self.can_clone() {
                self.split(self.clonable_nodes());
                continue;
            }
            return self;
        }
    }

    pub fn in_which_supernode(&self, nid: CfgLabel) -> SuperNodeId {
        println!("in which supernode called with nid = {}", nid);
        for (id, node) in &self.id2node {
            format!("super id {}\n", id);
            for sid in &node.cfg_ids {
                print!("\tsub id {}\n", sid);
            }
            if node.cfg_ids.contains(&nid) {
                return *id;
            }
        }
        println!("BBBBBBBBBBBBBBBBBB");
        println!("len id2node = {}", &self.id2node.len());
        panic!("No such node");
    }

    pub fn can_merge(&self) -> bool {
        for (_sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    match prec_id {
                        ProperEdge::Cond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Uncond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Terminal => {}
                    }
                }
            }
            if super_precs.len() == 1 {
                return true;
            }
        }
        return false;
    }

    pub fn can_clone(&self) -> bool {
        for (_sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    match prec_id {
                        ProperEdge::Cond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Uncond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Terminal => {}
                    }
                }
            }
            if super_precs.len() > 1 {
                return true;
            }
        }
        return false;
    }

    // returns two random mergeble nodes in format (master_id, slave_id)
    pub fn mergeble_nodes(&self) -> (SuperNodeId, SuperNodeId) {
        for (sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    match prec_id {
                        ProperEdge::Cond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Uncond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Terminal => {}
                    }
                }
            }
            if super_precs.len() == 1 {
                return (*super_precs.iter().next().unwrap(), *sid);
            }
        }

        panic!("no mergable nodes");
    }

    // returns clonable node with all its precessors in format (masters_ids, slave_id)
    pub fn clonable_nodes(&self) -> (HashSet<SuperNodeId>, SuperNodeId) {
        for (sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    match prec_id {
                        ProperEdge::Cond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Uncond(from, to) => {
                            super_precs.insert(self.in_which_supernode(*from));
                        }
                        ProperEdge::Terminal => {}
                    }
                }
            }
            if super_precs.len() == 1 {
                return (super_precs, *sid);
            }
        }
        panic!("no clonable nodes");
    }

    pub fn merge(&mut self, (master, slave): (SuperNodeId, SuperNodeId)) -> () {
        println!("merge nodes slave : {}, master : {}", slave, master);
        self.make_clone((master, slave));
        self.id2node.remove(&slave);
    }

    pub fn split(&mut self, (masters, slave): (HashSet<SuperNodeId>, SuperNodeId)) -> () {
        println!("split nodes slave : {}, masters:", slave);
        for id in &masters {
            println!("{},", id)
        }
        for master in masters {
            self.make_clone((master, slave));
        }
        self.id2node.remove(&slave);
    }

    // this function make a copy of slave supernode, then, node by node move its cfg nodes to master supernode
    // and remove all inedges of slave node, that are not from master node.
    pub fn make_clone(&mut self, (master, slave): (SuperNodeId, SuperNodeId)) -> () {
        let mut new_cfg_ids: HashMap<CfgLabel, CfgLabel> = HashMap::default(); // old -> new
        for id in &self.id2node.get(&slave).unwrap().cfg_ids {
            new_cfg_ids.insert(*id, self.next_id);
            self.next_cfg_id += 1;
        }
        for (old, _new) in &new_cfg_ids {
            let mut node = self
                .id2node
                .get(&slave)
                .unwrap()
                .cfg_ids2cfg_nodes
                .get(&old)
                .unwrap()
                .clone();

            let new_id = *new_cfg_ids.get(&node.id).unwrap();
            // node.id = new_id;
            self.copy_to_other_supernode(node.clone(), slave, master, new_id);
        }
    }

    pub fn cfg(self) -> Cfg {
        let mut id2n: HashMap<CfgLabel, Node> = HashMap::default();
        for (_sid, snode) in self.id2node {
            for (cid, cnode) in snode.cfg_ids2cfg_nodes {
                id2n.insert(cid, cnode);
            }
        }

        return Graph {
            id2node: (id2n),
            next_id: (self.next_cfg_id),
        }
        .cfg();
    }

    // this method copyes node (cfg node) from snode_from to snode_to (super nodes)
    pub fn copy_to_other_supernode(
        &mut self,
        node: Node,
        snode_from_id: SuperNodeId,
        snode_to_id: SuperNodeId,
        new_id: CfgLabel,
    ) -> () {
        let snode_from = self.id2node.get(&snode_from_id).unwrap().clone();
        let mut snode_to = self.id2node.get(&snode_to_id).unwrap().clone();
        snode_to.cfg_ids.insert(new_id);
        snode_to.cfg_ids2cfg_nodes.insert(new_id, node.clone()); // cut precs of this node

        snode_to
            .cfg_ids2cfg_nodes
            .get_mut(&new_id)
            .unwrap()
            .prec
            .retain(|prec_id: &CfgLabel| -> bool {
                snode_from.cfg_ids.contains(&prec_id) || snode_to.cfg_ids.contains(&prec_id)
            });

        let is_foreighn = |prec_id: &CfgLabel| -> bool {
            !snode_from.cfg_ids.contains(&prec_id) && !snode_to.cfg_ids.contains(&prec_id)
            // maybe wrong predicate ? maybe fixed
        };

        let mut foreighn_precs: HashSet<(CfgLabel, CfgLabel)> = HashSet::default();
        for id in &snode_from.cfg_ids2cfg_nodes.get(&node.id).unwrap().prec {
            // here should be other id. Maybe i fix it ?
            if is_foreighn(id) {
                foreighn_precs.insert((*id, node.id));
            }
        }
        for (from, to) in foreighn_precs {
            let origin = self.in_which_supernode(from);
            self.id2node
                .get_mut(&origin)
                .unwrap()
                .cfg_ids2cfg_nodes
                .get_mut(&from)
                .unwrap()
                .succ
                .remove(&to);

            // ????
            snode_to
                .cfg_ids2cfg_nodes
                .get_mut(&to)
                .unwrap()
                .prec
                .remove(&from);
        }

        *self.id2node.get_mut(&snode_to_id).unwrap() = snode_to;
    }

    pub fn remove_foreighn_edges(cfg_id: CfgLabel, from_id: SuperNodeId, to_id: SuperNodeId) {}
}
