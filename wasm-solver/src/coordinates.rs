//! The coordinates the two-phase search works in, and the canonical cube state
//! each coordinate value stands for.
//!
//! A coordinate is a small integer capturing part of a cube state: corner twist,
//! edge flip, which slots hold the middle layer edges, and the three
//! permutations phase 2 needs. The `state_from_*` functions build the state a
//! value stands for, which is how the move tables are generated.

use crate::combinatorics::{perm_unrank, subset_unrank};
use crate::geometry::{GEO, NUM_FACELETS, SLICE_SLOTS};

pub(crate) const N_TWIST: usize = 2187; // 3^7
pub(crate) const N_FLIP: usize = 2048; // 2^11
pub(crate) const N_SLICE: usize = 495; // C(12, 4)
pub(crate) const N_CORNER_PERM: usize = 40320; // 8!
pub(crate) const N_EDGE_PERM: usize = 40320; // 8!
pub(crate) const N_SLICE_PERM: usize = 24; // 4!

/// The ten moves that stay inside the phase-2 subgroup `G1`:
/// `U, U2, U', D, D2, D', R2, L2, F2, B2`.
pub(crate) const PHASE2_MOVES: [usize; 10] = [0, 1, 2, 9, 10, 11, 4, 13, 7, 16];

/// Canonical cube state whose twist coordinate is `twist`: every corner is
/// twisted *in place* by cyclically shifting its three stickers.
pub(crate) fn state_from_twist(twist: u16) -> [u8; NUM_FACELETS] {
    let geo = &*GEO;
    let mut co = [0u8; 8];
    let mut value = twist as usize;
    let mut sum = 0usize;
    for i in 0..7 {
        co[i] = (value % 3) as u8;
        value /= 3;
        sum += co[i] as usize;
    }
    co[7] = ((3 - sum % 3) % 3) as u8;

    let mut state = geo.solved;
    for i in 0..8 {
        let slots = geo.corner_facelet[i];
        let shift = co[i] as usize;
        for k in 0..3 {
            // The sticker that used to live in slot `k - shift` moves into slot
            // `k`, so the U/D sticker of slot 0 lands in slot `co[i]`.
            state[slots[k]] = geo.solved[slots[(k + 3 - shift) % 3]];
        }
    }
    state
}

/// Canonical cube state whose flip coordinate is `flip`: the selected edges
/// have their two stickers exchanged.
pub(crate) fn state_from_flip(flip: u16) -> [u8; NUM_FACELETS] {
    let geo = &*GEO;
    let mut eo = [0u8; 12];
    let mut value = flip as usize;
    let mut sum = 0usize;
    for i in 0..11 {
        eo[i] = (value % 2) as u8;
        value /= 2;
        sum += eo[i] as usize;
    }
    eo[11] = (sum % 2) as u8;

    let mut state = geo.solved;
    for i in 0..12 {
        if eo[i] == 1 {
            let slots = geo.edge_facelet[i];
            state[slots[0]] = geo.solved[slots[1]];
            state[slots[1]] = geo.solved[slots[0]];
        }
    }
    state
}

/// Canonical cube state placing the four middle layer edges into the slots of
/// the given subset index.
pub(crate) fn state_from_slice(slice: usize) -> [u8; NUM_FACELETS] {
    let geo = &*GEO;
    let set = subset_unrank(slice);
    let mut assigned = [0u8; 12];
    let mut slice_cubie = 8u8;
    let mut ud_cubie = 0u8;
    for slot in 0..12 {
        if set.contains(&slot) {
            assigned[slot] = slice_cubie;
            slice_cubie += 1;
        } else {
            assigned[slot] = ud_cubie;
            ud_cubie += 1;
        }
    }
    let mut state = geo.solved;
    for slot in 0..12 {
        let cubie = assigned[slot] as usize;
        for j in 0..2 {
            state[geo.edge_facelet[slot][j]] = geo.edge_color[cubie][j];
        }
    }
    state
}

/// Canonical cube state with the given corner permutation and no twist.
///
/// The three `state_from_*_perm` helpers document what the phase-2 coordinates
/// mean and are used to pin that down in the unit tests; the move tables are
/// generated straight from the cubie slot permutations instead, which is much
/// cheaper for coordinate spaces with 40320 entries.
#[allow(dead_code)]
pub(crate) fn state_from_corner_perm(index: u32) -> [u8; NUM_FACELETS] {
    let geo = &*GEO;
    let perm = perm_unrank(index, 8);
    let mut state = geo.solved;
    for slot in 0..8 {
        let cubie = perm[slot] as usize;
        for j in 0..3 {
            state[geo.corner_facelet[slot][j]] = geo.corner_color[cubie][j];
        }
    }
    state
}

/// Canonical cube state with the given U/D edge permutation.
#[allow(dead_code)]
pub(crate) fn state_from_ud_edge_perm(index: u32) -> [u8; NUM_FACELETS] {
    let geo = &*GEO;
    let perm = perm_unrank(index, 8);
    let mut state = geo.solved;
    for slot in 0..8 {
        let cubie = perm[slot] as usize;
        for j in 0..2 {
            state[geo.edge_facelet[slot][j]] = geo.edge_color[cubie][j];
        }
    }
    state
}

/// Canonical cube state with the given middle layer edge permutation.
#[allow(dead_code)]
pub(crate) fn state_from_slice_perm(index: u32) -> [u8; NUM_FACELETS] {
    let geo = &*GEO;
    let perm = perm_unrank(index, 4);
    let mut state = geo.solved;
    for k in 0..4 {
        let slot = SLICE_SLOTS[k];
        let cubie = 8 + perm[k] as usize;
        for j in 0..2 {
            state[geo.edge_facelet[slot][j]] = geo.edge_color[cubie][j];
        }
    }
    state
}
