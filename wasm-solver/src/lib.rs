//! # Two-phase Rubik's Cube solver (WebAssembly)
//!
//! A self-contained implementation of the two-phase search popularised by
//! Herbert Kociemba's Cube Explorer, compiled to WebAssembly with
//! `wasm-bindgen`.
//!
//! ## How the search works
//!
//! * **Phase 1** brings the cube into the subgroup `G1 = <U, D, R2, L2, F2, B2>`,
//!   i.e. all corners untwisted, all edges unflipped and the four UD-slice edges
//!   back inside the middle slice. It is guided by the coordinates
//!   `twist (2187) x slice (495)` and `flip (2048) x slice (495)`.
//! * **Phase 2** finishes the cube inside `G1` using `U, U2, U', D, D2, D',
//!   R2, L2, F2, B2`, guided by `corner perm (40320) x slice perm (24)` and
//!   `UD edge perm (40320) x slice perm (24)`.
//!
//! Because the phase-1 diameter is 12 and the phase-2 diameter is 18, *every*
//! solvable cube can be solved with at most 30 moves, so the search is complete.
//!
//! ## Why the tables are derived instead of hard-coded
//!
//! Every table is generated at runtime from the cube's geometry: the
//! position/normal of all 54 stickers, the eight corner slots and the twelve
//! edge slots. Face turns are obtained by *rotating that geometry* and
//! coordinate transitions are obtained by building small, canonical
//! representative states (twist one corner in place, flip one edge in place,
//! permute cubies) and letting the facelet permutations act on them. There is
//! not a single hand-written lookup table, so a typo cannot silently produce a
//! solver that happens to disagree with itself.
//!
//! ## Layout
//!
//! | Module | Contents |
//! | --- | --- |
//! | [`geometry`] | faces, the integer grid, stickers and cubie slots |
//! | [`facelets`] | facelet string parsing, formatting and permutations |
//! | [`combinatorics`] | permutation and subset ranking, parity |
//! | [`cubie`] | the cubie level view of a cube, and its coordinates |
//! | [`moves`] | move encoding, face turns and Singmaster notation |
//! | [`coordinates`] | phase coordinates and their canonical states |
//! | [`tables`] | move and pruning tables |
//! | [`search`] | the two-phase iterative deepening search |
//! | [`solver`] | validation and the solve entry points |
//! | [`wasm_api`] | the `#[wasm_bindgen]` surface |
//!
//! The public Rust API is [`solver::solve_facelets`], [`solver::validation_error`],
//! [`moves::apply_moves`], [`moves::move_indices`] and [`facelets::is_solved`];
//! everything else is crate internal. The functions the browser calls are the
//! `#[wasm_bindgen]` wrappers in [`wasm_api`].

pub mod combinatorics;
pub mod coordinates;
pub mod cubie;
pub mod facelets;
pub mod geometry;
pub mod moves;
pub mod search;
pub mod solver;
pub mod tables;
pub mod wasm_api;

#[cfg(test)]
mod tests;

pub use crate::cubie::CubieState;
pub use crate::facelets::{format_facelets, is_solved, parse_facelets, solved_string};
pub use crate::moves::{apply_moves, move_indices, move_name, parse_move, parse_moves};
pub use crate::search::MAX_SOLUTION_LENGTH;
pub use crate::solver::{
    checked_state, solve_facelets, solve_state, validate_stickers, validation_error,
};
