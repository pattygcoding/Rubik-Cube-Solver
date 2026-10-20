/**
 * The reactive cube state of the application.
 *
 * A single `CubeState` instance owns everything the UI needs: the 54 painted
 * sticker colours, the solver's answer, the playback cursor and the transition
 * the 3D view should animate. It is written with Svelte 5 runes, so any
 * component that reads a field re-renders when it changes.
 *
 * The cube the user paints (`colors`) is the *base* state. Playing a solution
 * never touches it: the displayed cube is always
 * `applyMoves(base, moves.slice(0, cursor))`, computed by the Rust engine.
 */

import { FACE_COLORS, UNKNOWN_STICKER } from './cube-geometry';
import {
	colorsToFacelets,
	describeError,
	faceletsToColors,
	getSolver,
	loadSolver,
	solvedColors
} from './solver';

/** Lifecycle of the WebAssembly engine. */
export type SolverStatus = 'loading' | 'building' | 'ready' | 'solving' | 'error';

/**
 * Why the painted cube cannot be solved.
 *
 * `incomplete` and `counts` are worked out from the stickers alone, so they are
 * known before the engine has loaded; `impossible` means the stickers are all
 * there but the cube could not exist, which only the engine can decide.
 */
export type ValidationState = 'incomplete' | 'counts' | 'impossible' | 'ok';

/** The verdict on the painted cube, plus an explanation for the user. */
export interface Validation {
	readonly state: ValidationState;
	/** `true` when the cube can be solved. */
	readonly ok: boolean;
	/** Ready to show message. */
	readonly message: string;
}

/** The kinds of change the 3D view knows how to animate. */
export type TransitionKind = 'none' | 'move' | 'undo' | 'snap';

/**
 * Description of the change that just happened.
 *
 * Views watch `id`, so they can animate a single move, play one backwards or
 * jump straight to a state.
 */
export interface CubeTransition {
	readonly id: number;
	readonly kind: TransitionKind;
	/** The move that was applied (for `move`) or reverted (for `undo`). */
	readonly move: string | null;
}

/** Facelet indices of the six centre stickers, which never move. */
export const CENTRE_STICKERS: readonly number[] = FACE_COLORS.map((_, face) => face * 9 + 4);

const isCentre = (index: number) => CENTRE_STICKERS.includes(index);

/** Number of random moves a scramble uses. */
const SCRAMBLE_LENGTH = 22;

export class CubeState {
	// -- the cube the user paints ------------------------------------------
	/** Sticker colours: 54 entries of `0..5` or `UNKNOWN_STICKER`. */
	colors = $state<number[]>(solvedColors());
	/** Colour the palette has selected. */
	activeColor = $state<number>(0);

	// -- solver -------------------------------------------------------------
	/** Solution moves in Singmaster notation. */
	moves = $state<string[]>([]);
	/** How many of `moves` are currently applied to the displayed cube. */
	cursor = $state(0);
	/** Longest solution the user accepts, passed to the engine. */
	maxDepth = $state(22);
	/** Lifecycle of the WebAssembly engine. */
	status = $state<SolverStatus>('loading');
	/** Informational note for the user (solution length, scramble, ...). */
	message = $state('');
	/** Error reported by the engine, e.g. an impossible cube. */
	error = $state('');
	/** Version string reported by the engine. */
	solverVersion = $state('');
	/** Bytes of lookup tables the engine allocated. */
	tableBytes = $state(0);
	/** Milliseconds spent building the lookup tables. */
	buildTimeMs = $state(0);

	// -- playback -----------------------------------------------------------
	/** Is the solution playing? */
	playing = $state(false);
	/** Milliseconds per move, between 100 and 800. */
	speed = $state(320);
	/** The change the 3D view should animate. */
	transition = $state<CubeTransition>({ id: 0, kind: 'none', move: null });
	/** Bumped when the engine becomes usable, so derived values recompute. */
	solverGeneration = $state(0);

	/** Timer of the playback loop. */
	private timer: ReturnType<typeof setTimeout> | null = null;

	// -- derived state ------------------------------------------------------

	/** Facelet string of the cube being painted. */
	facelets = $derived(colorsToFacelets(this.colors));

	/** Moves that are currently applied to the displayed cube. */
	displayedMoves = $derived(this.moves.slice(0, this.cursor));

	/**
	 * Facelet string of the cube as it is shown right now.
	 *
	 * While nothing is playing this is simply the painted cube, so the 3D view
	 * and the net agree even before the engine has loaded.
	 */
	displayFacelets = $derived.by(() => {
		const base = this.facelets;
		// Reading the generation makes this recompute once the engine is ready.
		void this.solverGeneration;
		if (this.cursor === 0 || this.moves.length === 0) return base;
		const solver = getSolver();
		if (!solver) return base;
		try {
			return solver.applyMoves(base, this.displayedMoves.join(' '));
		} catch (error) {
			this.error = describeError(error);
			return base;
		}
	});

