use std::collections::{HashSet, VecDeque};
use std::hash::Hash;

pub struct Bfs<T, ChFun> {
    visited: HashSet<T>,
    queue: VecDeque<T>,
    get_children: ChFun,
}

impl<'a, T, ChIt, ChFun> Bfs<&'a T, ChFun>
where
    ChIt: IntoIterator<Item = &'a T>,
    ChFun: FnMut(&T) -> ChIt,
{
    pub fn start_iter<I: Iterator<Item = &'a T>>(iter: I, get_children: ChFun) -> Self {
        Bfs {
            visited: HashSet::new(),
            queue: VecDeque::from_iter(iter),
            get_children,
        }
    }

    pub fn start_from(item: &'a T, get_children: ChFun) -> Self {
        Self::start_iter(Some(item).into_iter(), get_children)
    }

    pub fn start_from_except(item: &T, mut get_children: ChFun) -> Self {
        Self::start_iter(get_children(item).into_iter(), get_children)
    }
}

impl<'a, T, ChIt, ChFun> Iterator for Bfs<&'a T, ChFun>
where
    T: Hash + Eq,
    ChIt: IntoIterator<Item = &'a T>,
    ChFun: FnMut(&T) -> ChIt,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front().map(|curr| {
            let children = (self.get_children)(curr).into_iter().filter(|c| {
                if self.visited.contains(c) {
                    false
                } else {
                    self.visited.insert(c.to_owned());
                    true
                }
            });
            self.queue.extend(children);
            curr
        })
    }
}
