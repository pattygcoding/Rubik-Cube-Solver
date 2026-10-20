//! Ranking permutations and 4-subsets, and permutation parity.
//!
//! The phase coordinates are packed into single integers with these, which is
//! what lets the move and pruning tables be flat arrays.

pub(crate) fn perm_parity(p: &[u8]) -> u32 {
    let mut visited = vec![false; p.len()];
    let mut parity = 0;
    for i in 0..p.len() {
        if visited[i] {
            continue;
        }
        let mut len = 0;
        let mut j = i;
        while !visited[j] {
            visited[j] = true;
            j = p[j] as usize;
            len += 1;
        }
        if len % 2 == 0 {
            parity ^= 1;
        }
    }
    parity
}

/// Lexicographic (Lehmer code) index of a permutation, in `0..n!`.
pub(crate) fn perm_index(p: &[u8]) -> u32 {
    let n = p.len();
    let mut factorials = vec![1u32; n];
    for i in 1..n {
        factorials[i] = factorials[i - 1] * i as u32;
    }
    let mut index = 0u32;
    for i in 0..n {
        let mut smaller = 0u32;
        for j in (i + 1)..n {
            if p[j] < p[i] {
                smaller += 1;
            }
        }
        index += smaller * factorials[n - 1 - i];
    }
    index
}

/// Inverse of [`perm_index`].
pub(crate) fn perm_unrank(index: u32, n: usize) -> Vec<u8> {
    let mut factorials = vec![1u32; n];
    for i in 1..n {
        factorials[i] = factorials[i - 1] * i as u32;
    }
    let mut available: Vec<u8> = (0..n as u8).collect();
    let mut out = Vec::with_capacity(n);
    let mut rest = index;
    for i in 0..n {
        let f = factorials[n - 1 - i];
        let pick = (rest / f) as usize;
        rest %= f;
        out.push(available.remove(pick));
    }
    out
}

/// Colex index of a sorted 4-subset of `0..12`, in `0..495`.
pub(crate) fn subset_index(slots: &[usize; 4]) -> usize {
    binomial(slots[0], 1) + binomial(slots[1], 2) + binomial(slots[2], 3) + binomial(slots[3], 4)
}

/// Inverse of [`subset_index`]: the sorted `index`-th 4-subset of `0..12`.
pub(crate) fn subset_unrank(index: usize) -> [usize; 4] {
    let mut rest = index;
    let mut out = [0usize; 4];
    for k in (1..=4).rev() {
        let mut n = k - 1;
        while binomial(n + 1, k) <= rest {
            n += 1;
        }
        out[k - 1] = n;
        rest -= binomial(n, k);
    }
    out
}

/// `n` choose `k`.
fn binomial(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let mut result = 1usize;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}
