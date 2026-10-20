//! The two-phase search: phase 1 drives the cube into the subgroup
//! `<U, D, R2, L2, F2, B2>`, phase 2 finishes it inside that subgroup.

use crate::coordinates::PHASE2_MOVES;
use crate::cubie::CubieState;
use crate::geometry::NUM_FACELETS;
use crate::moves::{apply_move, face_axis, move_face, NUM_MOVES};
use crate::tables::TABLES;

/// Longest phase-1 search that is ever attempted. The phase-1 diameter is
/// exactly 12, so nothing is lost by capping the depth here.
pub(crate) const MAX_PHASE1_DEPTH: usize = 12;

/// Absolute upper bound of a returned solution: the phase-1 diameter plus the
/// phase-2 diameter. Every solvable cube fits inside this bound.
pub const MAX_SOLUTION_LENGTH: usize = 30;

/// Node budget of the attempt that respects the caller's length limit.
pub(crate) const NODE_BUDGET: u64 = 20_000_000;
/// Node budget of the fallback attempt, which may return a longer solution but
/// is guaranteed to succeed.
pub(crate) const FALLBACK_NODE_BUDGET: u64 = 120_000_000;

/// How far below the caller's length limit the solver is willing to look for a
/// shorter solution.
///
/// Attempts that end up failing have to exhaust the phase-1 tree and are the
/// expensive ones, so only a small window of limits is probed: the search
/// starts at `max_depth - LENGTH_WINDOW` and grows to `max_depth`, which
/// reliably shaves a couple of moves off without a noticeable wait.
pub(crate) const LENGTH_WINDOW: usize = 2;

/// Would a move on `face` be redundant after a move on `last_face`?
///
/// Two turns of the same face can always be merged, and turns of opposite faces
/// (U/D, R/L, F/B) commute, so only one of the two orders is ever explored.
fn move_allowed(face: usize, last_face: Option<usize>) -> bool {
    match last_face {
        None => true,
        Some(last) => face != last && !(face_axis(face) == face_axis(last) && face < last),
    }
}

/// The iterative-deepening two-phase search.
pub(crate) struct Search {
    /// The cube being solved, as stickers, so the phase-1 stack can be replayed
    /// to obtain the exact state phase 2 has to start from.
    start: [u8; NUM_FACELETS],
    phase1: Vec<usize>,
    phase2: Vec<usize>,
    pub(crate) nodes: u64,
    node_budget: u64,
    pub(crate) abandoned: bool,
}

impl Search {
    pub(crate) fn new(start: [u8; NUM_FACELETS], node_budget: u64) -> Self {
        Search {
            start,
            phase1: Vec::with_capacity(MAX_PHASE1_DEPTH + 1),
            phase2: Vec::with_capacity(24),
            nodes: 0,
            node_budget,
            abandoned: false,
        }
    }

    /// Lower bound on the number of moves needed to reach the phase-1 goal.
    pub(crate) fn phase1_lower_bound(&self) -> usize {
        let cubies = CubieState::read(&self.start).expect("solvable cube state");
        TABLES.phase1_heuristic(
            cubies.twist() as usize,
            cubies.flip() as usize,
            cubies.slice() as usize,
        )
    }

    /// Look for a solution of at most `limit` moves.
    ///
    /// Phase 1 is deepened one move at a time; as soon as a phase-1 endpoint is
    /// reached, phase 2 is solved from there with an increasing budget, so the
    /// first solution found is the shortest one this strategy can see.
    pub(crate) fn solve_within(&mut self, limit: usize) -> Option<Vec<usize>> {
        let cubies = CubieState::read(&self.start).ok()?;
        if cubies.is_solved() {
            return Some(Vec::new());
        }
        let twist = cubies.twist() as usize;
        let flip = cubies.flip() as usize;
        let slice = cubies.slice() as usize;

        let tables = &*TABLES;
        let h1 = tables.phase1_heuristic(twist, flip, slice);
        let deepest = MAX_PHASE1_DEPTH.min(limit);
        for depth1 in h1..=deepest {
            if self.abandoned {
                return None;
            }
            let budget2 = limit - depth1;
            self.phase1.clear();
            self.phase2.clear();
            if self.search_phase1(twist, flip, slice, depth1, None, budget2) {
                let mut solution = std::mem::take(&mut self.phase1);
                solution.extend(self.phase2.iter().copied());
                return Some(solution);
            }
        }
        None
    }
}

