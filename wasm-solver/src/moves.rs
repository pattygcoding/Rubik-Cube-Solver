//! Move encoding, face turns and Singmaster notation.
//!
//! A move is `face * 3 + variant`, with `0 = X`, `1 = X2` and `2 = X'`. Its
//! effect on a cube is a permutation of the 54 stickers, obtained by rotating
//! the stickers of the moving layer, so the moves can never disagree with the
//! geometry.

use std::sync::LazyLock;

use crate::facelets::{apply_perm, format_facelets, parse_facelets};
use crate::geometry::{rotate, AXIS_X, AXIS_Y, AXIS_Z, FACE_CHARS, GEO, NUM_FACELETS, NUM_FACES};

/// The six outer layer turns: `(axis, layer, quarters)`, where `quarters` is
/// the number of right-handed quarter turns about the *positive* axis that make
/// the face turn clockwise as seen from outside the cube.
pub(crate) const FACE_TURNS: [(usize, i8, i32); NUM_FACES] = [
    (AXIS_Y, 1, -1), // U
    (AXIS_X, 1, -1), // R
    (AXIS_Z, 1, -1), // F
    (AXIS_Y, -1, 1), // D
    (AXIS_X, -1, 1), // L
    (AXIS_Z, -1, 1), // B
];

/// A move is encoded as `face * 3 + variant`, `variant` being
/// `0 = X`, `1 = X2`, `2 = X'`. There are 18 moves in total.
pub const NUM_MOVES: usize = 18;

pub(crate) fn move_face(m: usize) -> usize {
    m / 3
}
pub(crate) fn move_variant(m: usize) -> usize {
    m % 3
}
pub(crate) fn move_quarters(m: usize) -> i32 {
    FACE_TURNS[move_face(m)].2 * (move_variant(m) as i32 + 1)
}

/// The axis a face turn rotates around (`U`/`D` -> y, `R`/`L` -> x, `F`/`B` -> z).
pub(crate) fn face_axis(face: usize) -> usize {
    FACE_TURNS[face].0
}

/// Singmaster name of a move, e.g. `"R2"` or `"U'"`.
pub fn move_name(m: usize) -> String {
    let mut name = String::new();
    name.push(FACE_CHARS[move_face(m)]);
    match move_variant(m) {
        1 => name.push('2'),
        2 => name.push('\''),
        _ => {}
    }
    name
}

/// Parse a Singmaster token such as `"R"`, `"R2"` or `"U'"` into a move index.
pub fn parse_move(token: &str) -> Option<usize> {
    let mut chars = token.chars();
    let face = chars.next()?;
    let face_idx = FACE_CHARS.iter().position(|c| *c == face)?;
    let rest: String = chars.collect();
    let variant = match rest.as_str() {
        "" => 0,
        "2" => 1,
        "'" | "Ã¢â‚¬â„¢" => 2,
        "'2" | "2'" => 1,
        _ => return None,
    };
    Some(face_idx * 3 + variant)
}

/// The facelet permutation of a move, as `perm[to] = from`.
///
/// Only the nine stickers of the moving layer change; the permutation is
/// obtained by rotating their grid position *and* their normal, then looking up
/// which sticker ends up on the resulting `(position, normal)` pair.
fn facelet_perm(m: usize) -> [u8; NUM_FACELETS] {
    let (axis, layer, _) = FACE_TURNS[move_face(m)];
    let quarters = move_quarters(m);
    let geo = &*GEO;
    let mut perm = [0u8; NUM_FACELETS];
    for i in 0..NUM_FACELETS {
        let (p, n) = geo.stickers[i];
        if p[axis] != layer {
            perm[i] = i as u8;
            continue;
        }
        let dest = geo.index_of(rotate(p, axis, quarters), rotate(n, axis, quarters));
        perm[dest] = i as u8;
    }
    perm
}

/// The 18 facelet permutations, indexed by move.
pub(crate) static MOVEPERM: LazyLock<[[u8; NUM_FACELETS]; NUM_MOVES]> =
    LazyLock::new(|| std::array::from_fn(facelet_perm));

/// Apply a move to a cube state.
pub(crate) fn apply_move(state: &[u8; NUM_FACELETS], m: usize) -> [u8; NUM_FACELETS] {
    apply_perm(state, &MOVEPERM[m])
}

/// Compose the moves of `moves` into a single permutation.
///
/// The result maps a *displayed* facelet to the facelet of the original cube
/// that is currently sitting there, i.e. `displayed[i] = base[composed[i]]`.
pub(crate) fn compose_perm(moves: &[usize]) -> [u8; NUM_FACELETS] {
    let mut composed: [u8; NUM_FACELETS] = std::array::from_fn(|i| i as u8);
    for &m in moves {
        let perm = &MOVEPERM[m];
        let mut next = [0u8; NUM_FACELETS];
        for i in 0..NUM_FACELETS {
            next[i] = composed[perm[i] as usize];
        }
        composed = next;
    }
    composed
}

/// Split a Singmaster move string such as `"R U R' U' F2"` into move indices.
pub fn parse_moves(moves: &str) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for token in moves.split_whitespace() {
        match parse_move(token) {
            Some(m) => out.push(m),
            None => {
                return Err(format!(
                    "Unrecognised move '{token}'. Expected Singmaster notation such as \
                     R, R2, R', U, D2, L'."
                ))
            }
        }
    }
    Ok(out)
}

/// Apply a Singmaster move list to a facelet string.
///
/// This is the authoritative implementation of "what does this move do", so the
/// frontend never has to model moves itself. `?` (or `.`) marks a sticker that
/// has not been painted yet and survives the permutation.
pub fn apply_moves(facelets: &str, moves: &str) -> Result<String, String> {
    let mut state = parse_facelets(facelets)?;
    for m in parse_moves(moves)? {
        state = apply_move(&state, m);
    }
    Ok(format_facelets(&state))
}

/// For a Singmaster move list, `result[displayed] = original` maps every sticker
/// of the resulting cube back to the sticker of the input cube it came from.
/// The UI uses this to keep painting accurate while a solution is displayed.
pub fn move_indices(moves: &str) -> Result<Vec<u8>, String> {
    Ok(compose_perm(&parse_moves(moves)?).to_vec())
}
