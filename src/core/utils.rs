use std::{iter::Iterator, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subsets {
    positions: Rc<Vec<usize>>,
}

impl Subsets {
    pub fn as_vec(&self) -> &Vec<usize> {
        &*self.positions
    }
}

#[derive(Debug)]
pub struct SubsetIterator {
    first:            bool,
    subset_count:     usize,
    positions:        Rc<Vec<usize>>,
    element_counters: Vec<usize>,
}

impl Iterator for SubsetIterator {
    type Item = Subsets;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut shift = !self.first;
            self.first = false;

            for i in Rc::get_mut(&mut self.positions)
                .expect("Subset must be destroyed before new iteration")
                .iter_mut()
            {
                if !shift {
                    break;
                }

                let old = *i;
                let new = if old + 1 >= self.subset_count {
                    shift = true;
                    old + 1 - self.subset_count
                } else {
                    shift = false;
                    old + 1
                };
                assert!(old != new || self.subset_count == 1);
                *i = new;
                self.element_counters[old] -= 1;
                self.element_counters[new] += 1;
            }
            if shift {
                return None;
            }

            if !self.element_counters.contains(&0) {
                break;
            }
        }
        Some(Subsets {
            positions: self.positions.clone(),
        })
    }
}

impl SubsetIterator {
    pub fn new(container_len: usize, subset_count: usize) -> Self {
        let mut element_counters = vec![0; subset_count];
        element_counters[0] = container_len;

        Self {
            first: true,
            subset_count,
            positions: Rc::new(vec![0; container_len]),
            element_counters,
        }
    }
}

#[cfg(test)]
mod subsets_test {
    use super::*;

    #[test]
    fn iterator_test() {
        let ss = SubsetIterator::new(3, 2);
        let res: Vec<Vec<usize>> = ss.map(|x| x.as_vec().clone()).collect();
        assert!(res.contains(&vec![0, 0, 1]));
        assert!(res.contains(&vec![0, 1, 0]));
        assert!(res.contains(&vec![0, 1, 1]));
        assert!(res.contains(&vec![1, 0, 0]));
        assert!(res.contains(&vec![1, 0, 1]));
        assert!(res.contains(&vec![1, 1, 0]));
        assert_eq!(res.len(), 6);
    }

    #[test]
    fn iterator_single_test() {
        let ss = SubsetIterator::new(3, 1);
        let res: Vec<Vec<usize>> = ss.map(|x| x.as_vec().clone()).collect();
        assert_eq!(res, vec![vec![0, 0, 0]]);
    }
}
