use std::collections::{HashSet, VecDeque};
use std::hash::Hash;

pub struct Cycles<'a, T, ChFun> {
    queue: VecDeque<&'a T>,
    path: VecDeque<&'a T>,
    visited: HashSet<&'a T>,
    path_set: HashSet<&'a T>,
    get_children: ChFun,
}

impl<'a, T, ChIt, ChFun> Cycles<'a, T, ChFun>
where
    ChIt: Iterator<Item = &'a T>,
    ChFun: FnMut(&T) -> ChIt,
    T: Eq + Hash,
{
    pub fn start_from(root: &'a T, get_children: ChFun) -> Self {
        Cycles {
            queue: VecDeque::from([root]),
            path: VecDeque::new(),
            visited: HashSet::new(),
            path_set: HashSet::new(),
            get_children,
        }
    }
}

impl<'a, T, ChIt, ChFun> Iterator for Cycles<'a, T, ChFun>
where
    ChIt: Iterator<Item = &'a T>,
    ChFun: FnMut(&T) -> ChIt,
    T: Eq + Hash,
{
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_back().map(|current| {
            self.visited.insert(current);
            self.path_set.insert(current);
            self.path.push_back(current);
            let children = (self.get_children)(current);
            self.queue.extend(children); //todo filter?
            todo!()
        })
    }
}
