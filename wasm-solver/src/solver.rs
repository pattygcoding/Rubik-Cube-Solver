//! The public API of the crate: validation and solving.
//!
//! Both entry points share [`checked_state`], so the UI and the solver can never
//! disagree about whether a cube is usable.

use crate::cubie::CubieState;
use crate::facelets::parse_facelets;
use crate::geometry::{FACE_CHARS, NUM_FACELETS, NUM_FACES, UNKNOWN};
use crate::moves::move_name;
use crate::search::{
    Search, FALLBACK_NODE_BUDGET, LENGTH_WINDOW, MAX_SOLUTION_LENGTH, NODE_BUDGET,
};

/// The default length limit when the caller passes an unusable `max_depth`.
pub const DEFAULT_MAX_DEPTH: u8 = 22;

/// Check the sticker level constraints: nine stickers of every colour and the
/// six centres in place. Everything else is checked on the cubie level.
pub fn validate_stickers(state: &[u8; NUM_FACELETS]) -> Result<(), String> {
    let mut counts = [0usize; 7];
    for sticker in state.iter() {
        counts[*sticker as usize] += 1;
    }
    if counts[UNKNOWN as usize] > 0 {
        return Err(format!(
            "Invalid cube: {} of the 54 stickers have not been painted yet. Paint them, or \
             press \"Clear stickers\" to start from a blank cube.",
            counts[UNKNOWN as usize]
        ));
    }
    for face in 0..NUM_FACES {
        let count = counts[face];
        if count != 9 {
            let hint = if count > 9 {
                "At least one of them belongs on another face."
            } else {
                "One of the other colours is painted where this one belongs."
            };
            return Err(format!(
                "Invalid cube: found {count} '{}' stickers but a cube needs exactly 9. {hint}",
                FACE_CHARS[face]
            ));
        }
    }
    for face in 0..NUM_FACES {
        let centre = face * 9 + 4;
        if state[centre] != face as u8 {
            return Err(format!(
                "Invalid cube: the centre of the {} face must be '{}' because centres never \
                 move. Press \"Solved cube\" to restore them.",
                FACE_CHARS[face], FACE_CHARS[face]
            ));
        }
    }
    Ok(())
}

/// Parse a facelet string and check everything that can be checked without
/// searching: characters, sticker counts, centres and the cubie level physics.
///
/// Both [`solve_facelets`] and [`validation_error`] go through this, so the UI
/// and the solver can never disagree about whether a cube is usable.
pub fn checked_state(facelets: &str) -> Result<[u8; NUM_FACELETS], String> {
    let state = parse_facelets(facelets)?;
    validate_stickers(&state)?;
    // Reading the cubie state is where the physical constraints are enforced.
    CubieState::from_facelets(&state)?;
    Ok(state)
}

/// The first thing that stops this cube from being solved, or `None` when it is
/// solvable. Used by the UI to explain the problem while the user paints.
pub fn validation_error(facelets: &str) -> Option<String> {
    checked_state(facelets).err()
}

/// Solve a cube, returned as a list of move indices.
///
/// First the search is asked for a solution of at most `max_depth` moves. If
/// that is impossible (or the node budget runs out) the search is repeated with
/// the theoretical bound of [`MAX_SOLUTION_LENGTH`] moves, which always
/// succeeds for a solvable cube, so the caller always gets an answer.
pub fn solve_state(state: &[u8; NUM_FACELETS], max_depth: u8) -> Result<Vec<usize>, String> {
    let cubies = CubieState::from_facelets(state)?;
    if cubies.is_solved() {
        return Ok(Vec::new());
    }

    let requested = if max_depth == 0 {
        DEFAULT_MAX_DEPTH as usize
    } else {
        max_depth as usize
    }
    .clamp(8, MAX_SOLUTION_LENGTH);

    // Asking for exactly `limit` moves and growing it inside a small window
    // below `requested` means the first solution found is the shortest one the
    // cheap strategy can produce, without exhausting the phase-1 tree.
    let mut search = Search::new(*state, NODE_BUDGET);
    let lower = search.phase1_lower_bound();
    let several = requested.saturating_sub(LENGTH_WINDOW).max(lower);
    for limit in several..=requested {
        if let Some(solution) = search.solve_within(limit) {
            return Ok(solution);
        }
        if search.abandoned {
            break;
        }
    }

    let mut fallback = Search::new(*state, FALLBACK_NODE_BUDGET);
    if let Some(solution) = fallback.solve_within(MAX_SOLUTION_LENGTH) {
        return Ok(solution);
    }

    Err(format!(
        "No solution found (the search explored {} nodes). The cube is solvable but the \
         search budget was exhausted.",
        search.nodes + fallback.nodes
    ))
}

/// Solve a cube given as a facelet string and render the answer in Singmaster
/// notation.
pub fn solve_facelets(facelets: &str, max_depth: u8) -> Result<String, String> {
    let state = checked_state(facelets)?;
    let moves = solve_state(&state, max_depth)?;
    Ok(moves
        .iter()
        .map(|m| move_name(*m))
        .collect::<Vec<_>>()
        .join(" "))
}
