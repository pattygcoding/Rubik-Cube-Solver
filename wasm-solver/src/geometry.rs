//! Cube geometry: the integer grid, the 54 stickers and the 20 cubie slots.
//!
//! Everything else in the crate is derived from what is defined here: the
//! position and outward normal of every sticker, and the corner/edge slots used
//! to read a cube state.

use std::sync::LazyLock;

/// Number of faces of a cube.
pub const NUM_FACES: usize = 6;
/// Number of stickers (facelets) of a cube.
pub const NUM_FACELETS: usize = 54;

/// Face identifiers. The order (and the letters used to write a facelet string)
/// matches the classic unfolded layout used by Kociemba's `FaceCube`:
/// `U = 0, R = 1, F = 2, D = 3, L = 4, B = 5`.
pub const U: u8 = 0;
/// Right face.
pub const R: u8 = 1;
/// Front face.
pub const F: u8 = 2;
/// Down face.
pub const D: u8 = 3;
/// Left face.
pub const L: u8 = 4;
/// Back face.
pub const B: u8 = 5;

/// The six colour letters, indexed by face.
pub const FACE_CHARS: [char; NUM_FACES] = ['U', 'R', 'F', 'D', 'L', 'B'];

/// Sticker value the UI uses while a sticker has not been painted yet.
pub const UNKNOWN: u8 = 6;

/// A point on the integer grid. `x` grows to the right, `y` grows upwards and
/// `z` grows towards the viewer, matching three.js' right-handed frame.
type Vec3 = [i8; 3];

pub(crate) const AXIS_X: usize = 0;
pub(crate) const AXIS_Y: usize = 1;
pub(crate) const AXIS_Z: usize = 2;

const PX: Vec3 = [1, 0, 0];
const NX: Vec3 = [-1, 0, 0];
const PY: Vec3 = [0, 1, 0];
const NY: Vec3 = [0, -1, 0];
const PZ: Vec3 = [0, 0, 1];
const NZ: Vec3 = [0, 0, -1];

fn is_positive_x(n: Vec3) -> bool {
    n[0] == 1 && n[1] == 0 && n[2] == 0
}
fn is_negative_x(n: Vec3) -> bool {
    n[0] == -1 && n[1] == 0 && n[2] == 0
}
fn is_positive_y(n: Vec3) -> bool {
    n[0] == 0 && n[1] == 1 && n[2] == 0
}
fn is_negative_y(n: Vec3) -> bool {
    n[0] == 0 && n[1] == -1 && n[2] == 0
}
fn is_positive_z(n: Vec3) -> bool {
    n[0] == 0 && n[1] == 0 && n[2] == 1
}

/// `(position, sticker normals in slot order)` for the eight corners, in cubie
/// order `URF, UFL, ULB, UBR, DFR, DLF, DBL, DRB`.
pub(crate) const CORNERS: [(Vec3, [Vec3; 3]); 8] = [
    ([1, 1, 1], [PY, PX, PZ]),
    ([-1, 1, 1], [PY, PZ, NX]),
    ([-1, 1, -1], [PY, NX, NZ]),
    ([1, 1, -1], [PY, NZ, PX]),
    ([1, -1, 1], [NY, PZ, PX]),
    ([-1, -1, 1], [NY, NX, PZ]),
    ([-1, -1, -1], [NY, NZ, NX]),
    ([1, -1, -1], [NY, PX, NZ]),
];

/// `(position, sticker normals in slot order)` for the twelve edges, in cubie
/// order `UR, UF, UL, UB, DR, DF, DL, DB, FR, FL, BL, BR`.
pub(crate) const EDGES: [(Vec3, [Vec3; 2]); 12] = [
    ([1, 1, 0], [PY, PX]),
    ([0, 1, 1], [PY, PZ]),
    ([-1, 1, 0], [PY, NX]),
    ([0, 1, -1], [PY, NZ]),
    ([1, -1, 0], [NY, PX]),
    ([0, -1, 1], [NY, PZ]),
    ([-1, -1, 0], [NY, NX]),
    ([0, -1, -1], [NY, NZ]),
    ([1, 0, 1], [PZ, PX]),
    ([-1, 0, 1], [PZ, NX]),
    ([-1, 0, -1], [NZ, NX]),
    ([1, 0, -1], [NZ, PX]),
];

/// Edge slots of the middle layer, i.e. the home of the four UD-slice edges.
pub(crate) const SLICE_SLOTS: [usize; 4] = [8, 9, 10, 11];

/// Rotate `v` by `quarters` right-handed quarter turns around `axis`.
pub(crate) fn rotate(v: Vec3, axis: usize, quarters: i32) -> Vec3 {
    let mut out = v;
    for _ in 0..quarters.rem_euclid(4) {
        out = match axis {
            AXIS_X => [out[0], -out[2], out[1]],
            AXIS_Y => [out[2], out[1], -out[0]],
            _ => [-out[1], out[0], out[2]],
        };
    }
    out
}

/// Index of the grid point `p` inside the `3 x 3 x 3` block, in `0..27`.
fn pos_index(p: Vec3) -> usize {
    ((p[0] + 1) as usize) * 9 + ((p[1] + 1) as usize) * 3 + ((p[2] + 1) as usize)
}

/// Index of a signed unit normal in `0..6`, ordered `+x, -x, +y, -y, +z, -z`.
fn normal_index(n: Vec3) -> usize {
    if is_positive_x(n) {
        0
    } else if is_negative_x(n) {
        1
    } else if is_positive_y(n) {
        2
    } else if is_negative_y(n) {
        3
    } else if is_positive_z(n) {
        4
    } else {
        5
    }
}

