/* tslint:disable */
/* eslint-disable */

/**
 * Apply a Singmaster move list to a facelet string. See [`apply_moves`].
 */
export function js_apply_moves(facelets: string, moves: string): string;

/**
 * Is the given facelet string a solved cube? See [`is_solved`].
 */
export function js_is_solved(facelets: string): boolean;

/**
 * Map every sticker of the cube after `moves` back to the original sticker.
 * See [`move_indices`].
 */
export function js_move_indices(moves: string): Uint8Array;

/**
 * The first thing that stops this cube from being solved, or `undefined` when
 * it is solvable.
 *
 * This runs the same checks as [`solve_cube`] but never searches, so the UI can
 * explain the problem while the user is still painting instead of waiting for a
 * failed solve. Unpainted stickers are accepted by the parser and reported by
 * [`validate_stickers`], which keeps this useful for a half painted cube.
 */
export function js_validation_error(facelets: string): string | undefined;

/**
 * Turn a Singmaster token (`"R"`, `"R2"`, `"U'"`) into a move index, or
 * `undefined` when the token is not a move. The UI uses this to translate the
 * solver's answer into animations.
 */
export function move_index(token: string): number | undefined;

/**
 * Geometry of a move, as `[axis, layer, quarters]`.
 *
 * `axis` is `0` for x, `1` for y and `2` for z, `layer` is `+1` or `-1` and
 * `quarters` counts *right-handed* quarter turns around the positive axis.
 * The 3D view rotates the layer mesh group by `quarters * 90Â°` so it can never
 * disagree with the solver about which way a face turns.
 */
export function move_turn(move_index: number): Int8Array;

/**
 * Number of moves the solver understands (18: six faces, three variants).
 */
export function num_moves(): number;

/**
 * Solve a cube given as a 54 character UDSLRF facelet string.
 *
 * `facelets` is the concatenation of the six faces in the order
 * `U, R, F, D, L, B`, nine stickers each, using the letters `U R F D L B` as
 * colour codes, e.g. `"UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB"`.
 * `max_depth` is the longest solution that is acceptable (0 picks a sane
 * default). The result is a space separated Singmaster move list such as
 * `"R U R' U' F2"`, or an empty string when the cube is already solved.
 *
 * Every kind of unusable input produces an informative error instead of a
 * panic: wrong length, unknown characters, wrong sticker counts, misplaced
 * centres, impossible cubies, a single twisted corner, a single flipped edge or
 * mismatched permutation parity.
 */
export function solve_cube(facelets: string, max_depth: number): string;

/**
 * The solved facelet string, which is also the reference every other function
 * works against.
 */
export function solved_facelets(): string;

/**
 * Version of the solver, surfaced in the UI footer.
 */
export function solver_version(): string;

/**
 * Install a panic hook that forwards Rust panics to the browser console.
 */
export function start(): void;

/**
 * Number of bytes of lookup tables currently held, for diagnostics.
 */
export function table_bytes(): number;

/**
 * Build every lookup table up front.
 *
 * Table generation is the only slow part of this module (roughly 6 MB of move
 * and pruning tables are filled with a breadth-first search). Calling this
 * while the user is still painting stickers makes the first "Solve" click feel
 * instant.
 */
export function warm_up(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly js_apply_moves: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly js_is_solved: (a: number, b: number) => [number, number, number];
    readonly js_move_indices: (a: number, b: number) => [number, number, number, number];
    readonly js_validation_error: (a: number, b: number) => [number, number];
    readonly move_index: (a: number, b: number) => number;
    readonly move_turn: (a: number) => [number, number];
    readonly num_moves: () => number;
    readonly solve_cube: (a: number, b: number, c: number) => [number, number, number, number];
    readonly solved_facelets: () => [number, number];
    readonly solver_version: () => [number, number];
    readonly start: () => void;
    readonly table_bytes: () => number;
    readonly warm_up: () => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
