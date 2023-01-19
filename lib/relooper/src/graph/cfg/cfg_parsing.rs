use crate::graph::cfg::CfgEdge::{Cond, Terminal, Uncond};
use crate::graph::cfg::{Cfg, CfgEdge};
use anyhow::{ensure, format_err};
use std::collections::HashMap;
use std::hash::Hash;
use std::str::FromStr;

impl<E: std::error::Error + Send + Sync + 'static, TLabel: FromStr<Err = E>> FromStr
    for CfgEdge<TLabel>
{
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (first_str, maybe_second) = {
            let mut split_i = value.split(' ');
            (split_i.next().unwrap(), split_i.next())
        };

        match maybe_second {
            None => {
                let a = TLabel::from_str(first_str)
                    .map(Uncond)
                    .map_err(|e| anyhow::Error::new(e)); //TODO unable to find solution for non-std-err conversion to anyhow error
                a
            }
            Some(uncond_str) => {
                let cond = TLabel::from_str(first_str).map_err(|e| anyhow::Error::new(e))?;
                let uncond = TLabel::from_str(uncond_str).map_err(|e| anyhow::Error::new(e))?;
                Ok(Cond(cond, uncond))
            }
        }
    }
}

impl<
        E: std::error::Error + Send + Sync + 'static,
        TLabel: FromStr<Err = E> + Eq + Hash + Clone,
    > TryFrom<&Vec<String>> for Cfg<TLabel>
{
    type Error = anyhow::Error;

    fn try_from(strings: &Vec<String>) -> Result<Self, Self::Error> {
        ensure!(
            strings.len() >= 2,
            "well-formed cfg should contain entry line and at least one edge"
        );

        let entry_str = strings.first().unwrap();
        let entry = TLabel::from_str(entry_str).map_err(|e| anyhow::Error::new(e))?;

        let mut out_edges: HashMap<TLabel, CfgEdge<TLabel>> =
            HashMap::with_capacity(strings.len() - 1);

        for edge_str in &strings[1..] {
            let (from, edge) = edge_str
                .split_once(' ')
                .ok_or_else(|| format_err!("invalid label-edge format".to_string()))?;
            let from = TLabel::from_str(from).map_err(|e| anyhow::Error::new(e))?;
            let edge = CfgEdge::from_str(edge)?;
            for to in edge.to_vec() {
                if !out_edges.contains_key(to) {
                    out_edges.insert(to.clone(), Terminal);
                }
            }
            out_edges.insert(from, edge);
        }

        Ok(Self { entry, out_edges })
    }
}
