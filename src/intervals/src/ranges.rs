
use std::collections::VecDeque;
use std::mem;
use std::cmp;
use std::ops::Range;
use std::slice::Iter;

use rayon::prelude::*;

use num::{One, Integer, PrimInt, Zero, CheckedAdd};
use crate::bounded::Bounded;

#[derive(Debug, Clone)]
pub struct Ranges<T>(pub Vec<Range<T>>)
where T: Integer + PrimInt
    + Bounded<T>
    + Send + Sync
    + std::fmt::Debug;

impl<T> Ranges<T>
where T: Integer + PrimInt
    + Bounded<T>
    + Send + Sync
    + std::fmt::Debug {
    pub fn new(mut data: Vec<Range<T>>, min_depth: Option<i8>, make_consistent: bool) -> Ranges<T> {
        let ranges = if make_consistent {
            (&mut data).par_sort_unstable_by(|left, right| left.start.cmp(&right.start));

            MergeOverlappingRangesIter::new(data.iter(), min_depth).collect::<Vec<_>>()
        } else {
            data
        };
        Ranges(ranges)
    }

    fn merge(&self, other: &Self, op: impl Fn(bool, bool) -> bool) -> Self {
        // Unflatten a stack containing T-typed elements
        // to a stack of Range<T> types elements without
        // copying the data.
        fn unflatten<T>(input: &mut Vec<T>) -> Vec<Range<T>> {
            let mut owned_input = Vec::<T>::new();
            // We swap the content refered by input with a new
            // allocated vector.
            // This fix the problem when ``input`` is freed by reaching out
            // the end of the caller scope.
            std::mem::swap(&mut owned_input, input);

            let len = owned_input.len() >> 1;
            let cap = owned_input.capacity();
            let ptr = owned_input.as_mut_ptr() as *mut Range<T>;
            
            mem::forget(owned_input);

            let result = unsafe {
                Vec::from_raw_parts(ptr, len, cap)
            };
            
            result
        }

        // Flatten a stack containing Range<T> typed elements to a stack containing
        // the start followed by the end value of the set of ranges (i.e. a Vec<T>).
        // This does a copy of the data. This is necessary because we do not want to
        // modify ``self`` as well as ``other`` and we want to return the result of 
        // the union of the two Ranges2D.
        fn flatten<T>(input: &Vec<Range<T>>) -> Vec<T>
        where T: Integer + Clone + Copy {
            input.clone()
                 .into_iter()
                 // Convert Range<T> to Vec<T> containing
                 // the start and the end values of the range.
                 .map(|r| vec![r.start, r.end])
                 // We can call flatten on a iterator containing other
                 // iterators (or collections in our case).
                 .flatten()
                 // Collect to get back a newly created Vec<T> 
                 .collect()
        }

        let sentinel = <T>::MAXPIX + One::one();
        // Flatten the Vec<Range<u64>> to Vec<u64>.
        // This operation returns new vectors
        let mut l = flatten(&self.0);
        // Push the sentinel
        l.push(sentinel);
        let mut r = flatten(&other.0);
        // Push the sentinel
        r.push(sentinel);

        let mut i = 0;
        let mut j = 0;

        let mut result: Vec<T> = vec![];

        while i < l.len() || j < r.len() {
            let c = cmp::min(l[i], r[j]);
            // If the two ranges have been processed
            // then we break the loop
            if c == sentinel {
                break;
            }

            let on_rising_edge_t1 = (i & 0x1) == 0;
            let on_rising_edge_t2 = (j & 0x1) == 0;
            let in_l = (on_rising_edge_t1 && c == l[i]) | (!on_rising_edge_t1 && c < l[i]);
            let in_r = (on_rising_edge_t2 && c == r[j]) | (!on_rising_edge_t2 && c < r[j]);

            let closed = (result.len() & 0x1) == 0;

            let add = !(closed ^ op(in_l, in_r));
            if add {
                result.push(c);
            }

            if c == l[i] {
                i += 1;
            }
            if c == r[j] {
                j += 1;
            }
        }

        Ranges(unflatten(&mut result))
    }

    fn merge_mut(&mut self, other: &mut Self, op: impl Fn(bool, bool) -> bool) {
        // Unflatten a stack containing T-typed elements
        // to a stack of Range<T> types elements without
        // copying the data.
        fn unflatten<T>(input: &mut Vec<T>) -> Vec<Range<T>> {
            let mut owned_input = Vec::<T>::new();
            // We swap the content refered by input with a new
            // allocated vector.
            // This fix the problem when ``input`` is freed by reaching out
            // the end of the caller scope.
            std::mem::swap(&mut owned_input, input);

            let len = owned_input.len() >> 1;
            let cap = owned_input.capacity();
            let ptr = owned_input.as_mut_ptr() as *mut Range<T>;
            
            mem::forget(owned_input);

            let result = unsafe {
                Vec::from_raw_parts(ptr, len, cap)
            };
            
            result
        }

        // Flatten a stack containing Range<T> typed elements to a stack containing
        // the start followed by the end value of the set of ranges (i.e. a Vec<T>).
        // This does a copy of the data. This is necessary because we do not want to
        // modify ``self`` as well as ``other`` and we want to return the result of 
        // the union of the two Ranges2D.
        fn flatten<T>(input: &mut Vec<Range<T>>) -> Vec<T> {
            let mut owned_input = Vec::<Range<T>>::new();
            // We swap the content refered by input with a new
            // allocated vector.
            // This fix the problem when ``input`` is freed by reaching out
            // the end of the caller scope.
            std::mem::swap(&mut owned_input, input);

            let len = owned_input.len() << 1;
            let cap = owned_input.capacity();
            let ptr = owned_input.as_mut_ptr() as *mut T;
            
            mem::forget(owned_input);

            let result = unsafe {
                Vec::from_raw_parts(ptr, len, cap)
            };
            
            result
        }

        let sentinel = <T>::MAXPIX + One::one();
        // Flatten the Vec<Range<u64>> to Vec<u64>.
        // This operation returns new vectors
        let mut l = flatten(&mut self.0);
        // Push the sentinel
        l.push(sentinel);
        let mut r = flatten(&mut other.0);
        // Push the sentinel
        r.push(sentinel);

        let mut i = 0;
        let mut j = 0;

        let mut result: Vec<T> = vec![];

        while i < l.len() || j < r.len() {
            let c = cmp::min(l[i], r[j]);
            // If the two ranges have been processed
            // then we break the loop
            if c == sentinel {
                break;
            }

            let on_rising_edge_t1 = (i & 0x1) == 0;
            let on_rising_edge_t2 = (j & 0x1) == 0;
            let in_l = (on_rising_edge_t1 && c == l[i]) | (!on_rising_edge_t1 && c < l[i]);
            let in_r = (on_rising_edge_t2 && c == r[j]) | (!on_rising_edge_t2 && c < r[j]);

            let closed = (result.len() & 0x1) == 0;

            let add = !(closed ^ op(in_l, in_r));
            if add {
                result.push(c);
            }

            if c == l[i] {
                i += 1;
            }
            if c == r[j] {
                j += 1;
            }
        }

        self.0 = unflatten(&mut result);
    }

    pub fn union(&self, other: &Self) -> Self {        
        self.merge(other, |a, b| a || b)
    }

    pub fn union_mut(&mut self, other: &mut Self) {
        self.merge_mut(other, |a, b| a || b)
    }

    pub fn intersection(&self, other: &Self) -> Self {
        self.merge(other, |a, b| a && b)
    }

    pub fn difference(&self, other: &Self) -> Self {
        self.merge(other, |a, b| a && !b)
    }

    pub fn complement(&self) -> Self {
        let mut result = Vec::<Range<T>>::with_capacity((self.0.len() + 1) as usize);

        if self.is_empty() {
            result.push(Zero::zero()..<T>::MAXPIX);
        } else {
            let mut s = 0;
            let mut last = if self[0].start == Zero::zero() {
                s = 1;
                self[0].end
            } else {
                Zero::zero()
            };

            result = self.0.iter()
                .skip(s)
                .map(|range| {
                    let r = last..range.start;
                    last = range.end;
                    r
                })
                .collect::<Vec<_>>();

            if last < <T>::MAXPIX {
                result.push(last..<T>::MAXPIX);
            }
        }
        Ranges(result)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> Iter<Range<T>> {
        self.0.iter()
    }

    pub fn contains(&self, x: &Range<T>) -> bool {
        let result = self.0.par_iter()
            .map(|r| {
                if x.start >= r.end || x.end <= r.start {
                    false
                } else {
                    true
                }
            })
            .reduce_with(|a, b| a || b)
            .unwrap();
        result
    }

    pub fn degrade(&mut self, depth: i8) {
        let shift = ((<T>::MAXDEPTH - depth) << 1) as u32;

        let mut offset: T = One::one();
        offset = offset.unsigned_shl(shift) - One::one();

        let mut mask: T = One::one();
        mask = mask.checked_mul(&!offset).unwrap();

        let adda: T = Zero::zero();
        let mut addb: T = One::one();
        addb = addb.checked_mul(&offset).unwrap();

        let capacity = self.0.len();
        let mut result = Vec::<Range<T>>::with_capacity(capacity);

        for range in self.iter() {
            let a: T = range.start.checked_add(&adda).unwrap() & mask;
            let b: T = range.end.checked_add(&addb).unwrap() & mask;

            if b > a {
                result.push(a..b);
            }
        }

        self.0 = result;
    }

    pub fn depth(&self) -> i8 {
        let total: T = self.iter().fold(Zero::zero(), |res, x| {
            res | x.start | x.end
        });

        let mut depth: i8 = <T>::MAXDEPTH - (total.trailing_zeros() >> 1) as i8;

        if depth < 0 {
            depth = 0;
        }
        depth
    }
}

impl<T> PartialEq for Ranges<T>
where T: Integer + PrimInt
    + Bounded<T>
    + Send + Sync
    + std::fmt::Debug {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Ranges<T>
where T: Integer + PrimInt
    + Bounded<T>
    + Send + Sync
    + std::fmt::Debug {}

use std::ops::Index;
impl<T> Index<usize> for Ranges<T>
where T: Integer + PrimInt
    + Bounded<T>
    + Send + Sync
    + std::fmt::Debug {
    type Output = Range<T>;

    fn index(&self, index: usize) -> &Range<T> {
        &self.0[index]
    }
}

use std::iter::FromIterator;
impl<T> FromIterator<Range<T>> for Ranges<T>
where T: Integer + PrimInt
    + Bounded<T>
    + Send + Sync
    + std::fmt::Debug {
    fn from_iter<I: IntoIterator<Item = Range<T>>>(iter: I) -> Self {
        let mut ranges = Ranges(Vec::<Range<T>>::new());

        for range in iter {
            ranges.0.push(range);
        }

        ranges
    }
}

#[derive(Debug)]
pub struct MergeOverlappingRangesIter<'a, T>
where T: Integer + Clone + Copy {
    last: Option<Range<T>>,
    ranges: Iter<'a, Range<T>>,
    split_ranges: VecDeque<Range<T>>,
    min_depth: Option<i8>,
}

impl<'a, T> MergeOverlappingRangesIter<'a, T> 
where T: Integer + PrimInt + Clone + Copy {
    fn new(mut ranges: Iter<'a, Range<T>>, min_depth: Option<i8>) -> MergeOverlappingRangesIter<'a, T> {
        let last = ranges.next().cloned();
        let split_ranges = VecDeque::<Range<T>>::new();
	    MergeOverlappingRangesIter {
            last,
            ranges,
	        split_ranges,
	        min_depth,
        }
    }

    fn split_range(&self, range: Range<T>) -> VecDeque<Range<T>> {
    	let mut ranges = VecDeque::<Range<T>>::new();
	    match self.min_depth {
            None => { ranges.push_back(range); },
            Some(ref val) => {
                let shift = 2 * (29 - val) as u32;

                let mut mask: T = One::one();
                mask = mask.unsigned_shl(shift) - One::one();

                if range.end - range.start < mask {
                    ranges.push_back(range);
                } else {
                    let offset = range.start & mask;
                    let mut s = range.start;
                    if offset > Zero::zero() {
                    s = (range.start - offset) + mask + One::one();
                        ranges.push_back(range.start..s);
                    }

                    while s + mask + One::one() < range.end {
                        let next = s + mask + One::one();
                        ranges.push_back(s..next);
                        s = next;
                    }

                    ranges.push_back(s..range.end);
                }
            }
	    }
	    ranges
    }

    /*fn merge(ranges: &mut [Range<T>], idx: usize) {
        if ranges.len() > 1 {
            let m_index = (v.len() >> 1) as usize;
            let mut (l_ranges, r_ranges) = v.split_at_mut(m_index);
            rayon::join(|| merge(l_ranges),
                        || merge(r_ranges));

            // Ranges are supposed to be sorted here
            let l_index = (l_ranges.len() - 1) as usize;
            let r_index = 0 as usize;

            if l_ranges[l_index].end > r_ranges[r_index].start {
                r_ranges[r_index].start = l_ranges[l_index].start;

                ranges.swap();
            }
        }
    }*/
}

impl<'a, T> Iterator for MergeOverlappingRangesIter<'a, T> 
where T: Integer + PrimInt + Clone + Copy {
    type Item = Range<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.split_ranges.is_empty() {
            return self.split_ranges.pop_front();
        } 

        while let Some(curr) = self.ranges.next() {
            let prev = self.last.as_mut().unwrap();
            if curr.start <= prev.end {
                prev.end = cmp::max(curr.end, prev.end);
            } else {
		        let range = self.last.clone();
                self.last = Some(curr.clone());

                self.split_ranges = self.split_range(range.unwrap());
                return self.split_ranges.pop_front();
            }
        }

        if self.last.is_some() {
            let range = self.last.clone();
            self.last = None;

            self.split_ranges = self.split_range(range.unwrap());
            return self.split_ranges.pop_front();
        } else {
            None
        }
    }
}

pub struct NestedToUniqIter<T>
where T: Integer + PrimInt + CheckedAdd
    + Bounded<T>
    + Sync + Send
    + std::fmt::Debug {
    ranges: Ranges<T>,
    id: usize,
    buffer: Vec<Range<T>>,
    depth: i8,
    shift: u32,
    off: T,
    depth_off: T,
}

impl<T> NestedToUniqIter<T>
where T: Integer + PrimInt + CheckedAdd 
        + Bounded<T>
        + Sync + Send
        + std::fmt::Debug {
    pub fn new(ranges: Ranges<T>) -> NestedToUniqIter<T> {
        let id = 0;
        let buffer = Vec::<Range<T>>::new();
        let depth = 0;
        let shift = ((T::MAXDEPTH - depth) << 1) as u32;

        let mut off: T = One::one();
        off = off.unsigned_shl(shift) - One::one();

        let mut depth_off: T = One::one();
        depth_off = depth_off.unsigned_shl((2 * depth + 2) as u32);

        NestedToUniqIter {
            ranges,
            id,
            buffer,

            depth,
            shift,
            off,
            depth_off,
        }
    }
}

impl<T> Iterator for NestedToUniqIter<T>
where T: Integer + PrimInt + CheckedAdd
        + Bounded<T>
        + Send + Sync
        + std::fmt::Debug {
    type Item = Range<T>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.ranges.is_empty() {
            let start_id = self.id;
            let end_id = self.ranges.0.len();
            for i in start_id..end_id {
                let range = &self.ranges[i];
                let t1 = range.start + self.off;
                let t2 = range.end;

                let pix1 = t1.unsigned_shr(self.shift);
                let pix2 = t2.unsigned_shr(self.shift);

                let c1 = pix1.unsigned_shl(self.shift);
                let c2 = pix2.unsigned_shl(self.shift);

                self.id += 1;

                if c2 > c1 {
                    self.buffer.push(c1..c2);

                    let e1 = self.depth_off.checked_add(&pix1).unwrap();
                    let e2 = self.depth_off.checked_add(&pix2).unwrap();

                    return Some(e1..e2);
                }
            }
            
            self.ranges = self.ranges.difference(
                &Ranges::<T>::new(
                    self.buffer.clone(),
                    None,
                    true
                )
            );
            self.id = 0;
            self.buffer.clear();

            self.depth += 1;
            assert!(self.depth <= <T>::MAXDEPTH ||
                   (self.depth > <T>::MAXDEPTH && self.ranges.is_empty()));
            if self.depth > <T>::MAXDEPTH && self.ranges.is_empty() {
                break;
            }

            // Recompute the constants for the new depth
            self.shift = ((T::MAXDEPTH - self.depth) << 1) as u32;
            self.off = One::one();
            self.off = self.off.unsigned_shl(self.shift) - One::one();

            self.depth_off = One::one();
            self.depth_off = self.depth_off.unsigned_shl((2 * self.depth + 2) as u32);
        }
        None 
    }
}

pub struct DepthPixIter<T>
where T: Integer + PrimInt + CheckedAdd
    + Bounded<T>
    + Sync + Send
    + std::fmt::Debug {
    ranges: Ranges<T>,
    current: Option<Range<T>>,

    last: Option<T>,
    depth: i8,
    shift: u32,

    offset: T,
    depth_offset: T,
}

impl<T> DepthPixIter<T>
where T: Integer + PrimInt + CheckedAdd
    + Bounded<T>
    + Sync + Send
    + std::fmt::Debug {
    pub fn new(ranges: Ranges<T>) -> DepthPixIter<T> {
        let depth = 0;
        let shift = ((T::MAXDEPTH - depth) << 1) as u32;

        let mut offset: T = One::one();
        offset = offset.unsigned_shl(shift) - One::one();

        let mut depth_offset: T = One::one();
        depth_offset = depth_offset.unsigned_shl((2 * depth + 2) as u32);

        let current = None;
        let last = None;
        DepthPixIter {
            ranges,
            current,
            last,
            depth,
            shift,
            offset,
            depth_offset,
        }
    }

    fn next_item_range(&mut self) -> Option<(i8, T)> {
        if let Some(current) = self.current.clone() {
            let last = self.last.unwrap();
            if last < current.end {
                let (depth, pix) = <T>::pix_depth(last);
                self.last = last
                    .checked_add(&One::one());

                Some((depth as i8, pix))
            } else {
                self.current = None;
                self.last = None;
                None
            }
        } else {
            None
        }
    }
}

impl<T> Iterator for DepthPixIter<T>
where T: Integer + PrimInt + CheckedAdd
    + Bounded<T>
    + Send + Sync
    + std::fmt::Debug {
    type Item = (i8, T);

    fn next(&mut self) -> Option<Self::Item> {
        let next_depth_pix = self.next_item_range();
        if next_depth_pix.is_some() {
            next_depth_pix
        } else {
            while !self.ranges.is_empty() {
                for range in self.ranges.iter() {
                    let t1 = range.start + self.offset;
                    let t2 = range.end;

                    let pix1 = t1.unsigned_shr(self.shift);
                    let pix2 = t2.unsigned_shr(self.shift);

                    let c1 = pix1.unsigned_shl(self.shift);
                    let c2 = pix2.unsigned_shl(self.shift);

                    if c2 > c1 {
                        self.ranges = self.ranges.difference(
                            &Ranges::<T>::new(vec![c1..c2], None, false)
                        );

                        let e1 = self.depth_offset
                            .checked_add(&pix1)
                            .unwrap();
                        let e2 = self.depth_offset
                            .checked_add(&pix2)
                            .unwrap();
                        
                        self.last = Some(e1);
                        self.current = Some(e1..e2);

                        return self.next_item_range();
                    }
                }
                self.depth += 1;

                // Recompute the constants for the new depth
                self.shift = ((T::MAXDEPTH - self.depth) << 1) as u32;
                self.offset = One::one();
                self.offset = self.offset.unsigned_shl(self.shift) - One::one();
                
                self.depth_offset = One::one();
                self.depth_offset = self.depth_offset.unsigned_shl((2 * self.depth + 2) as u32);
            }
            None
        }
    }
}

// Iterator responsible for converting
// ranges of uniq numbers to ranges of
// nested numbers
pub struct UniqToNestedIter<T>
where T: Integer + PrimInt + CheckedAdd
    + Bounded<T>
    + Sync + Send
    + std::fmt::Debug {
    ranges: Ranges<T>,
    cur: T,
    id: usize,
}

impl<T> UniqToNestedIter<T>
where T: Integer + PrimInt + CheckedAdd
    + Bounded<T>
    + Sync + Send
    + std::fmt::Debug {
    pub fn new(ranges: Ranges<T>) -> UniqToNestedIter<T> {
        let id = 0;

        let cur = if ranges.0.len() > 0 {
            ranges[id].start
        } else {
            Zero::zero()
        };
        UniqToNestedIter {
            ranges,
            cur,
            id,
        }
    }
}

impl<T> Iterator for UniqToNestedIter<T>
where T: Integer + PrimInt + CheckedAdd
    + Bounded<T>
    + Sync + Send
    + std::fmt::Debug {
    type Item = Range<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // Iteration through the ranges
        while self.id < self.ranges.0.len() {
            // We get the depth/ipix values of the 
            // current uniq number
            let (depth, ipix) = T::pix_depth(self.cur);

            // We compute the number of bit to shift
            let shift = (T::MAXDEPTH as u32 - depth) << 1;

            let one: T = One::one();
            // Compute the final nested range
            // for the depth given
            let e1 = ipix
                .unsigned_shl(shift);
            let e2 = ipix
                .checked_add(&one)
                .unwrap()
                .unsigned_shl(shift);

            self.cur = self.cur
                .checked_add(&one)
                .unwrap();

            let end = self.ranges[self.id].end;
            if self.cur == end {
                self.id += 1;

                if self.id < self.ranges.0.len() {
                    self.cur = self.ranges[self.id].start;
                }
            }

            return Some(e1..e2)
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::bounded::Bounded;
    use crate::ranges::Ranges;

    use num::PrimInt;
    use std::ops::Range;
    use rand::Rng;

    #[test]
    fn merge_range() {
        fn assert_merge(a: Vec<Range<u64>>, expected: Vec<Range<u64>>) {
            let ranges = Ranges::<u64>::new(a, None, true);
            let expected_ranges = Ranges::<u64>::new(expected, None, true);

            assert_eq!(ranges, expected_ranges);
        }

        assert_merge(vec![12..17, 3..5, 5..7, 6..8], vec![3..8, 12..17]);
        assert_merge(vec![0..1, 2..5], vec![0..1, 2..5]);
        assert_merge(vec![], vec![]);
        assert_merge(vec![0..6, 7..9, 8..13], vec![0..6, 7..13]);
    }

    #[test]
    fn merge_range_min_depth() {
        let ranges = Ranges::<u64>::new(vec![0..(1<<58)], Some(1), true);
        let expected_ranges = vec![0..(1<<56), (1<<56)..(1<<57), (1<<57)..3*(1<<56), 3*(1<<56)..(1<<58)];

        assert_eq!(ranges.0, expected_ranges);
    }

    #[test]
    fn test_union() {
        fn assert_union(a: Vec<Range<u64>>, b: Vec<Range<u64>>, expected: Vec<Range<u64>>) {
            let a = Ranges::<u64>::new(a, None, true);
            let b = Ranges::<u64>::new(b, None, true);
            
            let expected_ranges = Ranges::<u64>::new(expected, None, true);
            let ranges = a.union(&b);
            assert_eq!(ranges, expected_ranges);
        }

        assert_union(vec![12..17, 3..5, 5..7, 6..8], vec![0..1, 2..5], vec![0..1, 2..8, 12..17]);
        assert_union(vec![12..17, 3..5, 5..7, 6..8], vec![12..17, 3..5, 5..7, 6..8], vec![3..8, 12..17]);
        assert_union(vec![], vec![], vec![]);
        assert_union(vec![12..17], vec![], vec![12..17]);
        assert_union(vec![0..1, 2..3, 4..5], vec![1..22], vec![0..22]);
        assert_union(vec![0..10], vec![15..22], vec![0..10, 15..22]);
    }
    
    #[test]
    fn test_intersection() {
        fn assert_intersection(a: Vec<Range<u64>>, b: Vec<Range<u64>>, expected: Vec<Range<u64>>) {
            let a = Ranges::<u64>::new(a, None, true);
            let b = Ranges::<u64>::new(b, None, true);
            
            let expected_ranges = Ranges::<u64>::new(expected, None, true);
            let ranges = a.intersection(&b);
            assert_eq!(ranges, expected_ranges);
        }

        assert_intersection(vec![12..17, 3..5, 5..7, 6..8], vec![0..1, 2..5], vec![3..5]);
        assert_intersection(vec![], vec![0..1, 2..5], vec![]);
        assert_intersection(vec![], vec![], vec![]);
        assert_intersection(vec![2..6], vec![0..3, 4..8], vec![2..3, 4..6]);
        assert_intersection(vec![2..6], vec![2..6, 7..8], vec![2..6]);
        assert_intersection(vec![10..11], vec![10..11], vec![10..11]);
    }

    #[test]
    fn test_difference() {
        fn assert_difference(a: Vec<Range<u64>>, b: Vec<Range<u64>>, expected: Vec<Range<u64>>) {
            let a = Ranges::<u64>::new(a, None, true);
            let b = Ranges::<u64>::new(b, None, true);
            
            let expected_ranges = Ranges::<u64>::new(expected, None, true);
            let ranges = a.difference(&b);
            assert_eq!(ranges, expected_ranges);
        }

        assert_difference(vec![0..20], vec![5..7], vec![0..5, 7..20]);
        assert_difference(vec![0..20], vec![0..20], vec![]);
        assert_difference(vec![0..20], vec![], vec![0..20]);
        assert_difference(vec![0..20], vec![19..22], vec![0..19]);
        assert_difference(vec![0..20], vec![25..27], vec![0..20]);
        assert_difference(vec![0..20], vec![1..2, 3..4, 5..6], vec![0..1, 2..3, 4..5, 6..20]);
    }

    #[test]
    fn test_complement() {
        fn assert_complement(input: Vec<Range<u64>>, expected: Vec<Range<u64>>) {
            let ranges = Ranges::<u64>::new(input, None, true);
            let expected_ranges = Ranges::<u64>::new(expected, None, true);

            let result = ranges.complement();
            assert_eq!(result, expected_ranges);
        }

        fn assert_complement_pow_2(input: Vec<Range<u64>>) {
            let ranges = Ranges::<u64>::new(input.clone(), None, true);
            let start_ranges = Ranges::<u64>::new(input, None, true);

            let result = ranges.complement();
            let result = result.complement();

            assert_eq!(result, start_ranges);
        }

        assert_complement(vec![5..10], vec![0..5, 10..u64::MAXPIX]);
        assert_complement_pow_2(vec![5..10]);

        assert_complement(vec![0..10], vec![10..u64::MAXPIX]);
        assert_complement_pow_2(vec![0..10]);

        assert_complement(vec![0..1, 2..3, 4..5, 6..u64::MAXPIX], vec![1..2, 3..4, 5..6]);
        assert_complement_pow_2(vec![0..1, 2..3, 4..5, 6..u64::MAXPIX]);

        assert_complement(vec![], vec![0..u64::MAXPIX]);
        assert_complement_pow_2(vec![]);
    }
    
    #[test]
    fn test_depth() {
        let r1 = Ranges::<u64>::new(vec![0..4*4.pow(29 - 1)], None, true);
        assert_eq!(r1.depth(), 0);

        let r2 = Ranges::<u64>::new(vec![0..4*4.pow(29 - 3)], None, true);
        assert_eq!(r2.depth(), 2);

        let r3 = Ranges::<u64>::new(vec![0..3*4.pow(29 - 3)], None, true);
        assert_eq!(r3.depth(), 3);

        let r4 = Ranges::<u64>::new(vec![0..12*4.pow(29)], None, true);
        assert_eq!(r4.depth(), 0);
    }

    #[test]
    fn test_degrade() {
        let mut r1 = Ranges::<u64>::new(vec![0..4*4.pow(29 - 1)], None, true);
        r1.degrade(0);
        assert_eq!(r1.depth(), 0);

        let mut r2 = Ranges::<u64>::new(vec![0..4*4.pow(29 - 3)], None, true);
        r2.degrade(1);
        assert_eq!(r2.depth(), 1);

        let mut r3 = Ranges::<u64>::new(vec![0..3*4.pow(29 - 3)], None, true);
        r3.degrade(1);
        assert_eq!(r3.depth(), 1);

        let mut r4 = Ranges::<u64>::new(vec![0..12*4.pow(29)], None, true);
        r4.degrade(0);
        assert_eq!(r4.depth(), 0);

        let mut r5 = Ranges::<u64>::new(vec![0..4*4.pow(29 - 3)], None, true);
        r5.degrade(5);
        assert_eq!(r5.depth(), 2);
    }
    
    #[test]
    fn test_uniq_decompose() {
        macro_rules! uniq_to_pix_depth {
            ($t:ty, $size:expr) => {
                let mut rng = rand::thread_rng();

                (0..$size).for_each(|_| {
                    let depth = rng.gen_range(0, <$t>::MAXDEPTH) as u32;

                    let npix = 12 * 4.pow(depth);
                    let pix = rng.gen_range(0, npix);

                    let uniq = 4*4.pow(depth) + pix;
                    assert_eq!(<$t>::pix_depth(uniq), (depth, pix));
                });
            };
        }

        uniq_to_pix_depth!(u128, 10000);
        uniq_to_pix_depth!(u64, 10000);
        uniq_to_pix_depth!(u32, 10000);
        uniq_to_pix_depth!(u8, 10000);
    }
    
    use test::Bencher;

    #[bench]
    fn bench_uniq_to_depth_pix(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let n = test::black_box(100000);    

        let uniq: Vec<u64> = (0..n).map(|_| {
            let depth = rng.gen_range(0, 30);

            let npix = 12 * 4.pow(depth);
            let pix = rng.gen_range(0, npix);

            let u = 4 * 4.pow(depth) + pix;
            u
        }).collect();

        b.iter(|| {
            uniq.iter().fold(0, |a, b| a + (u64::pix_depth(*b).0 as u64))
        });
    }
}

