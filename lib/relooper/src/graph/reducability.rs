use crate::graph::cfg::CfgLabel;
use crate::graph::EnrichedCfg;
use crate::Cfg;
use std::collections::{HashMap, HashSet};

#[derive(Default, Clone)]
pub struct Node {
    pub id: CfgLabel,
    pub succ: HashSet<CfgLabel>,
    pub prec: HashSet<CfgLabel>,
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

pub type SuperNodeId = usize;

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
/// This SuperGraph is used as help struct to generate reducable CFG from irreducable one.
/// Main idea of algorithm:
/// 1) SuperGraph is graph where each node is group from one or more nodes  of CFG.
/// 2) Firstly, we make SuperGraph and each node of it contain exactly one CFG node.
/// 3) Then we do two operations -- merge and split until only one SuperGraph node left.
///
/// In process of this operations some CFG nodes will be cloned, and finally all this nodes will represent
/// equivalent reducable CFG.
struct SuperGraph {
    id2node: HashMap<SuperNodeId, SuperNode>,
    clone2origin: HashMap<CfgLabel, CfgLabel>, // clone2origin[x] = y means that cfg node with id=x was clonned from cfg node with id=y;
    next_id: SuperNodeId,
    next_cfg_id: CfgLabel,
}

impl SuperGraph {
    // Generate supergraph on given cfg
    pub fn build(mut self, g: &EnrichedCfg) -> SuperGraph {
        for &cfg_node in &g.cfg.nodes() {
            let tmp = SuperNode::default();
            // self.id2node.insert(self.next_id, tmp.build(cfg_node.clone(), self.next_id));
            self.id2node
                .insert(self.next_id, tmp.build(Node::new(cfg_node), self.next_id));
            self.next_id += 1;
        }
        // TODO: make superedges;
        return self;
    }

    /// Tryes make merge or clone operation while supergraph contains at least one node
    /// Now algorithm is greedy -- while we can merge we merge, if we can't merge but can split we split
    /// I think it's not optimal way but it works.
    pub fn run(mut self) -> SuperGraph {
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

    /// Return number of supernode that contain cfg node with given id
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

    /// Checks if there is any mergeble nodes in Supergraph
    pub fn can_merge(&self) -> bool {
        for (_sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    super_precs.insert(self.in_which_supernode(*prec_id));
                }
            }
            if super_precs.len() == 1 {
                return true;
            }
        }
        return false;
    }

    /// Checks if there is any clonable nodes in supergraph
    pub fn can_clone(&self) -> bool {
        for (_sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    super_precs.insert(self.in_which_supernode(*prec_id));
                }
            }
            if super_precs.len() > 1 {
                return true;
            }
        }
        return false;
    }

    /// Returns two random mergeble nodes in format (master_id, slave_id)
    pub fn mergeble_nodes(&self) -> (SuperNodeId, SuperNodeId) {
        for (sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    super_precs.insert(self.in_which_supernode(*prec_id));
                }
            }
            if super_precs.len() == 1 {
                return (*super_precs.iter().next().unwrap(), *sid);
            }
        }

        panic!("no mergable nodes");
    }

    /// Returns clonable node with all its precessors in format (masters_ids, slave_id)
    pub fn clonable_nodes(&self) -> (HashSet<SuperNodeId>, SuperNodeId) {
        for (sid, snode) in &self.id2node {
            let mut super_precs: HashSet<SuperNodeId> = HashSet::default();
            for (_cfg_id, cfg_node) in &snode.cfg_ids2cfg_nodes {
                for prec_id in &cfg_node.prec {
                    super_precs.insert(self.in_which_supernode(*prec_id));
                }
            }
            if super_precs.len() == 1 {
                return (super_precs, *sid);
            }
        }
        panic!("no clonable nodes");
    }

    /// This method moves all cfg nodes from slave SuperNode to master SuperNode and destroys slave SuperNode
    pub fn merge(&mut self, (master, slave): (SuperNodeId, SuperNodeId)) -> () {
        println!("merge nodes slave : {}, master : {}", slave, master);
        self.make_clone((master, slave));
        self.id2node.remove(&slave);
    }

    /// This method makes next operation for all master nodes
    /// 1) make a clone of slave node and remove all inedges of cfg nodes in this copy that origin not in this clone or current master
    /// 2) move all cfg nodes from slave clone to current master and destroy the clone
    ///
    /// After all it destroys original slave node
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

    /// This function make a copy of slave supernode, then, node by node move its cfg nodes to master supernode
    /// and remove all inedges of slave node, that are not from master node.
    pub fn make_clone(&mut self, (master, slave): (SuperNodeId, SuperNodeId)) -> () {
        let mut new_cfg_ids: HashMap<CfgLabel, CfgLabel> = HashMap::default(); // old -> new
        for id in &self.id2node.get(&slave).unwrap().cfg_ids {
            new_cfg_ids.insert(*id, self.next_id);
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

    pub fn cfg(self) -> EnrichedCfg {
        let mut id2n: HashMap<CfgLabel, Node> = HashMap::default();
        for (_sid, snode) in self.id2node {
            for (cid, cnode) in snode.cfg_ids2cfg_nodes {
                id2n.insert(cid, cnode);
            }
        }
        todo!()
        // return EnrichedCfg {
        //     id2node: (id2n),
        //     merge_nodes: (HashSet::default()),
        //     loop_nodes: (HashSet::default()),
        //     if_nodes: (HashSet::default()),
        // };
    }

    /// This method copyes node (cfg node) from snode_from to snode_to (super nodes)
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
}

/// Return reducable equivalent CFG by given CFG.
pub fn reducable(enriched_cfg: &EnrichedCfg) -> EnrichedCfg {
    return SuperGraph::default().build(&enriched_cfg).run().cfg();
}

#[test]
pub fn test_reducer() -> () {
    println!("test reducer");
    let graph = Cfg::from(vec![
        (0, 1, true),
        (0, 2, false),
        (1, 3, true),
        (2, 3, false),
        (3, 4, false),
        (1, 5, false),
        (5, 6, true),
        (5, 7, false),
        (6, 8, false),
        (7, 8, false),
        (4, 9, false),
        (8, 9, true),
        (8, 5, false),
    ]);
    // let graph = Cfg::from(vec![(0, 1), (0, 2), (1, 3), (1, 4), (1, 5), (2, 6), (6, 7)]);

    let e_graph = EnrichedCfg::new(graph, 0);
    let reducable = reducable(&e_graph);

    let dot_lines: Vec<String> = vec![
        "digraph {".to_string(),
        e_graph.cfg_to_dot(),
        String::new(),
        e_graph.dom_to_dot(),
        String::new(),
        reducable.cfg_to_dot(),
        "}".to_string(),
    ];

    std::fs::write("reduced.dot", dot_lines.join("\n")).expect("fs error");
}
