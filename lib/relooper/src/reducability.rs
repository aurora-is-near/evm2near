use crate::cfg::CfgLabel;
use crate::graph::{EnrichedCfg, Node};
use std::collections::{HashMap, HashSet};

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
struct SuperGraph {
    id2node: HashMap<SuperNodeId, SuperNode>,
    clone2origin: HashMap<CfgLabel, CfgLabel>, // clone2origin[x] = y means that cfg node with id=x was clonned from cfg node with id=y;
    next_id: SuperNodeId,
    next_cfg_id: CfgLabel,
}

impl SuperGraph {
    // i think here is a mistake
    pub fn build(mut self, g: &EnrichedCfg) -> SuperGraph {
        println!("len g.id2node = {}", g.id2node.len());
        for (_id, cfg_node) in &g.id2node {
            let tmp = SuperNode::default();
            self.id2node
                .insert(self.next_id, tmp.build(cfg_node.clone(), self.next_id));
            self.next_id += 1;
        }
        // TODO: make superedges;
        return self;
    }

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
                    super_precs.insert(self.in_which_supernode(*prec_id));
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
                    super_precs.insert(self.in_which_supernode(*prec_id));
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
                    super_precs.insert(self.in_which_supernode(*prec_id));
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
                    super_precs.insert(self.in_which_supernode(*prec_id));
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

    pub fn reducable(enriched_cfg: &EnrichedCfg) -> EnrichedCfg {
        return SuperGraph::default().build(&enriched_cfg).run().cfg();
    }
}

// #[test]
// pub fn test_build() -> () {
//     for graph_no in 0..1 {
//         match graph_no {
//             0 => {
//                 let g = read_graph("1.txt");
//                 let sg = SuperGraph::default().build(&g);
//                 for i in 0..7 {
//                     assert!(sg.origin2block.contains_key(&i));
//                 }
//                 let mut cfg_ids: HashSet<CfgLabel> = HashSet::default();
//                 for i in 0..7 {
//                     let node = sg.id2node.get(&i).unwrap();
//                     assert!(node.super_id == i);
//                     for id in &node.cfg_ids {
//                         println!("{}", id);
//                         cfg_ids.insert(*id);
//                     }
//                 }
//                 for i in 0..7 {
//                     assert!(cfg_ids.contains(&i));
//                 }
//             }
//             _ => panic!("Test build for graph {} is not implemented!", graph_no),
//         }
//     }
// }
