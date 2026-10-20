//! The move and pruning tables the two-phase search runs on.
//!
//! Move tables are built by applying every move to the canonical state of every
//! coordinate value and reading the coordinate back, so they are self-consistent
//! by construction. Pruning tables are a breadth-first search from the goal,
//! which makes them an admissible lower bound on the moves still needed.

use std::sync::LazyLock;

use crate::combinatorics::{perm_index, perm_unrank, subset_index};
use crate::coordinates::{
    state_from_flip, state_from_slice, state_from_twist, N_CORNER_PERM, N_EDGE_PERM, N_FLIP,
    N_SLICE, N_SLICE_PERM, N_TWIST, PHASE2_MOVES,
};
use crate::cubie::CubieState;
use crate::geometry::{rotate, CORNERS, EDGES, NUM_FACELETS, SLICE_SLOTS};
use crate::moves::{apply_move, move_face, move_quarters, FACE_TURNS, NUM_MOVES};

// ---------------------------------------------------------------------------
// Move tables
// ---------------------------------------------------------------------------

/// Destination slot of every corner slot under a move, i.e. `dst[src]`.
fn corner_slot_perm(m: usize) -> [u8; 8] {
    let (axis, layer, _) = FACE_TURNS[move_face(m)];
    let quarters = move_quarters(m);
    std::array::from_fn(|src| {
        let p = CORNERS[src].0;
        if p[axis] != layer {
            return src as u8;
        }
        let target = rotate(p, axis, quarters);
        CORNERS
            .iter()
            .position(|(q, _)| *q == target)
            .expect("corner grid") as u8
    })
}

/// Destination slot of every edge slot under a move, i.e. `dst[src]`.
fn edge_slot_perm(m: usize) -> [u8; 12] {
    let (axis, layer, _) = FACE_TURNS[move_face(m)];
    let quarters = move_quarters(m);
    std::array::from_fn(|src| {
        let p = EDGES[src].0;
        if p[axis] != layer {
            return src as u8;
        }
        let target = rotate(p, axis, quarters);
        EDGES
            .iter()
            .position(|(q, _)| *q == target)
            .expect("edge grid") as u8
    })
}

/// `inverse[perm[src]] = src`.
fn inverse_slot_perm<const N: usize>(perm: &[u8; N]) -> [u8; N] {
    let mut inverse = [0u8; N];
    for (src, &dst) in perm.iter().enumerate() {
        inverse[dst as usize] = src as u8;
    }
    inverse
}

/// The U/D edge part of the edge slot permutation, as a permutation of `0..8`.
fn ud_slot_perm(m: usize) -> [u8; 8] {
    let full = edge_slot_perm(m);
    let mut out = [0u8; 8];
    for (i, slot) in out.iter_mut().enumerate() {
        let dst = full[i] as usize;
        assert!(
            dst < 8,
            "a phase-2 move must keep the U/D edges in their layer"
        );
        *slot = dst as u8;
    }
    out
}

/// The middle layer part of the edge slot permutation, as a permutation of `0..4`.
fn slice_slot_perm(m: usize) -> [u8; 4] {
    let full = edge_slot_perm(m);
    let mut out = [0u8; 4];
    for (k, slot) in out.iter_mut().enumerate() {
        let dst = full[SLICE_SLOTS[k]] as usize;
        assert!(
            dst >= 8,
            "a phase-2 move must keep the middle layer together"
        );
        *slot = (dst - 8) as u8;
    }
    out
}

/// `count x moves.len()` transition table for a coordinate read from stickers.
///
/// Every move is applied to the canonical state of the coordinate value and the
/// coordinate is read back, which is what makes the tables self-consistent.
fn build_state_move_table(
    count: usize,
    moves: &[usize],
    state_of: impl Fn(usize) -> [u8; NUM_FACELETS],
    coord_of: impl Fn(&CubieState) -> usize,
) -> Vec<u16> {
    let nm = moves.len();
    let mut table = vec![0u16; count * nm];
    for value in 0..count {
        let base = state_of(value);
        for (i, &m) in moves.iter().enumerate() {
            let moved = apply_move(&base, m);
            let cubies = CubieState::read(&moved).expect("canonical state must be readable");
            table[value * nm + i] = coord_of(&cubies) as u16;
        }
    }
    table
}

