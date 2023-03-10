use std::collections::VecDeque;

pub struct Bfs<T, ChFun> {
    queue: VecDeque<T>,
    get_children: ChFun,
}

impl<T, ChIt, ChFun> Iterator for Bfs<T, ChFun>
where
    ChIt: Iterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front().map(|curr| {
            let children = (self.get_children)(&curr);
            self.queue.extend(children);
            curr
        })
    }
}

impl<T, ChIt, ChFun> Bfs<T, ChFun>
where
    ChIt: Iterator<Item = T>,
    ChFun: FnMut(&T) -> ChIt,
{
    pub fn start_iter<I: Iterator<Item = T>>(iter: I, get_children: ChFun) -> Self {
        Bfs {
            queue: VecDeque::from_iter(iter),
            get_children,
        }
    }

    pub fn start_from(item: T, get_children: ChFun) -> Self {
        Self::start_iter(Some(item).into_iter(), get_children)
    }

    pub fn start_from_except(item: T, mut get_children: ChFun) -> Self {
        Self::start_iter(get_children(&item), get_children)
    }
}
