# Rubik's Cube Solver - the Rust/WebAssembly two-phase engine

The engine behind the arcade's [Rubik's Cube Solver](https://arcade.pattygcoding.com/rubix):
a Herbert Kociemba style two-phase search, written in Rust and compiled to
WebAssembly with `wasm-bindgen`.

**This repository is the solver only.** It used to ship a SvelteKit UI; that UI now
lives in the [arcade](https://github.com/pattygcoding/Arcade-PG-Website)
(`src/app/pages/rubix`), ported to Angular, and it loads the WebAssembly module from
its own `public/wasm/`. What is left here is the crate: the solver, its lookup
tables and its tests.

## Quick start

```bash
cargo test --manifest-path wasm-solver/Cargo.toml --release   # 17 tests, about a second
```

Building the WebAssembly module the arcade serves:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
wasm-pack build wasm-solver --target web --release --no-pack \
  --out-dir pkg --out-name rubix_wasm_solver
```

Then copy the two files the browser actually fetches into the arcade, keeping the
names `wasm-pack` gives them: the glue resolves its binary relative to itself with
`new URL('rubix_wasm_solver_bg.wasm', import.meta.url)`, so the pair has to sit in
one directory under those names.

```bash
cp pkg/rubix_wasm_solver.js      ../Arcade-PG-Website/public/wasm/
cp pkg/rubix_wasm_solver_bg.wasm ../Arcade-PG-Website/public/wasm/
```

The two generated `.d.ts` files are not copied: the arcade declares the handful of
exports it uses in `src/app/pages/rubix/rubix-solver.ts`.

### Requirements

| Tool | Version used |
| --- | --- |
| Rust | 1.98 (`rustup target add wasm32-unknown-unknown` for the wasm build) |
| wasm-pack | 0.15 |

`--target web` produces an ES module whose default export is `init()`, which the
arcade awaits before touching any export:

```ts
const module = await import('/wasm/rubix_wasm_solver.js');
await module.default();                          // instantiate the .wasm
module.warm_up();                                // build the lookup tables (~250 ms)
const moves = module.solve_cube(facelets, 22);    // "R U R' U' F2 ..."
```

## The solver

The crate is split by responsibility; `lib.rs` is only the crate root that declares
the modules and re-exports the public API.

| File | Contents |
| --- | --- |
| `wasm-solver/src/lib.rs` | crate docs, module declarations, public re-exports |
| `wasm-solver/src/geometry.rs` | faces, the integer grid, the 54 stickers and the 20 cubie slots |
| `wasm-solver/src/facelets.rs` | facelet string parsing, formatting, permutations |
| `wasm-solver/src/combinatorics.rs` | permutation and subset ranking, parity |
| `wasm-solver/src/cubie.rs` | the cubie level view of a cube (which cubie is where, twist/flip) |
| `wasm-solver/src/moves.rs` | move encoding, face turns, Singmaster notation, `apply_moves` |
| `wasm-solver/src/coordinates.rs` | phase coordinates and the canonical state each value stands for |
| `wasm-solver/src/tables.rs` | the move and pruning tables (the only heavy part) |
| `wasm-solver/src/search.rs` | the two-phase iterative deepening search |
| `wasm-solver/src/solver.rs` | validation and the solve entry points |
| `wasm-solver/src/wasm_api.rs` | the thin `#[wasm_bindgen]` wrappers |
| `wasm-solver/src/tests.rs` | the test suite for the crate as a whole |

The search itself:

* **Phase 1** drives the cube into the subgroup `G1 = <U, D, R2, L2, F2, B2>`: all
  corners untwisted, all edges unflipped and the four middle layer edges back in the
  slice. It is guided by the `twist (2187) x slice (495)` and
  `flip (2048) x slice (495)` pruning tables.
* **Phase 2** finishes the cube inside `G1` using `U, U2, U', D, D2, D', R2, L2, F2, B2`,
  guided by `corner permutation (40320) x slice permutation (24)` and
  `UD edge permutation (40320) x slice permutation (24)`.
* The phase-1 depth is grown first and the phase-2 depth is capped by the requested
  `max_depth`, then falls back to the 30 move bound which always succeeds, so the UI
  always gets an answer.

Every lookup table is *derived* at runtime from the cube's geometry: the position and
normal of all 54 stickers, the eight corner slots and the twelve edge slots. Face
turns are obtained by rotating that geometry, and coordinate transitions come from
small canonical states (twist one corner in place, flip one edge in place, permute
cubies). There is no hand written lookup table anywhere, and the unit tests compare
the derived tables against the classic unfolded net layout, so a typo cannot produce
a solver that disagrees with itself.

### API

| Export | Purpose |
| --- | --- |
| `solve_cube(facelets, max_depth)` | `"UUU...BBB"` to `"R U R' U' F2"`, or `""` when already solved |
| `solved_facelets()` | the reference facelet string |
| `js_apply_moves(facelets, moves)` | apply a move list to a facelet string |
| `js_move_indices(moves)` | `result[displayed] = original`, so the UI can paint while a solution is shown |
| `js_is_solved(facelets)` | cheap "is this cube solved" check |
| `js_validation_error(facelets)` | the first thing that stops the cube from being solved, or `undefined` when it is solvable (no search, so it is cheap enough to call on every paint) |
| `move_index(token)` / `move_turn(index)` | Singmaster token to axis, layer and quarter turns, so a 3D view can never disagree with the solver about which way a face turns |
| `warm_up()` | build every table up front |
| `table_bytes()` | bytes of lookup tables currently held (about 5.5 MB) |

Facelet strings are the concatenation of the six faces in the order `U, R, F, D, L, B`,
nine stickers each, using the letters `U R F D L B` as colour codes. Every export that
takes one accepts the same layout the classic Kociemba net draws, where the top row of
`U` is the back of the cube and the top row of `D` is the front, so the cross folds up
without mirroring anything.

## Invalid cubes

`solve_cube` and `js_validation_error` share one code path (`checked_state`), so a UI
and the solver can never disagree about whether a cube is usable. Both return an
informative error instead of panicking for:

| Case | Reported as |
| --- | --- |
| not 54 characters, or unknown characters | the offending position |
| a colour that does not appear exactly nine times | which colour, and how many times |
| a displaced centre sticker | which centre moved |
| an impossible cubie | which corner or edge could not exist |
| a single twisted corner or flipped edge | the twist/flip that does not add up |
| mismatched permutation parity | the parity mismatch |

`js_validation_error` never searches, so it is cheap enough to call on every paint:
the arcade runs it after each sticker the user clicks and explains the problem while
they are still painting instead of waiting for a failed solve.

## Tests

```bash
cargo test --manifest-path wasm-solver/Cargo.toml --release
```

The suite solves batches of random scrambles and verifies every answer by replaying
it, and checks that `validation_error` and `solve_cube` always agree, including on 400
random 54-character strings built from `U R F D L B ?`.

```bash
cargo test --release statistics -- --ignored --nocapture
```

prints table build time and solution length statistics: median about 20 moves and
roughly 10 ms per solve on a desktop CPU, with the lookup tables taking about 250 ms
to build in the browser.

The arcade's own tests (the Angular page that drives this module) live with it, in
`Arcade-PG-Website/src/app/pages/rubix/rubix.spec.ts`.

## Known limitations

* This is a two-phase search, not an optimal solver: answers are usually 20-22 moves.
* `max_depth` is honoured when possible. If nothing fits, the engine returns the
  shortest answer it can find under the 30 move bound, and the UI says so.
* The tables are rebuilt on every page load (about 250 ms). Committing them would cost
  megabytes and would have to stay in step with the geometry code, which is derived
  from that same geometry.