	/** Sticker colours of the displayed cube. */
	displayColors = $derived(faceletsToColors(this.displayFacelets));

	/** How often each colour appears, plus the number of unpainted stickers. */
	counts = $derived.by(() => {
		const perColor = FACE_COLORS.map(() => 0);
		let unknown = 0;
		for (const color of this.colors) {
			if (color >= 0 && color < perColor.length) perColor[color] += 1;
			else unknown += 1;
		}
		return { perColor, unknown };
	});

	/** True when all 54 stickers are painted and every colour appears 9 times. */
	isComplete = $derived(
		this.counts.unknown === 0 && this.counts.perColor.every((count) => count === 9)
	);

	/**
	 * The verdict on the painted cube.
	 *
	 * Sticker level problems are found here; whether the stickers describe a cube
	 * that could physically exist is decided by the engine's validator, which is
	 * the same code `solve_cube` runs, so the two can never disagree.
	 */
	validation = $derived.by<Validation>(() => {
		// Reading the generation makes this recompute once the engine is ready.
		void this.solverGeneration;
		const { unknown, perColor } = this.counts;

		if (unknown > 0) {
			return {
				state: 'incomplete',
				ok: false,
				message: `${54 - unknown} of 54 stickers painted. ${unknown === 54 ? 'Paint the cube or press Scramble to get a random one.' : 'Keep going, or right click a sticker to clear it.'}`
			};
		}
		const wrong = FACE_COLORS.map((color, index) => ({ color, count: perColor[index] })).filter(
			(entry) => entry.count !== 9
		);
		if (wrong.length > 0) {
			const detail = wrong
				.map((entry) => `${entry.color.name} ${entry.count}${entry.count > 9 ? ' (too many)' : ' (too few)'}`)
				.join(', ');
			return {
				state: 'counts',
				ok: false,
				message: `Every colour needs exactly nine stickers: ${detail}. Fix the stickers, or press Scramble to start from a valid cube.`
			};
		}

		const solver = getSolver();
		if (!solver) {
			return {
				state: 'incomplete',
				ok: false,
				message: 'Waiting for the solving engine before this cube can be checked.'
			};
		}
		try {
			const problem = solver.validationError(this.facelets);
			if (problem) return { state: 'impossible', ok: false, message: problem };
		} catch (error) {
			return { state: 'impossible', ok: false, message: describeError(error) };
		}
		return { state: 'ok', ok: true, message: 'All 54 stickers painted and the cube is solvable.' };
	});

	/** True when the engine accepts the painted cube. */
	isSolvable = $derived(this.validation.ok);

	/** The single sentence to show the user about the cube. */
	validationMessage = $derived(this.validation.message);

	/** True when the displayed cube is solved. */
	isSolved = $derived.by(() => {
		const facelets = this.displayFacelets;
		for (let face = 0; face < 6; face++) {
			const first = facelets[face * 9];
			for (let k = 1; k < 9; k++) {
				if (facelets[face * 9 + k] !== first) return false;
			}
		}
		return true;
	});

	/**
	 * True when the *painted* cube is already solved.
	 *
	 * The transport controls are about the cube the user painted, not about how
	 * far a solution has been played, so a finished solution does not stop Play
	 * from replaying it.
	 */
	isPaintedSolved = $derived.by(() => {
		const facelets = this.facelets;
		if (facelets.includes('?')) return false;
		for (let face = 0; face < 6; face++) {
			const first = facelets[face * 9];
			for (let k = 1; k < 9; k++) {
				if (facelets[face * 9 + k] !== first) return false;
			}
		}
		return true;
	});

	/** The move that was applied last, if any. */
	currentMove = $derived(this.cursor > 0 ? this.moves[this.cursor - 1] : null);
	/** The move that is about to be applied, if any. */
	upcomingMove = $derived(this.moves[this.cursor] ?? null);
	/** How many moves are left to play. */
	remainingMoves = $derived(Math.max(0, this.moves.length - this.cursor));
	/** Fraction of the solution that has been played, `0..1`. */
	progress = $derived(this.moves.length === 0 ? 0 : this.cursor / this.moves.length);
	/** True when the answer is longer than the requested maximum. */
	exceedsLimit = $derived(this.moves.length > 0 && this.moves.length > this.maxDepth);
	/** True when a solution exists and can be played. */
	hasSolution = $derived(this.moves.length > 0);
	/** True while the engine is loading or solving. */
	isBusy = $derived(
		this.status === 'loading' || this.status === 'building' || this.status === 'solving'
	);
	/**
	 * True when the Play button should be available.
	 *
	 * Play is the primary action: it is offered as soon as the painted cube is
	 * solvable and not already solved, and it solves the cube itself when there
	 * is no solution yet. Anything else - an unpainted, miscounted or impossible
	 * cube, an already solved one, or a busy engine - turns it off.
	 */
	canPlay = $derived(!this.isBusy && this.isSolvable && !this.isPaintedSolved);
	// -- lifecycle ----------------------------------------------------------

