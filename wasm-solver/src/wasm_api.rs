//! The `#[wasm_bindgen]` surface of the crate.
//!
//! These are thin wrappers around the plain Rust functions in the other modules
//! (which are the ones the tests exercise, because `#[wasm_bindgen]` functions
//! taking `&str` cannot run outside WebAssembly); they only convert the error
//! type to `JsValue`.

use wasm_bindgen::prelude::*;

use crate::facelets::{is_solved, solved_string};
use crate::moves::{
    apply_moves, move_face, move_indices, move_variant, parse_move, FACE_TURNS, NUM_MOVES,
};
use crate::solver::{solve_facelets, validation_error};
use crate::tables::TABLES;

/// Solve a cube given as a 54 character UDSLRF facelet string.
///
/// `facelets` is the concatenation of the six faces in the order
/// `U, R, F, D, L, B`, nine stickers each, using the letters `U R F D L B` as
/// colour codes, e.g. `"UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB"`.
/// `max_depth` is the longest solution that is acceptable (0 picks a sane
/// default). The result is a space separated Singmaster move list such as
/// `"R U R' U' F2"`, or an empty string when the cube is already solved.
///
/// Every kind of unusable input produces an informative error instead of a
/// panic: wrong length, unknown characters, wrong sticker counts, misplaced
/// centres, impossible cubies, a single twisted corner, a single flipped edge or
/// mismatched permutation parity.
#[wasm_bindgen]
pub fn solve_cube(facelets: &str, max_depth: u8) -> Result<String, JsValue> {
    solve_facelets(facelets, max_depth).map_err(|error| JsValue::from_str(&error))
}

/// The solved facelet string, which is also the reference every other function
/// works against.
#[wasm_bindgen]
pub fn solved_facelets() -> String {
    solved_string()
}

// --- thin WebAssembly wrappers --------------------------------------------
//
// `#[wasm_bindgen]` functions that take `&str` cannot run on the host, so the
// logic lives in the plain Rust functions in the other modules and these
// wrappers only convert the error type.

/// Apply a Singmaster move list to a facelet string. See [`apply_moves`].
#[wasm_bindgen]
pub fn js_apply_moves(facelets: &str, moves: &str) -> Result<String, JsValue> {
    apply_moves(facelets, moves).map_err(|error| JsValue::from_str(&error))
}

/// Map every sticker of the cube after `moves` back to the original sticker.
/// See [`move_indices`].
#[wasm_bindgen]
pub fn js_move_indices(moves: &str) -> Result<Vec<u8>, JsValue> {
    move_indices(moves).map_err(|error| JsValue::from_str(&error))
}

/// Is the given facelet string a solved cube? See [`is_solved`].
#[wasm_bindgen]
pub fn js_is_solved(facelets: &str) -> Result<bool, JsValue> {
    is_solved(facelets).map_err(|error| JsValue::from_str(&error))
}

/// The first thing that stops this cube from being solved, or `undefined` when
/// it is solvable.
///
/// This runs the same checks as [`solve_cube`] but never searches, so the UI can
/// explain the problem while the user is still painting instead of waiting for a
/// failed solve. Unpainted stickers are accepted by the parser and reported by
/// [`validate_stickers`], which keeps this useful for a half painted cube.
#[wasm_bindgen]
pub fn js_validation_error(facelets: &str) -> Option<String> {
    validation_error(facelets)
}

/// Turn a Singmaster token (`"R"`, `"R2"`, `"U'"`) into a move index, or
/// `undefined` when the token is not a move. The UI uses this to translate the
/// solver's answer into animations.
#[wasm_bindgen]
pub fn move_index(token: &str) -> Option<u8> {
    parse_move(token).map(|m| m as u8)
}

/// Geometry of a move, as `[axis, layer, quarters]`.
///
/// `axis` is `0` for x, `1` for y and `2` for z, `layer` is `+1` or `-1` and
/// `quarters` counts *right-handed* quarter turns around the positive axis.
/// The 3D view rotates the layer mesh group by `quarters * 90Â°` so it can never
/// disagree with the solver about which way a face turns.
#[wasm_bindgen]
pub fn move_turn(move_index: u32) -> Vec<i8> {
    let m = move_index as usize;
    if m >= NUM_MOVES {
        return Vec::new();
    }
    let (axis, layer, base) = FACE_TURNS[move_face(m)];
    vec![
        axis as i8,
        layer,
        (base * (move_variant(m) as i32 + 1)) as i8,
    ]
}

/// Number of moves the solver understands (18: six faces, three variants).
#[wasm_bindgen]
pub fn num_moves() -> u32 {
    NUM_MOVES as u32
}

/// Build every lookup table up front.
///
/// Table generation is the only slow part of this module (roughly 6 MB of move
/// and pruning tables are filled with a breadth-first search). Calling this
/// while the user is still painting stickers makes the first "Solve" click feel
/// instant.
#[wasm_bindgen]
pub fn warm_up() {
    // Touching the tables is enough to force the lazy initialisation.
    let tables = &*TABLES;
    std::hint::black_box(tables.slice_goal);
}

/// Number of bytes of lookup tables currently held, for diagnostics.
#[wasm_bindgen]
pub fn table_bytes() -> usize {
    let t = &*TABLES;
    t.twist_move.len() * 2
        + t.flip_move.len() * 2
        + t.slice_move.len() * 2
        + t.corner_perm_move.len() * 2
        + t.ud_edge_perm_move.len() * 2
        + t.slice_perm_move.len() * 2
        + t.prune_twist_slice.len()
        + t.prune_flip_slice.len()
        + t.prune_corner_slice.len()
        + t.prune_edge_slice.len()
}

/// Version of the solver, surfaced in the UI footer.
#[wasm_bindgen]
pub fn solver_version() -> String {
    format!("two-phase {} (wasm)", env!("CARGO_PKG_VERSION"))
}

/// Install a panic hook that forwards Rust panics to the browser console.
#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
