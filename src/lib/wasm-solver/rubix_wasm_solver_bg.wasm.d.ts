/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const js_apply_moves: (a: number, b: number, c: number, d: number) => [number, number, number, number];
export const js_is_solved: (a: number, b: number) => [number, number, number];
export const js_move_indices: (a: number, b: number) => [number, number, number, number];
export const js_validation_error: (a: number, b: number) => [number, number];
export const move_index: (a: number, b: number) => number;
export const move_turn: (a: number) => [number, number];
export const num_moves: () => number;
export const solve_cube: (a: number, b: number, c: number) => [number, number, number, number];
export const solved_facelets: () => [number, number];
export const solver_version: () => [number, number];
export const start: () => void;
export const table_bytes: () => number;
export const warm_up: () => void;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_externrefs: WebAssembly.Table;
export const __externref_table_dealloc: (a: number) => void;
export const __wbindgen_start: () => void;