/// Recursive part of the search, split into its own `impl` block for
/// readability.
impl Search {
    /// Depth-first phase-1 search with a fixed depth of exactly `depth_left`.
    fn search_phase1(
        &mut self,
        twist: usize,
        flip: usize,
        slice: usize,
        depth_left: usize,
        last_face: Option<usize>,
        budget2: usize,
    ) -> bool {
        let tables = &*TABLES;
        if tables.phase1_heuristic(twist, flip, slice) > depth_left {
            return false;
        }
        if depth_left == 0 {
            if twist != 0 || flip != 0 || slice != tables.slice_goal {
                return false;
            }
            return self.solve_phase2_from_stack(budget2, last_face);
        }

        self.nodes += 1;
        if self.nodes > self.node_budget {
            self.abandoned = true;
            return false;
        }

        for m in 0..NUM_MOVES {
            let face = move_face(m);
            if !move_allowed(face, last_face) {
                continue;
            }
            let next_twist = tables.twist_move[twist * NUM_MOVES + m] as usize;
            let next_flip = tables.flip_move[flip * NUM_MOVES + m] as usize;
            let next_slice = tables.slice_move[slice * NUM_MOVES + m] as usize;
            self.phase1.push(m);
            if self.search_phase1(
                next_twist,
                next_flip,
                next_slice,
                depth_left - 1,
                Some(face),
                budget2,
            ) {
                return true;
            }
            self.phase1.pop();
            if self.abandoned {
                return false;
            }
        }
        false
    }

    /// Replay the phase-1 moves on the original cube to obtain the exact state
    /// phase 2 starts from.
    fn state_after_phase1(&self) -> [u8; NUM_FACELETS] {
        let mut state = self.start;
        for &m in &self.phase1 {
            state = apply_move(&state, m);
        }
        state
    }

    /// Solve phase 2 from the current phase-1 endpoint.
    fn solve_phase2_from_stack(&mut self, budget: usize, last_face: Option<usize>) -> bool {
        let state = self.state_after_phase1();
        let Ok(cubies) = CubieState::read(&state) else {
            return false;
        };
        // The phase-1 goal guarantees this, but the check costs nothing and
        // keeps the subtraction below from ever underflowing.
        if cubies.ep[0..8].iter().any(|e| *e >= 8) || cubies.ep[8..12].iter().any(|e| *e < 8) {
            return false;
        }
        let corner_perm = cubies.corner_perm() as usize;
        let ud_edge_perm = cubies.ud_edge_perm() as usize;
        let slice_perm = cubies.slice_perm() as usize;

        let tables = &*TABLES;
        let h = tables.phase2_heuristic(corner_perm, ud_edge_perm, slice_perm);
        if h > budget {
            return false;
        }
        for limit in h..=budget {
            self.phase2.clear();
            if self.search_phase2(corner_perm, ud_edge_perm, slice_perm, limit, last_face) {
                return true;
            }
            if self.abandoned {
                return false;
            }
        }
        false
    }

    /// Depth-first phase-2 search with a fixed depth of exactly `depth_left`.
    fn search_phase2(
        &mut self,
        corner_perm: usize,
        ud_edge_perm: usize,
        slice_perm: usize,
        depth_left: usize,
        last_face: Option<usize>,
    ) -> bool {
        let tables = &*TABLES;
        if tables.phase2_heuristic(corner_perm, ud_edge_perm, slice_perm) > depth_left {
            return false;
        }
        if depth_left == 0 {
            return corner_perm == 0 && ud_edge_perm == 0 && slice_perm == 0;
        }

        self.nodes += 1;
        if self.nodes > self.node_budget {
            self.abandoned = true;
            return false;
        }

        let num_moves = PHASE2_MOVES.len();
        for (slot, &m) in PHASE2_MOVES.iter().enumerate() {
            let face = move_face(m);
            if !move_allowed(face, last_face) {
                continue;
            }
            let next_corner = tables.corner_perm_move[corner_perm * num_moves + slot] as usize;
            let next_edge = tables.ud_edge_perm_move[ud_edge_perm * num_moves + slot] as usize;
            let next_slice = tables.slice_perm_move[slice_perm * num_moves + slot] as usize;
            self.phase2.push(m);
            if self.search_phase2(
                next_corner,
                next_edge,
                next_slice,
                depth_left - 1,
                Some(face),
            ) {
                return true;
            }
            self.phase2.pop();
            if self.abandoned {
                return false;
            }
        }
        false
    }
}
