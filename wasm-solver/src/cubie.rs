//! The cubie level view of a cube: which corner or edge sits in which slot, and
//! how it is twisted or flipped.
//!
//! [`CubieState::read`] is the only place in the crate where sticker *colours*
//! are interpreted; everything else works on permutations and orientations.

use crate::combinatorics::{perm_index, perm_parity, subset_index};
use crate::geometry::{D, FACE_CHARS, GEO, NUM_FACELETS, U};

/// Names of the eight corner slots, used in error messages.
pub(crate) const CORNER_SLOT_NAME: [&str; 8] =
    ["URF", "UFL", "ULB", "UBR", "DFR", "DLF", "DBL", "DRB"];
/// Names of the twelve edge slots, used in error messages.
pub(crate) const EDGE_SLOT_NAME: [&str; 12] = [
    "UR", "UF", "UL", "UB", "DR", "DF", "DL", "DB", "FR", "FL", "BL", "BR",
];

/// The cubie level description of a cube state.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CubieState {
    /// `cp[i]` = corner cubie sitting in corner slot `i`.
    pub cp: [u8; 8],
    /// `co[i]` = slot of corner `i` that carries the U/D coloured sticker.
    pub co: [u8; 8],
    /// `ep[i]` = edge cubie sitting in edge slot `i`.
    pub ep: [u8; 12],
    /// `eo[i]` = `1` when the two stickers of edge `i` are swapped.
    pub eo: [u8; 12],
}

impl CubieState {
    /// Read the stickers of a cube state into corner/edge permutations
    /// **without** checking the physical constraints. Used while generating the
    /// coordinate tables, where the canonical states are deliberately not
    /// physically reachable.
    pub fn read(state: &[u8; NUM_FACELETS]) -> Result<Self, String> {
        let geo = &*GEO;
        let mut cp = [0u8; 8];
        let mut co = [0u8; 8];
        for i in 0..8 {
            let slots = geo.corner_facelet[i];
            let mut ori = None;
            for (k, slot) in slots.iter().enumerate() {
                let c = state[*slot];
                if c == U || c == D {
                    ori = Some(k);
                    break;
                }
            }
            let Some(ori) = ori else {
                return Err(format!(
                    "Invalid cube: corner {} does not carry exactly one white or yellow \
                     sticker. Check the three stickers around that corner.",
                    CORNER_SLOT_NAME[i]
                ));
            };
            let c1 = state[slots[(ori + 1) % 3]];
            let c2 = state[slots[(ori + 2) % 3]];
            let mut cubie = None;
            for (j, colors) in geo.corner_color.iter().enumerate() {
                if colors[1] == c1 && colors[2] == c2 {
                    cubie = Some(j);
                    break;
                }
            }
            match cubie {
                Some(j) => {
                    cp[i] = j as u8;
                    co[i] = ori as u8;
                }
                None => {
                    return Err(format!(
                        "Invalid cube: corner {} holds the stickers {}/{}/{}, which do not \
                         form a real corner cubie. Those three colours never meet on a cube, \
                         so at least one sticker is on the wrong face.",
                        CORNER_SLOT_NAME[i],
                        sticker_char(state[slots[0]]),
                        sticker_char(state[slots[1]]),
                        sticker_char(state[slots[2]])
                    ))
                }
            }
        }

        let mut ep = [0u8; 12];
        let mut eo = [0u8; 12];
        for i in 0..12 {
            let slots = geo.edge_facelet[i];
            let a = state[slots[0]];
            let b = state[slots[1]];
            let mut cubie = None;
            for (j, colors) in geo.edge_color.iter().enumerate() {
                if colors[0] == a && colors[1] == b {
                    cubie = Some((j, 0u8));
                    break;
                }
                if colors[0] == b && colors[1] == a {
                    cubie = Some((j, 1u8));
                    break;
                }
            }
            match cubie {
                Some((j, flipped)) => {
                    ep[i] = j as u8;
                    eo[i] = flipped;
                }
                None => {
                    return Err(format!(
                        "Invalid cube: edge {} holds the stickers {}/{}, which do not form a \
                         real edge cubie. Those two colours are not neighbours on a cube, so at \
                         least one sticker is on the wrong face.",
                        EDGE_SLOT_NAME[i],
                        sticker_char(state[slots[0]]),
                        sticker_char(state[slots[1]])
                    ))
                }
            }
        }

        let cubies = CubieState { cp, co, ep, eo };
        Ok(cubies)
    }

