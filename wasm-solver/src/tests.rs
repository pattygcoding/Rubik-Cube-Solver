//! The test suite for the crate as a whole.
//!
//! These tests deliberately reach into most modules: they check the derived
//! tables, the coordinate round trips, random scrambles and validation.

use crate::combinatorics::*;
use crate::coordinates::*;
use crate::cubie::*;
use crate::facelets::*;
use crate::geometry::*;
use crate::moves::*;
use crate::search::*;
use crate::solver::*;
use crate::tables::*;

/// Deterministic pseudo random numbers, so failures are reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn solved() -> [u8; NUM_FACELETS] {
    GEO.solved
}

/// Hand verified sticker indices from Kociemba's `FaceCube` reference
/// implementation. They are the strongest check that the geometry in this
/// file is laid out exactly like the classic unfolded net.
#[test]
fn sticker_layout_matches_the_reference() {
    let expected_corner_facelet: [[usize; 3]; 8] = [
        [8, 9, 20],   // URF = U9 R1 F3
        [6, 18, 38],  // UFL = U7 F1 L3
        [0, 36, 47],  // ULB = U1 L1 B3
        [2, 45, 11],  // UBR = U3 B1 R3
        [29, 26, 15], // DFR = D3 F9 R7
        [27, 44, 24], // DLF = D1 L9 F7
        [33, 53, 42], // DBL = D7 B9 L7
        [35, 17, 51], // DRB = D9 R9 B7
    ];
    let expected_edge_facelet: [[usize; 2]; 12] = [
        [5, 10],  // UR = U6 R2
        [7, 19],  // UF = U8 F2
        [3, 37],  // UL = U4 L2
        [1, 46],  // UB = U2 B2
        [32, 16], // DR = D6 R8
        [28, 25], // DF = D2 F8
        [30, 43], // DL = D4 L8
        [34, 52], // DB = D8 B8
        [23, 12], // FR = F6 R4
        [21, 41], // FL = F4 L6
        [50, 39], // BL = B6 L4
        [48, 14], // BR = B4 R6
    ];
    assert_eq!(GEO.corner_facelet, expected_corner_facelet);
    assert_eq!(GEO.edge_facelet, expected_edge_facelet);

    let expected_corner_color: [[u8; 3]; 8] = [
        [U, R, F],
        [U, F, L],
        [U, L, B],
        [U, B, R],
        [D, F, R],
        [D, L, F],
        [D, B, L],
        [D, R, B],
    ];
    let expected_edge_color: [[u8; 2]; 12] = [
        [U, R],
        [U, F],
        [U, L],
        [U, B],
        [D, R],
        [D, F],
        [D, L],
        [D, B],
        [F, R],
        [F, L],
        [B, L],
        [B, R],
    ];
    assert_eq!(GEO.corner_color, expected_corner_color);
    assert_eq!(GEO.edge_color, expected_edge_color);

    assert_eq!(GEO.solved, parse_facelets(&solved_string()).unwrap());
}

/// A clockwise R turn, worked out by hand from the geometry: the URF corner
/// travels to UBR, the UR edge to BR, and so on.
#[test]
fn r_move_matches_the_hand_derived_tables() {
    let moved = apply_move(&solved(), R as usize * 3);
    let cubies = CubieState::read(&moved).unwrap();
    assert_eq!(cubies.cp, [4, 1, 2, 0, 7, 5, 6, 3]);
    assert_eq!(cubies.co, [2, 0, 0, 1, 1, 0, 0, 2]);
    assert_eq!(cubies.ep, [8, 1, 2, 3, 11, 5, 6, 7, 4, 9, 10, 0]);
    assert_eq!(cubies.eo, [0; 12]);
}

/// Four turns of a face must be the identity, and every permutation must be
/// a permutation.
#[test]
fn move_permutations_are_well_formed() {
    for m in 0..NUM_MOVES {
        let perm = MOVEPERM[m];
        let mut seen = [false; NUM_FACELETS];
        for p in perm {
            assert!(!seen[p as usize], "move {m} is not a permutation");
            seen[p as usize] = true;
        }
        let mut state = solved();
        for _ in 0..4 {
            state = apply_move(&state, m);
        }
        assert_eq!(
            state,
            solved(),
            "four {:?} turns must be the identity",
            move_name(m)
        );
    }
}