/// `count x moves.len()` transition table for a coordinate that *is* a cubie
/// slot permutation. Much cheaper than the sticker based version and exact.
fn build_perm_move_table<const N: usize>(
    count: usize,
    moves: &[usize],
    inverse_slot_perms: &[[u8; N]; NUM_MOVES],
    index_to_perm: impl Fn(usize) -> Vec<u8>,
    perm_to_index: impl Fn(&[u8]) -> usize,
) -> Vec<u16> {
    let nm = moves.len();
    let mut table = vec![0u16; count * nm];
    let mut next = vec![0u8; N];
    for value in 0..count {
        let perm = index_to_perm(value);
        for (i, &m) in moves.iter().enumerate() {
            let inverse = &inverse_slot_perms[m];
            for j in 0..N {
                next[j] = perm[inverse[j] as usize];
            }
            table[value * nm + i] = perm_to_index(&next) as u16;
        }
    }
    table
}

/// A breadth-first pruning table over two coordinates, in bytes.
///
/// The search is undirected (the move set is closed under inverses), so a BFS
/// starting at the goal gives the exact minimum number of moves needed to bring
/// the coordinate pair to the goal. That value is a lower bound for the whole
/// cube, which is what an admissible heuristic needs.
fn build_prune_table(
    n_a: usize,
    n_b: usize,
    num_moves: usize,
    move_a: &[u16],
    move_b: &[u16],
    start_a: usize,
    start_b: usize,
) -> Vec<u8> {
    let mut table = vec![0xFFu8; n_a * n_b];
    let start = start_a * n_b + start_b;
    table[start] = 0;
    let mut queue: Vec<u32> = Vec::with_capacity(1 << 16);
    queue.push(start as u32);
    let mut head = 0usize;
    while head < queue.len() {
        let current = queue[head] as usize;
        head += 1;
        let depth = table[current];
        let a = current / n_b;
        let b = current % n_b;
        for i in 0..num_moves {
            let na = move_a[a * num_moves + i] as usize;
            let nb = move_b[b * num_moves + i] as usize;
            let next = na * n_b + nb;
            if table[next] == 0xFF {
                table[next] = depth + 1;
                queue.push(next as u32);
            }
        }
    }
    table
}

// ---------------------------------------------------------------------------
// The complete table set
// ---------------------------------------------------------------------------

/// Everything the two-phase search needs. Built once, lazily.
pub struct Tables {
    pub(crate) twist_move: Vec<u16>,
    pub(crate) flip_move: Vec<u16>,
    pub(crate) slice_move: Vec<u16>,
    pub(crate) corner_perm_move: Vec<u16>,
    pub(crate) ud_edge_perm_move: Vec<u16>,
    pub(crate) slice_perm_move: Vec<u16>,
    pub(crate) prune_twist_slice: Vec<u8>,
    pub(crate) prune_flip_slice: Vec<u8>,
    pub(crate) prune_corner_slice: Vec<u8>,
    pub(crate) prune_edge_slice: Vec<u8>,
    /// Index of the subset that puts all slice edges back in the middle layer.
    pub(crate) slice_goal: usize,
}

