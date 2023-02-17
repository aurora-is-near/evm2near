use crate::graph::cfg::CfgEdge::{Cond, Uncond};
use crate::graph::cfg::{Cfg, CfgEdge};
use anyhow::{ensure, format_err};
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
            None => TLabel::from_str(first_str)
                .map(Uncond)
                .map_err(|e| anyhow::Error::new(e)),
            Some(uncond_str) => {
                let cond = TLabel::from_str(first_str).map_err(|e| anyhow::Error::new(e))?;
                let uncond = TLabel::from_str(uncond_str).map_err(|e| anyhow::Error::new(e))?;
                Ok(Cond(cond, uncond))
            }
        }
    }
}

impl<E, TLabel> TryFrom<&Vec<String>> for Cfg<TLabel>
where
    E: std::error::Error + Send + Sync + 'static,
    TLabel: FromStr<Err = E> + Eq + Hash + Clone,
{
    type Error = anyhow::Error;

    fn try_from(strings: &Vec<String>) -> Result<Self, Self::Error> {
        ensure!(
            strings.len() >= 2,
            "well-formed cfg should contain entry line and at least one edge"
        );

        let entry_str = strings.first().unwrap();
        let entry = TLabel::from_str(entry_str)?;
        let mut cfg = Cfg::new(entry);

        for edge_str in &strings[1..] {
            let (from, edge) = edge_str
                .split_once(' ')
                .ok_or_else(|| format_err!("invalid label-edge format".to_string()))?;
            let from = TLabel::from_str(from)?;
            let edge = CfgEdge::from_str(edge)?;
            cfg.add_edge(from, edge);
        }

        Ok(cfg)
    }
}
