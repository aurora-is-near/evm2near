use queues::IsQueue;
use queues::Queue;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::vec::Vec;

pub type NodeId = isize;

#[derive(Default, Clone)]
struct Node {
    pub id: NodeId,
    pub succ: HashSet<NodeId>,
    pub prec: HashSet<NodeId>,
}

#[derive(Clone)]
pub struct Block {
    pub data: u32,
}

impl Block {
    pub fn new() -> Self {
        Block { data: 1 }
    }
    pub fn copy(&self) -> Block {
        return Block { data: self.data };
    }
}

// TODO: set begin = 0 as default
#[derive(Default)]
pub struct Graph {
    id2block: HashMap<NodeId, Block>,
    id2node: HashMap<NodeId, Node>,
    next_id: NodeId,
    merge_nodes: HashSet<NodeId>,
    loop_nodes: HashSet<NodeId>,
    if_nodes: HashSet<NodeId>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            id2block: HashMap::new(),
            id2node: HashMap::new(),
            next_id: 0,
            merge_nodes: HashSet::new(),
            loop_nodes: HashSet::new(),
            if_nodes: HashSet::new(),
        }
    }

    pub fn add_vertex(&mut self, block: Block) -> () {
        self.id2block.insert(self.next_id, block);
        self.id2node.insert(self.next_id, Node::default());
        self.next_id += 1;
        // println!("{}", self.next_id);
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> () {
        self.id2node.get_mut(&from).unwrap().succ.insert(to);
        self.id2node.get_mut(&to).unwrap().prec.insert(from);
    }
    // TODO cache result
    pub fn reverse_postorder(&self, begin: NodeId) -> Vec<NodeId> {
        let mut res = Vec::<NodeId>::default();
        let mut visited = HashSet::<NodeId>::default();
        self.dfs(begin, &mut res, &mut visited);
        res.reverse();
        return res;
    }

    fn dfs(
        &self,
        current_node: NodeId,
        res: &mut Vec<NodeId>,
        visited: &mut HashSet<NodeId>,
    ) -> () {
        for id in &self.id2node.get(&current_node).unwrap().succ {
            if !visited.contains(&id) {
                visited.insert(*id);
                self.dfs(*id, res, visited);
            }
        }
        res.push(current_node);
    }

    pub fn print_keys(&self) -> () {
        println!("id2block");
        for k in self.id2block.keys() {
            println!("{}", *k);
        }
    }

    pub fn domination_tree(&self, begin: NodeId) -> HashMap<NodeId, NodeId> /* map points from node id to id of its dominator */
    {
        let mut result = HashMap::<NodeId, NodeId>::new();
        let mut bfs = Queue::<NodeId>::new();
        let mut visited = HashSet::<NodeId>::new();
        let nodes = self.reverse_postorder(begin);
        for n in nodes {
            result.insert(n, begin);
        }
        bfs.add(begin).unwrap(); // should be next. upd: i dont think so
        visited.insert(begin);
        loop {
            if bfs.size() == 0 {
                break;
            }
            let cur_id = bfs.peek().unwrap();
            visited.insert(cur_id);
            bfs.remove().unwrap();
            self.update_dominators(cur_id, begin, &mut result);
            for id in &self.id2node.get(&cur_id).unwrap().succ {
                if !visited.contains(id) {
                    bfs.add(*id).unwrap();
                }
            }
        }
        return result;
    }

    fn update_dominators(
        &self,
        cur_id: NodeId,
        origin: NodeId,
        result: &mut HashMap<NodeId, NodeId>,
    ) -> () {
        let reachable = self.reverse_postorder(origin);
        let mut reachable_set = HashSet::<NodeId>::default();
        for node in reachable {
            reachable_set.insert(node);
        }
        let mut reached = Vec::<NodeId>::default();
        let mut visited = HashSet::<NodeId>::default();
        visited.insert(cur_id);
        self.dfs(origin, &mut reached, &mut visited);
        for id in reached {
            reachable_set.remove(&id);
        }
        reachable_set.remove(&cur_id);
        for id in reachable_set {
            result.insert(id, cur_id);
        }
    }

    pub fn put_labels(&mut self, begin: NodeId) -> () {
        self.put_merge_labels(begin);
        self.put_loop_labels(begin);
        self.put_if_labels(begin);
    }

    fn put_merge_labels(&mut self, begin: NodeId) {
        for (id, node) in &self.id2node {
            let mut forward_inedjes = 0;
            for origin in &node.prec {
                if self.is_forward(begin, *origin, *id) {
                    forward_inedjes += 1;
                }
            }
            if forward_inedjes >= 2 {
                self.merge_nodes.insert(*id);
            }
        }
    }
    // TODO: check if edge exist
    fn is_forward(&self, begin: NodeId, from: NodeId, to: NodeId) -> bool {
        let order = self.reverse_postorder(begin);
        for id in order {
            if id == from {
                return true;
            }
            if id == to {
                return false;
            }
        }
        return false;
    }

    fn is_backward(&self, begin: NodeId, from: NodeId, to: NodeId) -> bool {
        return !self.is_forward(begin, from, to);
    }

    fn put_loop_labels(&mut self, begin: NodeId) -> () {
        for (id, node) in &self.id2node {
            for origin in &node.prec {
                if self.is_backward(begin, *origin, *id) {
                    self.loop_nodes.insert(*id);
                    break;
                }
            }
        }
    }

    fn put_if_labels(&mut self, begin: NodeId) -> () {
        for (id, node) in &self.id2node {
            if node.succ.len() > 1 {
                self.if_nodes.insert(*id);
            }
        }
    }

    pub fn print_labels(&self) -> () {
        for (id, node) in &self.id2node {
            print!("{}: ", id);
            if self.merge_nodes.contains(id) {
                print!("MERGE, ");
            }
            if self.loop_nodes.contains(id) {
                print!("LOOP, ");
            }
            if self.if_nodes.contains(id) {
                print!("IF, ");
            }
            print!("\n");
        }
    }

    pub fn gen_dot(&self, graphname: &str) -> () {
        let mut res = format!("digraph {graphname} {{ \n");
        for i in 0..self.next_id {
            let s = format!("    N{i}[label=\"N{i}\"];\n");
            res.push_str(&s);
        }
        for (from, node) in &self.id2node {
            for to in &node.succ {
                let s = format!("    N{from} -> N{to}[label=\"\"];\n");
                res.push_str(&s);
            }
        }
        res.push_str("}\n");
        let mut file = File::create(format!("dots/{graphname}.dot")).unwrap();
        file.write_all(res.as_bytes()).unwrap();
    }

    pub fn reducable(&self) -> Graph {
        return Supergraph::default().build(&self).run().cfg();
    }
}