    /// Read a cube state and check that it is physically reachable.
    ///
    /// This is the only place where sticker *colours* are interpreted;
    /// everything else in this file works on the cubie level.
    pub fn from_facelets(state: &[u8; NUM_FACELETS]) -> Result<Self, String> {
        let cubies = Self::read(state)?;
        cubies.validate()?;
        Ok(cubies)
    }

    /// All physical constraints that decide whether the cube can be solved.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen_corners = [false; 8];
        for c in self.cp {
            if seen_corners[c as usize] {
                return Err(format!(
                    "Invalid cube: corner cubie {} appears in more than one slot, so two corners \
                     are painted with the same colours. Check every corner of that colour.",
                    CORNER_SLOT_NAME[c as usize]
                ));
            }
            seen_corners[c as usize] = true;
        }
        let mut seen_edges = [false; 12];
        for e in self.ep {
            if seen_edges[e as usize] {
                return Err(format!(
                    "Invalid cube: edge cubie {} appears in more than one slot, so two edges are \
                     painted with the same colours. Check every edge of that colour.",
                    EDGE_SLOT_NAME[e as usize]
                ));
            }
            seen_edges[e as usize] = true;
        }

        let twist: u32 = self.co.iter().map(|v| *v as u32).sum();
        if twist % 3 != 0 {
            return Err(format!(
                "Invalid cube: the corner twists add up to {twist}, which is not a multiple of \
                 3 - a single corner must be twisted the wrong way. Check the three stickers of \
                 every corner, or press \"Solved cube\" to start again."
            ));
        }
        let flips: u32 = self.eo.iter().map(|v| *v as u32).sum();
        if flips % 2 != 0 {
            return Err(format!(
                "Invalid cube: {flips} edges are flipped, which is an odd number - a single \
                 edge must be flipped. Check the two stickers of every edge, or press \
                 \"Solved cube\" to start again."
            ));
        }
        if perm_parity(&self.cp) != perm_parity(&self.ep) {
            return Err(
                "Invalid cube: the corner and the edge permutation have different parity, which \
                 is impossible on a physical cube. Swap two stickers of the same kind (two \
                 corners or two edges) to fix it, or press \"Solved cube\" to start again."
                    .to_string(),
            );
        }
        Ok(())
    }

    /// Is this the solved cube?
    pub fn is_solved(&self) -> bool {
        self.cp == [0, 1, 2, 3, 4, 5, 6, 7]
            && self.co == [0; 8]
            && self.ep == [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
            && self.eo == [0; 12]
    }

    /// Corner twist coordinate in `0..2187`.
    pub fn twist(&self) -> u16 {
        let mut value = 0u16;
        for i in (0..7).rev() {
            value = value * 3 + self.co[i] as u16;
        }
        value
    }

    /// Edge flip coordinate in `0..2048`.
    pub fn flip(&self) -> u16 {
        let mut value = 0u16;
        for i in (0..11).rev() {
            value = value * 2 + self.eo[i] as u16;
        }
        value
    }

    /// Index of the set of slots holding the four middle layer edges, `0..495`.
    pub fn slice(&self) -> u16 {
        let mut set = [0usize; 4];
        let mut n = 0;
        for (slot, cubie) in self.ep.iter().enumerate() {
            if *cubie >= 8 {
                set[n] = slot;
                n += 1;
            }
        }
        subset_index(&set) as u16
    }

    /// Lexicographic index of the corner permutation, `0..40320`.
    pub fn corner_perm(&self) -> u32 {
        perm_index(&self.cp)
    }

    /// Lexicographic index of the U/D edge permutation, `0..40320`.
    pub fn ud_edge_perm(&self) -> u32 {
        perm_index(&self.ep[0..8])
    }

    /// Lexicographic index of the middle layer edge permutation, `0..24`.
    pub fn slice_perm(&self) -> u32 {
        let local: Vec<u8> = self.ep[8..12].iter().map(|v| v - 8).collect();
        perm_index(&local)
    }
}

/// Printable form of a sticker, used in error messages.
fn sticker_char(sticker: u8) -> char {
    if sticker < 6 {
        FACE_CHARS[sticker as usize]
    } else {
        '?'
    }
}
