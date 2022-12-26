use crate::graph::cfg::{CfgEdge, CfgLabel};
use crate::graph::EnrichedCfg;

impl EnrichedCfg {
    fn labels(&self, n: CfgLabel) -> String {
        let mut res = "".to_string();
        if self.loop_nodes.contains(&n) {
            res += "l";
        }
        if self.if_nodes.contains(&n) {
            res += "i";
        }
        if self.merge_nodes.contains(&n) {
            res += "m";
        }

        res
    }

    pub fn to_dot(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push("digraph res {".to_string());
        lines.push("subgraph cluster_cfg { label=\"cfg\";".to_string());
        lines.push("nstart[label=\"start\"]".to_string());
        lines.push("nend[label=\"end\"]".to_string());

        let mut edges: Vec<String> = Vec::new();
        for n in self.cfg.nodes() {
            lines.push(format!("n{n}[label=\"{n} {}\"];", self.labels(n)));
            match self.cfg.edge(n) {
                CfgEdge::Uncond(u) => {
                    edges.push(format!("n{n} -> n{u};"));
                }
                CfgEdge::Cond(t, f) => {
                    edges.push(format!("n{n} -> n{t}[style=\"dashed\"];"));
                    edges.push(format!("n{n} -> n{f};"));
                }
                CfgEdge::Terminal => {
                    edges.push(format!("n{n} -> nend;"));
                }
            }
        }
        lines.push(format!("nstart -> n{}", self.entry));
        lines.extend(edges);
        lines.push("}".to_string());
        lines.push(String::new());

        lines.push("subgraph cluster_dom { label=\"dom\"; edge [dir=\"back\"];".to_string());
        for n in self.cfg.nodes() {
            lines.push(format!("d{n}[label=\"{n}\"];"));
        }
        for (&n, &d) in &self.domination.dominated {
            lines.push(format!("d{d} -> d{n};"));
        }
        lines.push("}".to_string());
        lines.push(String::new());

        lines.push("}".to_string());
        lines.join("\n")
    }
}
