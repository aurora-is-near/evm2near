use std::borrow::{Borrow, BorrowMut};
use std::collections::{HashSet, VecDeque};
use std::hash::Hash;

pub trait Traverse : Default {
    type Item;
    // type TraverseIterator: Iterator;
    fn collect(&mut self, iter: impl Iterator<Item = Self::Item>);
    fn next(&mut self) -> Option<Self::Item>;

    fn traverse<ChFun, ChIt>(mut self, children: ChFun) -> Vec<Self::Item>
        where
            Self::Item: Copy,
            ChIt: Iterator<Item = Self::Item>,
            ChFun: Fn(Self::Item) -> ChIt,
    {
        let mut res = Vec::new();

        while let Some(curr) = self.next() {
            let c = children(curr);
            res.push(curr);
            self.collect(c);
        }

        res
    }

    fn entries(items: impl Iterator<Item = Self::Item>) -> Self {
        let mut traverse = Self::default();
        traverse.collect(items);
        traverse
    }

    fn entry(item: Self::Item) -> Self {
        Self::entries(Some(item).into_iter())
    }

    // fn entry_exclude(item: Self::Item) -> Self {
    //     Self::entries()
    // }
}

pub struct BfsTree<T> {
    queue: VecDeque<T>,
}

impl<T> Default for BfsTree<T> {

    fn default() -> Self {
        BfsTree {
            queue: Default::default(),
        }
    }
}

impl<T> Traverse for BfsTree<T> {
    type Item = T;

    fn collect(&mut self, iter: impl Iterator<Item = Self::Item>) {
        for x in iter {
            self.queue.push_back(x);
        }
    }

    fn next(&mut self) -> Option<T> {
        self.queue.pop_front()
    }
}

pub struct BfsGraph<T> {
    queue: VecDeque<T>,
    visited: HashSet<T>,
}

impl<T> Default for BfsGraph<T> {
    fn default() -> Self {
        BfsGraph {
            queue: Default::default(),
            visited: Default::default(),
        }
    }
}

impl<T: Eq + Hash + Copy> Traverse for BfsGraph<T> {
    type Item = T;

    fn collect(&mut self, iter: impl Iterator<Item = Self::Item>) {
        for x in iter {
            if !self.visited.contains(&x) {
                self.visited.insert(x);
                self.queue.push_back(x);
            }
        }
    }

    fn next(&mut self) -> Option<T> {
        self.queue.pop_front()
    }
}

pub struct DfsTree<T> {
    stack: VecDeque<VecDeque<T>>,
    current: Option<VecDeque<T>>,
}

impl<T> Default for DfsTree<T> {
    fn default() -> Self {
        DfsTree {
            stack: Default::default(),
            current: Default::default(),
        }
    }
}

impl<T> Traverse for DfsTree<T> {
    type Item = T;

    fn collect(&mut self, iter: impl Iterator<Item = Self::Item>) {
        let new_items: VecDeque<T> = iter.collect();
        if !new_items.is_empty() {
            let old = self.current.replace(new_items);
            if let Some(old_curr) = old {
                if !old_curr.is_empty() {
                    self.stack.push_back(old_curr)
                }
            }
        }
    }

    fn next(&mut self) -> Option<Self::Item> {
        // self.current
        //     .take()
        //     .and_then(|q| if q.is_empty() { None } else { Some(q) })
        //     .as_mut()
        //     .or_else(|| {
        //         self.current = self.stack.pop_back();
        //         self.current.as_mut()
        //     })
        //     .map(|q| q.pop_front().expect("should contain value"))

        if let Some(e) = self.current.as_ref() {
            if e.is_empty() {
                self.current = None
            }
        }
        if self.current.is_none() {
            self.current = self.stack.pop_back();
        }
        self.current.as_mut().map(|q| q.pop_front().expect("should contain value"))
    }
}

pub struct DfsGraph<T> {
    dfs_tree: DfsTree<T>,
    visited: HashSet<T>,
}

impl<T> Default for DfsGraph<T> {
    fn default() -> Self {
        DfsGraph {
            dfs_tree: Default::default(),
            visited: Default::default(),
        }
    }
}

impl<T: Eq + Hash + Copy> Traverse for DfsGraph<T> {
    type Item = T;

    fn collect(&mut self, iter: impl Iterator<Item = Self::Item>) {
        let items = iter.filter(|item| {
            if self.visited.contains(item) {
                false
            } else {
                self.visited.insert(*item);
                true
            }
        });
        self.dfs_tree.collect(items.into_iter());
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.dfs_tree.next()
    }
}

// impl<T> Iterator for Bfs<T> {
//     type Item = T;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         self.queue.pop_front()
//     }
// }

// struct TraverseIterator<T, Trav> where Trav: TraversePolicy<Item=T> {
//     policy: Trav,
// }