	/**
	 * Load the WebAssembly engine and build its lookup tables.
	 *
	 * Call this once from the browser: the tables take a moment to build, which
	 * is why it happens while the user is still painting stickers.
	 */
	async init(): Promise<void> {
		if (this.status !== 'loading') return;
		try {
			const solver = await loadSolver();
			this.solverVersion = solver.version;
			this.status = 'building';
			// Yield once so the UI can show the "preparing" state.
			await new Promise((resolve) => setTimeout(resolve, 0));
			const started = performance.now();
			solver.warmUp();
			this.buildTimeMs = Math.round(performance.now() - started);
			this.tableBytes = solver.tableBytes;
			this.solverGeneration += 1;
			this.status = 'ready';
			this.message = `Solving engine ready: ${(this.tableBytes / 1024 / 1024).toFixed(1)} MB of lookup tables built in ${this.buildTimeMs} ms.`;
		} catch (error) {
			this.status = 'error';
			this.error = `Could not load the WebAssembly solver: ${describeError(error)}`;
		}
	}

	// -- painting -----------------------------------------------------------

	/**
	 * Paint one sticker of the *displayed* cube.
	 *
	 * The index is mapped back to the painted cube, so clicking a sticker while
	 * a solution is displayed still edits the right sticker. Centre stickers are
	 * ignored: they never move, so their colour identifies the face.
	 */
	paintSticker(displayIndex: number, color: number): void {
		const index = this.baseStickerIndex(displayIndex);
		if (isCentre(index)) {
			this.message = 'Centre stickers are fixed - they say which face a sticker belongs to.';
			return;
		}
		const next = [...this.colors];
		next[index] = color;
		this.colors = next;
		this.clearSolution('Editing the cube cleared the previous solution.');
	}

	/** Remove the colour of one sticker. */
	eraseSticker(displayIndex: number): void {
		this.paintSticker(displayIndex, UNKNOWN_STICKER);
	}

	/** Put the solved cube back. */
	resetToSolved(): void {
		this.colors = solvedColors();
		this.clearSolution();
		this.message = 'Cube reset to the solved state.';
		this.error = '';
	}

	/** Clear every sticker except the centres, ready for a fresh paint. */
	clearPaint(): void {
		this.colors = this.colors.map((color, index) => (isCentre(index) ? color : UNKNOWN_STICKER));
		this.clearSolution();
		this.message = 'Cleared every sticker - paint the cube to describe your scramble.';
		this.error = '';
	}

	/**
	 * Apply a random move sequence to the cube.
	 *
	 * Random moves keep whatever is already there, so an unsolvable cube would
	 * stay unsolvable forever. When the painted cube is incomplete or impossible
	 * it is reset to a solved cube first, which is what makes Scramble a way out
	 * of a mistake as well as a way to get a random puzzle.
	 */
	scramble(length = SCRAMBLE_LENGTH): void {
		const solver = getSolver();
		if (!solver) {
			this.error = 'The solving engine is still loading.';
			return;
		}
		const reset = !this.isSolvable;
		const faces = ['U', 'R', 'F', 'D', 'L', 'B'];
		const suffixes = ['', '2', "'"];
		const parts: string[] = [];
		let previous = -1;
		for (let i = 0; i < length; i++) {
			let face = Math.floor(Math.random() * faces.length);
			while (face === previous) face = Math.floor(Math.random() * faces.length);
			previous = face;
			parts.push(faces[face] + suffixes[Math.floor(Math.random() * suffixes.length)]);
		}
		try {
			const from = reset ? solvedColors() : this.colors;
			this.colors = faceletsToColors(solver.applyMoves(colorsToFacelets(from), parts.join(' ')));
			this.clearSolution();
			this.error = '';
			this.message = reset
				? `This cube could not be solved, so it was reset to a solved cube and then scrambled with ${length} random moves.`
				: `Scrambled with ${length} random moves.`;
		} catch (error) {
			this.error = describeError(error);
		}
	}

	/**
	 * Map a sticker of the displayed cube back to the painted cube.
	 *
	 * Uses the engine's composed move permutation, so it stays correct for any
	 * number of applied moves.
	 */
	baseStickerIndex(displayIndex: number): number {
		if (this.cursor === 0 || this.moves.length === 0) return displayIndex;
		const solver = getSolver();
		if (!solver) return displayIndex;
		try {
			const permutation = solver.moveIndices(this.displayedMoves.join(' '));
			return permutation[displayIndex] ?? displayIndex;
		} catch (error) {
			this.error = describeError(error);
			return displayIndex;
		}
	}