pub fn ReadGraph(filepath: &str) -> Graph {
    println!("{}:", filepath);
    let mut fullpath = "test/".to_owned();
    fullpath.push_str(filepath);
    let data = fs::read_to_string(fullpath).unwrap();
    let lines = data.split("\n").collect::<Vec<&str>>();
    let mut result = Graph::new();
    let size = lines[0].parse::<NodeId>().unwrap();
    for _ in 0..size {
        result.add_vertex(Block::new());
    }
    for line in lines {
        if line.contains(" ") {
            let nums = line.split(" ").collect::<Vec<&str>>();
            result.add_edge(
                nums[0].parse::<NodeId>().unwrap(),
                nums[1].parse::<NodeId>().unwrap(),
            )
        }
    }
    return result;
}

pub type SuperNodeId = isize;

#[derive(Default)]
struct SuperNode {
    super_id: SuperNodeId,
    cfg_ids: HashSet<NodeId>, // which cfg nodes in this supernode
    cfg_ids2cfg_nodes: HashMap<NodeId, Node>,
    super_prec: HashSet<SuperNodeId>,
    super_succ: HashSet<SuperNodeId>,
}

impl SuperNode {
    pub fn build(self, cfg_node: Node, id: SuperNodeId) -> SuperNode {
        self.super_id = id;
        self.cfg_ids.insert(cfg_node.id);
        self.cfg_ids2cfg_nodes.insert(cfg_node.id, cfg_node);
        return self;
    }
}

#[derive(Default)]
struct Supergraph {
    id2node: HashMap<SuperNodeId, SuperNode>,
    clone2origin: HashMap<NodeId, NodeId>, // clone2origin[x] = y means that cfg node with id=x was clonned from cfg node with id=y;
    origin2block: HashMap<NodeId, Block>,
    next_id: SuperNodeId,
}

impl Supergraph {
    pub fn build(self, g: &Graph) -> Supergraph {
        for (id, block) in g.id2block {
            self.origin2block.insert(id, block);
        }
        for (id, cfg_node) in g.id2node {
            self.id2node.insert(
                self.next_id,
                SuperNode::default().build(cfg_node, self.next_id),
            );
            self.next_id += 1;
        }

        // TODO: make superedges;

        return self;
    }

    pub fn run(self) -> Supergraph {
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

    pub fn can_merge(&self) -> bool {
        for (sid, snode) in self.id2node {
            if snode.super_prec.len() == 1 {
                return true;
            }
        }
        return false;
    }

    pub fn can_clone(&self) -> bool {
        for (sid, snode) in self.id2node {
            if snode.super_prec.len() > 1 {
                return true;
            }
        }
        return false;
    }

    // returns two random mergeble nodes in format (master_id, slave_id)
    pub fn mergeble_nodes(&self) -> (SuperNodeId, SuperNodeId) {
        for (sid, snode) in self.id2node {
            if snode.super_prec.len() == 1 {
                return (*snode.super_prec.iter().next().unwrap(), sid);
            }
        }
        panic!("no mergable nodes");
    }

    // returns clonable node with all its precessors in format (masters_ids, slave_id)
    pub fn clonable_nodes(&self) -> (HashSet<SuperNodeId>, SuperNodeId) {
        for (sid, snode) in self.id2node {
            if snode.super_prec.len() > 1 {
                return (snode.super_prec.clone(), sid);
            }
        }
        panic!("no clonable nodes");
    }

    pub fn merge(&mut self, (master, slave): (SuperNodeId, SuperNodeId)) -> () {
        self.make_clone((master, slave));
        self.id2node.remove(&slave);
    }

    pub fn split(&mut self, (masters, slave): (HashSet<SuperNodeId>, SuperNodeId)) -> () {
        for master in masters {
            self.make_clone((master, slave));
        }
        self.id2node.remove(&slave);
    }

    pub fn make_clone(&mut self, (master, slave): (SuperNodeId, SuperNodeId)) -> () {
        // TODO clone nodes with other ids.

        for cfg_id in self.id2node.get(&slave).unwrap().cfg_ids {
            self.id2node
                .get_mut(&master)
                .unwrap()
                .cfg_ids
                .insert(cfg_id);
        }
        for (cfg_id, cfg_node) in self.id2node.get(&slave).unwrap().cfg_ids2cfg_nodes {
            self.id2node
                .get_mut(&master)
                .unwrap()
                .cfg_ids2cfg_nodes
                .insert(cfg_id, cfg_node);
        }
        for succ in self.id2node.get(&slave).unwrap().super_succ {
            self.id2node
                .get_mut(&master)
                .unwrap()
                .super_succ
                .insert(succ);
        }
    }
}