#[test]
fn subset_ranking_round_trips() {
    let mut count = 0usize;
    for a in 0..12 {
        for b in (a + 1)..12 {
            for c in (b + 1)..12 {
                for d in (c + 1)..12 {
                    let slots = [a, b, c, d];
                    let index = subset_index(&slots);
                    assert!(index < N_SLICE);
                    assert_eq!(subset_unrank(index), slots);
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, N_SLICE);
}

#[test]
fn permutation_ranking_round_trips() {
    for n in [4usize, 8] {
        let total = (1..=n).product::<usize>();
        for index in 0..total {
            let perm = perm_unrank(index as u32, n);
            assert_eq!(perm_index(&perm) as usize, index);
        }
    }
}

/// The canonical states must be exact inverses of the coordinate readers,
/// otherwise every move table built from them would be wrong.
#[test]
fn canonical_states_round_trip() {
    for twist in 0..N_TWIST as u16 {
        let cubies = CubieState::read(&state_from_twist(twist)).unwrap();
        assert_eq!(cubies.twist(), twist);
        assert_eq!(cubies.co.iter().map(|v| *v as u32).sum::<u32>() % 3, 0);
    }
    for flip in 0..N_FLIP as u16 {
        let cubies = CubieState::read(&state_from_flip(flip)).unwrap();
        assert_eq!(cubies.flip(), flip);
        assert_eq!(cubies.eo.iter().map(|v| *v as u32).sum::<u32>() % 2, 0);
    }
    for slice in 0..N_SLICE {
        let cubies = CubieState::read(&state_from_slice(slice)).unwrap();
        assert_eq!(cubies.slice() as usize, slice);
    }
    for index in 0..N_CORNER_PERM as u32 {
        let cubies = CubieState::read(&state_from_corner_perm(index)).unwrap();
        assert_eq!(cubies.corner_perm(), index);
    }
    for index in 0..N_EDGE_PERM as u32 {
        let cubies = CubieState::read(&state_from_ud_edge_perm(index)).unwrap();
        assert_eq!(cubies.ud_edge_perm(), index);
    }
    for index in 0..N_SLICE_PERM as u32 {
        let cubies = CubieState::read(&state_from_slice_perm(index)).unwrap();
        assert_eq!(cubies.slice_perm(), index);
    }
}

/// The cheap permutation based tables must agree with the straightforward
/// "apply the move and read the coordinate back" implementation.
#[test]
fn permutation_tables_agree_with_direct_simulation() {
    let tables = &*TABLES;
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..200 {
        let index = rng.below(N_CORNER_PERM) as u32;
        let state = state_from_corner_perm(index);
        for (slot, &m) in PHASE2_MOVES.iter().enumerate() {
            let moved = apply_move(&state, m);
            let expected = CubieState::read(&moved).unwrap().corner_perm() as usize;
            assert_eq!(
                tables.corner_perm_move[index as usize * 10 + slot] as usize,
                expected
            );
        }

        let index = rng.below(N_EDGE_PERM) as u32;
        let state = state_from_ud_edge_perm(index);
        for (slot, &m) in PHASE2_MOVES.iter().enumerate() {
            let moved = apply_move(&state, m);
            let expected = CubieState::read(&moved).unwrap().ud_edge_perm() as usize;
            assert_eq!(
                tables.ud_edge_perm_move[index as usize * 10 + slot] as usize,
                expected
            );
        }
    }
    for slice in 0..N_SLICE_PERM {
        let state = state_from_slice_perm(slice as u32);
        for (slot, &m) in PHASE2_MOVES.iter().enumerate() {
            let moved = apply_move(&state, m);
            let expected = CubieState::read(&moved).unwrap().slice_perm() as usize;
            assert_eq!(tables.slice_perm_move[slice * 10 + slot] as usize, expected);
        }
    }
}

#[test]
fn pruning_tables_are_zero_at_the_goal_and_bounded() {
    let tables = &*TABLES;
    // U and D turns do not change corner twist or edge flip, and they keep
    // the middle layer edges inside the middle layer.
    for face in [U as usize, D as usize] {
        for variant in 0..3 {
            let m = face * 3 + variant;
            let from_goal = tables.slice_goal * NUM_MOVES + m;
            assert_eq!(
                tables.twist_move[m],
                0,
                "{} must not twist corners",
                move_name(m)
            );
            assert_eq!(
                tables.flip_move[m],
                0,
                "{} must not flip edges",
                move_name(m)
            );
            assert_eq!(
                tables.slice_move[from_goal] as usize,
                tables.slice_goal,
                "{} must not move the slice edges out of the middle layer",
                move_name(m)
            );
        }
    }
    assert_eq!(tables.prune_twist_slice[tables.slice_goal], 0);
    assert_eq!(tables.prune_flip_slice[tables.slice_goal], 0);
    assert_eq!(tables.prune_corner_slice[0], 0);
    assert_eq!(tables.prune_edge_slice[0], 0);

    // These diameters come straight out of the BFS and are a compact
    // regression test for the whole table chain (geometry -> moves ->
    // coordinates -> pruning).
    let reachable_max = |table: &[u8]| table.iter().copied().filter(|v| *v != 0xFF).max().unwrap();
    assert_eq!(reachable_max(&tables.prune_twist_slice), 9);
    assert_eq!(reachable_max(&tables.prune_flip_slice), 9);
    assert_eq!(reachable_max(&tables.prune_corner_slice), 14);
    assert_eq!(reachable_max(&tables.prune_edge_slice), 12);
}

/// Solve a batch of random scrambles and verify every answer by replaying
/// it on the scramble.
#[test]
fn random_scrambles_are_solved() {
    let mut rng = Rng(0x0BAD_C0DE_1234_5678);
    let mut lengths = Vec::new();
    for round in 0..20 {
        let mut state = solved();
        for _ in 0..20 {
            state = apply_move(&state, rng.below(NUM_MOVES));
        }
        let scramble = format_facelets(&state);
        assert!(!is_solved(&scramble).unwrap());

        let answer = solve_facelets(&scramble, 22)
            .unwrap_or_else(|e| panic!("round {round} failed to solve: {e}"));
        assert!(!answer.is_empty());

        let mut check = state;
        for m in parse_moves(&answer).unwrap() {
            check = apply_move(&check, m);
        }
        assert_eq!(
            check,
            solved(),
            "round {round}: '{scramble}' + '{answer}' is not solved"
        );
        lengths.push(answer.split_whitespace().count());
    }
    let worst = *lengths.iter().max().unwrap();
    assert!(
        worst <= MAX_SOLUTION_LENGTH,
        "the worst solution had {worst} moves, above the {MAX_SOLUTION_LENGTH} bound"
    );
}

/// A cube that is already in the phase-2 subgroup is a good check of the
/// phase-2 half of the search.
#[test]
fn subgroup_cubes_are_solved() {
    let mut rng = Rng(0xFEED_FACE_0102_0304);
    for _ in 0..10 {
        let mut state = solved();
        for _ in 0..20 {
            let m = PHASE2_MOVES[rng.below(PHASE2_MOVES.len())];
            state = apply_move(&state, m);
        }
        let scramble = format_facelets(&state);
        let answer = solve_facelets(&scramble, 22).unwrap();
        let mut check = state;
        for m in parse_moves(&answer).unwrap() {
            check = apply_move(&check, m);
        }
        assert_eq!(check, solved());
    }
}

#[test]
fn the_solved_cube_needs_no_moves() {
    let answer = solve_facelets(&solved_string(), 22).unwrap();
    assert_eq!(answer, "");
    assert!(is_solved(&solved_string()).unwrap());
}

#[test]
fn facelet_strings_round_trip() {
    let mut state = solved();
    state[0] = UNKNOWN;
    state[37] = UNKNOWN;
    let text = format_facelets(&state);
    assert!(text.contains('?'));
    assert_eq!(parse_facelets(&text).unwrap(), state);
}

#[test]
fn invalid_inputs_are_reported() {
    // Wrong length.
    let error = solve_facelets("UUUU", 22).unwrap_err();
    assert!(error.contains("expected 54 characters"), "{error}");

    // Unknown character.
    let mut chars: Vec<char> = solved_string().chars().collect();
    chars[7] = 'X';
    let error = solve_facelets(&chars.iter().collect::<String>(), 22).unwrap_err();
    assert!(error.contains("unexpected character 'X'"), "{error}");

    // Wrong sticker counts (one 'U' became an 'R').
    let mut chars: Vec<char> = solved_string().chars().collect();
    chars[0] = 'R';
    let error = solve_facelets(&chars.iter().collect::<String>(), 22).unwrap_err();
    assert!(error.contains("needs exactly 9"), "{error}");

    // A centre that moved: swap the centres of U and R, which keeps the
    // per-colour counts intact.
    let mut chars: Vec<char> = solved_string().chars().collect();
    chars[4] = 'R';
    chars[13] = 'U';
    let error = solve_facelets(&chars.iter().collect::<String>(), 22).unwrap_err();
    assert!(error.contains("centre of the U face"), "{error}");

    // One corner twisted in place.
    let mut state = solved();
    let slots = GEO.corner_facelet[0];
    let original = [state[slots[0]], state[slots[1]], state[slots[2]]];
    state[slots[0]] = original[1];
    state[slots[1]] = original[2];
    state[slots[2]] = original[0];
    let error = solve_facelets(&format_facelets(&state), 22).unwrap_err();
    assert!(error.contains("corner twists add up to"), "{error}");

    // One edge flipped.
    let mut state = solved();
    let slots = GEO.edge_facelet[0];
    state.swap(slots[0], slots[1]);
    let error = solve_facelets(&format_facelets(&state), 22).unwrap_err();
    assert!(error.contains("edges are flipped"), "{error}");

    // Two corners swapped: odd parity against an even edge permutation.
    let mut state = solved();
    for j in 0..3 {
        let a = GEO.corner_facelet[0][j];
        let b = GEO.corner_facelet[1][j];
        state.swap(a, b);
    }
    let error = solve_facelets(&format_facelets(&state), 22).unwrap_err();
    assert!(error.contains("parity"), "{error}");

    // Unpainted stickers.
    let mut state = solved();
    state[3] = UNKNOWN;
    let error = solve_facelets(&format_facelets(&state), 22).unwrap_err();
    assert!(error.contains("have not been painted"), "{error}");
}

#[test]
fn validation_error_accepts_solvable_cubes_and_explains_the_rest() {
    // A solved cube and real scrambles are fine.
    assert_eq!(validation_error(&solved_string()), None);
    let mut rng = Rng(0x5EED_1234_5678_9ABC);
    for _ in 0..20 {
        let mut state = solved();
        for _ in 0..25 {
            state = apply_move(&state, rng.below(NUM_MOVES));
        }
        assert_eq!(validation_error(&format_facelets(&state)), None);
    }

    // Every category of problem is reported, and none of them panics.
    let mut unpainted = solved();
    unpainted[7] = UNKNOWN;
    let message = validation_error(&format_facelets(&unpainted)).unwrap();
    assert!(message.contains("have not been painted yet"), "{message}");
    assert!(message.contains("Clear stickers"), "{message}");

    let mut counts = solved();
    counts[0] = R;
    let message = validation_error(&format_facelets(&counts)).unwrap();
    assert!(message.contains("needs exactly 9"), "{message}");
    assert!(
        message.contains("is painted where this one belongs"),
        "{message}"
    );

    // Too many of one colour (checked after U, so U stays at nine).
    let mut counts = solved();
    counts[18] = R;
    let message = validation_error(&format_facelets(&counts)).unwrap();
    assert!(message.contains("needs exactly 9"), "{message}");
    assert!(message.contains("belongs on another face"), "{message}");

    let mut centre = solved();
    centre[4] = R;
    centre[13] = U;
    let message = validation_error(&format_facelets(&centre)).unwrap();
    assert!(message.contains("centre of the U face"), "{message}");
    assert!(message.contains("Solved cube"), "{message}");

    let mut twisted = solved();
    let slots = GEO.corner_facelet[0];
    let original = [twisted[slots[0]], twisted[slots[1]], twisted[slots[2]]];
    twisted[slots[0]] = original[1];
    twisted[slots[1]] = original[2];
    twisted[slots[2]] = original[0];
    let message = validation_error(&format_facelets(&twisted)).unwrap();
    assert!(message.contains("corner twists add up to"), "{message}");
    assert!(message.contains("Check the three stickers"), "{message}");

    let mut flipped = solved();
    let slots = GEO.edge_facelet[0];
    flipped.swap(slots[0], slots[1]);
    let message = validation_error(&format_facelets(&flipped)).unwrap();
    assert!(message.contains("edges are flipped"), "{message}");
    assert!(message.contains("Check the two stickers"), "{message}");

    let mut swapped = solved();
    for j in 0..3 {
        let a = GEO.corner_facelet[0][j];
        let b = GEO.corner_facelet[1][j];
        swapped.swap(a, b);
    }
    let message = validation_error(&format_facelets(&swapped)).unwrap();
    assert!(message.contains("parity"), "{message}");
    assert!(message.contains("Swap two stickers"), "{message}");

    // Wrong length and unknown characters are reported too.
    assert!(validation_error("UUU")
        .unwrap()
        .contains("expected 54 characters"));
    assert!(validation_error(&"X".repeat(54))
        .unwrap()
        .contains("unexpected character"));
}

/// Whatever the input, validation must answer instead of panicking, and it
/// must agree with the solver about whether a cube can be solved.
#[test]
fn validation_never_panics_and_agrees_with_the_solver() {
    let mut rng = Rng(0xC0FF_EE00_1234_5678);
    let letters = ['U', 'R', 'F', 'D', 'L', 'B', '?'];
    for _ in 0..400 {
        let random: String = (0..NUM_FACELETS)
            .map(|_| letters[rng.below(letters.len())])
            .collect();
        let validation = validation_error(&random);
        let solve = solve_facelets(&random, 20);
        assert_eq!(
            validation.is_none(),
            solve.is_ok(),
            "validation and solve disagree about '{random}'"
        );
        if let (Some(a), Err(b)) = (&validation, &solve) {
            assert_eq!(a, b, "different messages for '{random}'");
        }
    }

    // Short and over-long strings must be safe as well.
    for length in 0..120 {
        let text: String = std::iter::repeat('U').take(length).collect();
        let _ = validation_error(&text);
    }
}

#[test]
fn move_parsing_and_composition() {
    assert_eq!(parse_move("R"), Some(3));
    assert_eq!(parse_move("R2"), Some(4));
    assert_eq!(parse_move("R'"), Some(5));
    assert_eq!(parse_move("u"), None);
    assert_eq!(parse_move("R3"), None);

    // "R R'" is the identity, "R2" is the same as "R R".
    let perm = compose_perm(&parse_moves("R R'").unwrap());
    assert_eq!(perm, std::array::from_fn(|i| i as u8));
    let double = compose_perm(&parse_moves("R2").unwrap());
    let twice = compose_perm(&parse_moves("R R").unwrap());
    assert_eq!(double, twice);

    // Applying a move and its inverse cancels out; six "sexy moves" are
    // also the identity.
    let start = format_facelets(&solved());
    let sexy = "R U R' U' ".repeat(6);
    assert_eq!(apply_moves(&start, &sexy).unwrap(), start);
    assert_eq!(apply_moves(&start, "R R R R").unwrap(), start);
    assert_ne!(apply_moves(&start, "R U R' U'").unwrap(), start);
    assert!(apply_moves(&start, "R U Q").is_err());
}

#[test]
#[ignore = "statistics, run with --ignored --nocapture"]
fn statistics() {
    use std::time::Instant;
    let t0 = Instant::now();
    let _ = &*TABLES;
    println!("table build: {:?}", t0.elapsed());

    let mut rng = Rng(0x1234_5678_9ABC_DEF0);
    let mut times = Vec::new();
    let mut lengths = Vec::new();
    for _ in 0..25 {
        let mut state = solved();
        for _ in 0..25 {
            state = apply_move(&state, rng.below(NUM_MOVES));
        }
        let text = format_facelets(&state);
        let t = Instant::now();
        let answer = solve_facelets(&text, 22).unwrap();
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        lengths.push(answer.split_whitespace().count());
        let mut check = state;
        for m in parse_moves(&answer).unwrap() {
            check = apply_move(&check, m);
        }
        assert_eq!(check, solved());
    }
    lengths.sort_unstable();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "solutions: min {} median {} max {}  |  time ms: median {:.1} max {:.1}",
        lengths[0],
        lengths[lengths.len() / 2],
        lengths[lengths.len() - 1],
        times[times.len() / 2],
        times[times.len() - 1],
    );
}

#[test]
fn phase2_moves_stay_inside_the_subgroup() {
    for &m in PHASE2_MOVES.iter() {
        let face = move_face(m);
        let variant = move_variant(m);
        if variant != 1 {
            assert!(
                face == U as usize || face == D as usize,
                "{} leaves G1",
                move_name(m)
            );
        }
        // None of them may twist a corner, flip an edge or disturb the slice.
        let moved = apply_move(&solved(), m);
        let cubies = CubieState::read(&moved).unwrap();
        assert_eq!(cubies.twist(), 0, "{} twists corners", move_name(m));
        assert_eq!(cubies.flip(), 0, "{} flips edges", move_name(m));
        assert_eq!(cubies.slice() as usize, TABLES.slice_goal);
    }
}