/// The face a signed normal points at.
fn face_of_normal(n: Vec3) -> u8 {
    if is_positive_x(n) {
        R
    } else if is_negative_x(n) {
        L
    } else if is_positive_y(n) {
        U
    } else if is_negative_y(n) {
        D
    } else if is_positive_z(n) {
        F
    } else {
        B
    }
}

/// Position and outward normal of the sticker at `(face, row, column)` of the
/// classic unfolded net (row `0` is the top row of the face as drawn).
///
/// The net is laid out as `L F R B` around the middle band, with `U` above `F`
/// and `D` below it. Columns therefore run left-to-right as `x: -1 -> +1` for
/// `U`/`F`/`D` (mirrored for `B`), the horizontal band runs back-to-front and
/// `y` runs top-to-bottom.
fn sticker_geometry(face: u8, row: usize, col: usize) -> (Vec3, Vec3) {
    let c = col as i8 - 1; // -1 .. 1, left to right
    let r = 1 - row as i8; //  1 .. -1, top to bottom
    match face {
        U => ([c, 1, -r], PY),
        R => ([1, r, -c], PX),
        F => ([c, r, 1], PZ),
        D => ([c, -1, r], NY),
        L => ([-1, r, c], NX),
        _ => ([-c, r, -1], NZ),
    }
}

// ---------------------------------------------------------------------------
// Derived geometry
// ---------------------------------------------------------------------------

/// Everything that can be derived from the cube's geometry.
pub(crate) struct Geometry {
    /// `(position, normal)` of each of the 54 stickers, indexed by facelet.
    pub(crate) stickers: [(Vec3, Vec3); NUM_FACELETS],
    /// `sticker_at[pos_index * 6 + normal_index]`, or `usize::MAX` when the
    /// grid point carries no sticker for that normal.
    pub(crate) sticker_at: [usize; 27 * 6],
    /// The solved cube, i.e. the reference facelet string.
    pub(crate) solved: [u8; NUM_FACELETS],
    /// Facelet indices of the three stickers of every corner, in slot order.
    pub(crate) corner_facelet: [[usize; 3]; 8],
    /// Face colours of the three stickers of every corner, in slot order.
    pub(crate) corner_color: [[u8; 3]; 8],
    /// Facelet indices of the two stickers of every edge, in slot order.
    pub(crate) edge_facelet: [[usize; 2]; 12],
    /// Face colours of the two stickers of every edge, in slot order.
    pub(crate) edge_color: [[u8; 2]; 12],
}

impl Geometry {
    fn new() -> Self {
        let mut stickers = [([0, 0, 0], [0, 0, 0]); NUM_FACELETS];
        let mut sticker_at = [usize::MAX; 27 * 6];
        let mut solved = [0u8; NUM_FACELETS];
        for face in 0..NUM_FACES {
            for row in 0..3 {
                for col in 0..3 {
                    let idx = face * 9 + row * 3 + col;
                    let (p, n) = sticker_geometry(face as u8, row, col);
                    stickers[idx] = (p, n);
                    sticker_at[pos_index(p) * 6 + normal_index(n)] = idx;
                    solved[idx] = face as u8;
                }
            }
        }

        let mut corner_facelet = [[0usize; 3]; 8];
        let mut corner_color = [[0u8; 3]; 8];
        for (i, (p, normals)) in CORNERS.iter().enumerate() {
            for (j, n) in normals.iter().enumerate() {
                corner_facelet[i][j] = sticker_at[pos_index(*p) * 6 + normal_index(*n)];
                corner_color[i][j] = face_of_normal(*n);
            }
        }

        let mut edge_facelet = [[0usize; 2]; 12];
        let mut edge_color = [[0u8; 2]; 12];
        for (i, (p, normals)) in EDGES.iter().enumerate() {
            for (j, n) in normals.iter().enumerate() {
                edge_facelet[i][j] = sticker_at[pos_index(*p) * 6 + normal_index(*n)];
                edge_color[i][j] = face_of_normal(*n);
            }
        }

        let geo = Geometry {
            stickers,
            sticker_at,
            solved,
            corner_facelet,
            corner_color,
            edge_facelet,
            edge_color,
        };
        geo.assert_complete();
        geo
    }

    /// Facelet index of the sticker sitting on grid point `p` facing `n`.
    pub(crate) fn index_of(&self, p: Vec3, n: Vec3) -> usize {
        self.sticker_at[pos_index(p) * 6 + normal_index(n)]
    }

    /// Panics if the derivation above ever produced an inconsistent model.
    /// These checks are cheap and run once per process.
    fn assert_complete(&self) {
        for (i, &idx) in self.sticker_at.iter().enumerate() {
            assert!(
                idx == usize::MAX || idx < NUM_FACELETS,
                "bad sticker slot {i}"
            );
        }
        for i in 0..8 {
            for j in 0..3 {
                assert_ne!(
                    self.corner_facelet[i][j],
                    usize::MAX,
                    "corner {i} slot {j} has no sticker"
                );
            }
        }
        for i in 0..12 {
            for j in 0..2 {
                assert_ne!(
                    self.edge_facelet[i][j],
                    usize::MAX,
                    "edge {i} slot {j} has no sticker"
                );
            }
        }
        // The four middle-layer edges must not touch the U or D face.
        for i in 8..12 {
            for j in 0..2 {
                assert!(
                    self.edge_color[i][j] != U && self.edge_color[i][j] != D,
                    "slice edge {i} is not a middle-layer edge"
                );
            }
        }
    }
}

/// The single, lazily-built geometry of the cube.
pub(crate) static GEO: LazyLock<Geometry> = LazyLock::new(Geometry::new);