	// -- solving ------------------------------------------------------------

	/** Ask the engine to solve the painted cube. */
	async solve(): Promise<void> {
		const solver = getSolver();
		if (!solver) {
			this.error = 'The solving engine is still loading.';
			return;
		}
		this.pause();
		this.error = '';
		this.message = '';
		this.status = 'solving';
		// Yield once so the UI can show that the engine is working.
		await new Promise((resolve) => setTimeout(resolve, 0));

		try {
			const answer = solver.solve(this.facelets, this.maxDepth).trim();
			const moves = answer.length === 0 ? [] : answer.split(/\s+/);
			this.moves = moves;
			this.cursor = 0;
			this.bump('snap');
			if (moves.length === 0) {
				this.message = 'This cube is already solved.';
			} else if (moves.length > this.maxDepth) {
				this.message = `Found a ${moves.length} move solution, which is longer than the requested maximum of ${this.maxDepth} moves.`;
			} else {
				this.message = `Solved in ${moves.length} moves. Press play or step through it.`;
			}
		} catch (error) {
			this.moves = [];
			this.cursor = 0;
			this.error = describeError(error);
		} finally {
			this.status = 'ready';
		}
	}

	// -- playback -----------------------------------------------------------

	/**
	 * Start, or resume, playing the solution.
	 *
	 * Play works as soon as the painted cube is solvable and unsolved: when there
	 * is no solution yet the cube is solved first, so the button is never a dead
	 * end. A solution that has already been played to the end simply starts again.
	 */
	async play(): Promise<void> {
		if (this.isBusy) return;
		if (this.moves.length === 0) {
			if (!this.canPlay) return;
			await this.solve();
			if (this.moves.length === 0) return;
		}
		if (this.cursor >= this.moves.length) {
			this.cursor = 0;
			this.bump('snap');
		}
		this.playing = true;
		this.scheduleStep();
	}

	/** Stop playing, keeping the current position. */
	pause(): void {
		this.playing = false;
		if (this.timer !== null) {
			clearTimeout(this.timer);
			this.timer = null;
		}
	}

	/** Play or pause, depending on the current state. */
	togglePlay(): void {
		if (this.playing) this.pause();
		else void this.play();
	}

	/** Apply the next move, animating it. Returns false at the end. */
	stepForward(): boolean {
		if (this.cursor >= this.moves.length) return false;
		const move = this.moves[this.cursor];
		this.cursor += 1;
		this.bump('move', move);
		return true;
	}

	/** Undo the last move, animating it backwards. Returns false at the start. */
	stepBackward(): boolean {
		if (this.cursor <= 0) return false;
		const move = this.moves[this.cursor - 1];
		this.cursor -= 1;
		this.bump('undo', move);
		return true;
	}

	/**
	 * Move the cursor to any position.
	 *
	 * A single step is animated; anything else makes the cube jump to the state
	 * for that position.
	 */
	jumpTo(cursor: number): void {
		const target = Math.max(0, Math.min(this.moves.length, Math.round(cursor)));
		if (target === this.cursor) return;
		this.pause();
		if (target === this.cursor + 1) {
			this.stepForward();
			return;
		}
		if (target === this.cursor - 1) {
			this.stepBackward();
			return;
		}
		this.cursor = target;
		this.bump('snap');
	}

	/** Set the playback speed, in milliseconds per move. */
	setSpeed(milliseconds: number): void {
		this.speed = Math.max(100, Math.min(800, Math.round(milliseconds)));
	}

	/** Set the longest solution the engine may return. */
	setMaxDepth(depth: number): void {
		this.maxDepth = Math.max(15, Math.min(30, Math.round(depth)));
	}

	// -- internals ----------------------------------------------------------

	private scheduleStep(): void {
		if (this.timer !== null) clearTimeout(this.timer);
		this.timer = setTimeout(() => {
			this.timer = null;
			if (!this.playing) return;
			const advanced = this.stepForward();
			if (advanced && this.cursor < this.moves.length) {
				this.scheduleStep();
			} else {
				this.playing = false;
				if (advanced) this.message = 'Solution finished.';
			}
		}, this.speed);
	}

	/** Drop the solution, snapping the cube back to the painted state. */
	private clearSolution(message?: string): void {
		const hadSolution = this.moves.length > 0 || this.cursor > 0;
		this.pause();
		this.moves = [];
		this.cursor = 0;
		if (hadSolution) {
			this.bump('snap');
			if (message) this.message = message;
		}
	}

	/** Announce a change for the 3D view to animate. */
	private bump(kind: TransitionKind, move: string | null = null): void {
		this.transition = { id: this.transition.id + 1, kind, move };
	}
}
