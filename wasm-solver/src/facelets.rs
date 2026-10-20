//! Facelet strings: parsing, formatting and applying sticker permutations.

use crate::geometry::{B, D, F, FACE_CHARS, L, NUM_FACELETS, NUM_FACES, R, U, UNKNOWN};

/// The solved facelet string: `UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB`.
pub fn solved_string() -> String {
    FACE_CHARS
        .iter()
        .flat_map(|c| std::iter::repeat(*c).take(9))
        .collect()
}

/// Apply a facelet permutation (`perm[to] = from`) to a cube state.
pub(crate) fn apply_perm(
    state: &[u8; NUM_FACELETS],
    perm: &[u8; NUM_FACELETS],
) -> [u8; NUM_FACELETS] {
    let mut out = [UNKNOWN; NUM_FACELETS];
    for i in 0..NUM_FACELETS {
        out[i] = state[perm[i] as usize];
    }
    out
}

/// Parse a facelet string into sticker ids.
pub fn parse_facelets(facelets: &str) -> Result<[u8; NUM_FACELETS], String> {
    let chars: Vec<char> = facelets.chars().collect();
    if chars.len() != NUM_FACELETS {
        return Err(format!(
            "Invalid facelet string: expected {NUM_FACELETS} characters, received {}.",
            chars.len()
        ));
    }
    let mut stickers = [UNKNOWN; NUM_FACELETS];
    for (i, c) in chars.iter().enumerate() {
        match *c {
            'U' => stickers[i] = U,
            'R' => stickers[i] = R,
            'F' => stickers[i] = F,
            'D' => stickers[i] = D,
            'L' => stickers[i] = L,
            'B' => stickers[i] = B,
            // `?` (and `.`) mark a sticker that has not been painted yet. They
            // are legal while painting the cube; `solve_cube` rejects them.
            '?' | '.' => stickers[i] = UNKNOWN,
            other => {
                return Err(format!(
                    "Invalid facelet string: unexpected character '{other}' at index {i}; \
                     expected one of U, R, F, D, L, B (uppercase Singmaster faces)."
                ))
            }
        }
    }
    Ok(stickers)
}

/// Render a sticker array back into a facelet string.
pub fn format_facelets(state: &[u8; NUM_FACELETS]) -> String {
    state
        .iter()
        .map(|s| if *s < 6 { FACE_CHARS[*s as usize] } else { '?' })
        .collect()
}

/// Is the given facelet string a solved cube?
///
/// Cheap: it only compares the stickers of every face against that face's
/// centre. Unpainted stickers (`?`) make it answer `false`.
pub fn is_solved(facelets: &str) -> Result<bool, String> {
    let state = parse_facelets(facelets)?;
    for face in 0..NUM_FACES {
        let sticker = state[face * 9 + 4];
        for k in 0..9 {
            if state[face * 9 + k] != sticker {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