impl Tables {
    fn new() -> Self {
        let all_moves: Vec<usize> = (0..NUM_MOVES).collect();
        let phase2: Vec<usize> = PHASE2_MOVES.to_vec();

        // -- phase 1 coordinates (read from stickers, small coordinate spaces) --
        let twist_move = build_state_move_table(
            N_TWIST,
            &all_moves,
            |t| state_from_twist(t as u16),
            |c| c.twist() as usize,
        );
        let flip_move = build_state_move_table(
            N_FLIP,
            &all_moves,
            |f| state_from_flip(f as u16),
            |c| c.flip() as usize,
        );
        let slice_move = build_state_move_table(N_SLICE, &all_moves, state_from_slice, |c| {
            c.slice() as usize
        });

        // -- phase 2 coordinates (pure permutations, large coordinate spaces) --
        let corner_inverse_perms: [[u8; 8]; NUM_MOVES] =
            std::array::from_fn(|m| inverse_slot_perm(&corner_slot_perm(m)));

        let mut ud_inverse_perms = [[0u8; 8]; NUM_MOVES];
        let mut slice_inverse_perms = [[0u8; 4]; NUM_MOVES];
        for &m in PHASE2_MOVES.iter() {
            ud_inverse_perms[m] = inverse_slot_perm(&ud_slot_perm(m));
            slice_inverse_perms[m] = inverse_slot_perm(&slice_slot_perm(m));
        }

        let corner_perm_move = build_perm_move_table::<8>(
            N_CORNER_PERM,
            &phase2,
            &corner_inverse_perms,
            |i| perm_unrank(i as u32, 8),
            |p| perm_index(p) as usize,
        );
        let ud_edge_perm_move = build_perm_move_table::<8>(
            N_EDGE_PERM,
            &phase2,
            &ud_inverse_perms,
            |i| perm_unrank(i as u32, 8),
            |p| perm_index(p) as usize,
        );
        let slice_perm_move = build_perm_move_table::<4>(
            N_SLICE_PERM,
            &phase2,
            &slice_inverse_perms,
            |i| perm_unrank(i as u32, 4),
            |p| perm_index(p) as usize,
        );

        let slice_goal = subset_index(&[8, 9, 10, 11]);

        // -- pruning tables --
        let prune_twist_slice = build_prune_table(
            N_TWIST,
            N_SLICE,
            all_moves.len(),
            &twist_move,
            &slice_move,
            0,
            slice_goal,
        );
        let prune_flip_slice = build_prune_table(
            N_FLIP,
            N_SLICE,
            all_moves.len(),
            &flip_move,
            &slice_move,
            0,
            slice_goal,
        );
        let prune_corner_slice = build_prune_table(
            N_CORNER_PERM,
            N_SLICE_PERM,
            phase2.len(),
            &corner_perm_move,
            &slice_perm_move,
            0,
            0,
        );
        let prune_edge_slice = build_prune_table(
            N_EDGE_PERM,
            N_SLICE_PERM,
            phase2.len(),
            &ud_edge_perm_move,
            &slice_perm_move,
            0,
            0,
        );

        Tables {
            twist_move,
            flip_move,
            slice_move,
            corner_perm_move,
            ud_edge_perm_move,
            slice_perm_move,
            prune_twist_slice,
            prune_flip_slice,
            prune_corner_slice,
            prune_edge_slice,
            slice_goal,
        }
    }

    /// Lower bound on the number of moves needed to reach the phase-1 goal.
    pub(crate) fn phase1_heuristic(&self, twist: usize, flip: usize, slice: usize) -> usize {
        let a = self.prune_twist_slice[twist * N_SLICE + slice];
        let b = self.prune_flip_slice[flip * N_SLICE + slice];
        usize::from(a.max(b))
    }

    /// Lower bound on the number of moves needed to reach the phase-2 goal.
    pub(crate) fn phase2_heuristic(
        &self,
        corner_perm: usize,
        ud_edge_perm: usize,
        slice_perm: usize,
    ) -> usize {
        let a = self.prune_corner_slice[corner_perm * N_SLICE_PERM + slice_perm];
        let b = self.prune_edge_slice[ud_edge_perm * N_SLICE_PERM + slice_perm];
        usize::from(a.max(b))
    }
}

/// The process-wide table set. Building it takes a moment, which is why the
/// frontend calls [`warm_up`] while the user is still painting stickers.
pub(crate) static TABLES: LazyLock<Tables> = LazyLock::new(Tables::new);
