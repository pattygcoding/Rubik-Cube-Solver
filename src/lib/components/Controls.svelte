<script lang="ts">
	/**
	 * Everything the user drives the app with: the colour palette, the solve
	 * button, playback controls and the move timeline.
	 */
	import type { CubeState } from '../CubeState.svelte.js';
	import { FACE_COLORS, UNKNOWN_CSS, UNKNOWN_STICKER } from '../cube-geometry';

	let { cube }: { cube: CubeState } = $props();

	/** Palette entries: the six colours plus an eraser. */
	const PALETTE = [
		...FACE_COLORS.map((color) => ({
			value: color.id as number,
			name: color.name,
			css: color.css
		})),
		{ value: UNKNOWN_STICKER, name: 'Erase', css: UNKNOWN_CSS }
	];

	/** How many stickers are still unpainted. */
	const missing = $derived(cube.counts.unknown);

	/** Why the Solve button is (or is not) available. */
	const solveReason = $derived(
		cube.isBusy
			? 'The solving engine is busy.'
			: cube.isSolvable
				? 'Solve the cube you painted.'
				: cube.validation.message
	);

	/** Scramble always works: it resets an unusable cube first. */
	const scrambleReason = $derived(
		cube.isSolvable
			? 'Scramble the cube with 22 random moves.'
			: 'This cube cannot be solved, so it will be reset to a solved cube and then scrambled.'
	);

	/**
	 * Why the Play button is (or is not) available.
	 *
	 * Play is offered whenever the painted cube is solvable and not already
	 * solved; it solves the cube itself when no solution has been found yet.
	 */
	const playReason = $derived(
		cube.playing
			? 'Pause the solution.'
			: cube.isBusy
				? 'The solving engine is busy.'
				: cube.isPaintedSolved
					? 'The cube is already solved, so there is nothing to play.'
					: cube.isSolvable
						? cube.hasSolution
							? 'Play the solution.'
							: 'Solve the cube and play the solution move by move.'
						: cube.validation.message
	);

	/** Colours whose sticker count is not nine. */
	const wrongCounts = $derived(
		FACE_COLORS.map((color, index) => ({ color, count: cube.counts.perColor[index] })).filter(
			(entry) => entry.count !== 9
		)
	);

	let timeline = $state<HTMLDivElement | undefined>(undefined);

	// Keep the move that is about to be applied in view.
	$effect(() => {
		const cursor = cube.cursor;
		const total = cube.moves.length;
		void cursor;
		void total;
		const active = timeline?.querySelector('.move.next, .move.current');
		active?.scrollIntoView({ block: 'nearest', inline: 'center', behavior: 'smooth' });
	});

	// Number keys pick a colour, so painting stays quick.
	function onKeyDown(event: KeyboardEvent) {
		if (event.target instanceof HTMLInputElement) return;
		const digit = Number.parseInt(event.key, 10);
		if (Number.isFinite(digit) && digit >= 1 && digit <= 6) {
			cube.activeColor = digit - 1;
		} else if (event.key === '0' || event.key === 'e') {
			cube.activeColor = UNKNOWN_STICKER;
		}
	}

	function moveClass(index: number): string {
		if (index === cube.cursor - 1) return 'move current';
		if (index < cube.cursor) return 'move done';
		if (index === cube.cursor) return 'move next';
		return 'move';
	}
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="controls">
	<section class="panel palette">
		<h2>Palette</h2>
		<div class="swatches">
			{#each PALETTE as entry (entry.value)}
				<button
					type="button"
					class="swatch"
					class:active={cube.activeColor === entry.value}
					style:--swatch={entry.css}
					title="{entry.name} ({entry.value === UNKNOWN_STICKER ? '0' : entry.value + 1})"
					aria-pressed={cube.activeColor === entry.value}
					onclick={() => (cube.activeColor = entry.value)}
				>
					<span class="dot"></span>
					<span class="name">{entry.name}</span>
				</button>
			{/each}
		</div>
		<div class="counts" aria-label="Stickers painted per colour">
			{#each FACE_COLORS as color, index (color.id)}
				{@const count = cube.counts.perColor[index]}
				<span
					class="count"
					class:bad={count !== 9}
					style:--swatch={color.css}
					title="{color.name}: {count} of 9 stickers"
				>
					<span class="dot"></span>
					<span class="num">{count}</span>
				</span>
			{/each}
			{#if missing > 0}
				<span class="count unknown" title="{missing} stickers are not painted yet">
					<span class="dot"></span>
					<span class="num">{missing}</span>
				</span>
			{/if}
		</div>

		{#if missing > 0}
			<p class="hint">
				{54 - missing} of 54 stickers painted. Click stickers on the cube or in the net; keys 1-6 pick
				a colour, 0 erases.
			</p>
		{:else if wrongCounts.length > 0}
			<p class="hint">
				Every colour needs exactly nine stickers, but
				{wrongCounts.map((entry) => `${entry.color.name} has ${entry.count}`).join(' and ')}.
			</p>
		{:else}
			<p class="hint">All 54 stickers painted. Click any sticker to change it.</p>
		{/if}
	</section>

	<section class="panel actions">
		<h2>Solve</h2>
		<div class="row">
			<button
				class="primary"
				onclick={() => cube.solve()}
				disabled={cube.isBusy || !cube.isSolvable}
				title={solveReason}
			>
				{cube.status === 'solving' ? 'Solving…' : 'Solve cube'}
			</button>
			<label class="depth" for="max-depth">
				max
				<input
					id="max-depth"
					name="max-depth"
					type="number"
					min="15"
					max="30"
					value={cube.maxDepth}
					oninput={(event) => cube.setMaxDepth(Number((event.target as HTMLInputElement).value))}
				/>
				moves
			</label>
		</div>
		<div class="row secondary">
			<button onclick={() => cube.scramble()} disabled={cube.isBusy} title={scrambleReason}>
				Scramble
			</button>
			<button
				onclick={() => cube.resetToSolved()}
				disabled={cube.isBusy}
				title="Replace the cube with a solved one."
			>
				Solved cube
			</button>
			<button
				onclick={() => cube.clearPaint()}
				disabled={cube.isBusy}
				title="Unpaint every sticker except the centres."
			>
				Clear stickers
			</button>
		</div>

		{#if cube.status === 'loading'}
			<p class="hint">Loading the WebAssembly solver…</p>
		{:else if cube.status === 'building'}
			<p class="hint">Building the two-phase lookup tables…</p>
		{:else}
			<p
				class="validation"
				class:ok={cube.validation.state === 'ok'}
				class:impossible={cube.validation.state === 'impossible'}
			>
				{#if cube.validation.state === 'ok'}
					<span class="tick" aria-hidden="true">✓</span>
				{:else if cube.validation.state === 'impossible'}
					<span class="tick" aria-hidden="true">!</span>
				{:else}
					<span class="tick" aria-hidden="true">…</span>
				{/if}
				{cube.validation.message}
			</p>
		{/if}

		{#if cube.error}
			<p class="error" role="alert">{cube.error}</p>
		{:else if cube.message}
			<p class="hint">{cube.message}</p>
		{/if}
	</section>

	<section class="panel playback">
		<h2>Playback</h2>
		<div class="row transport">
			<button
				title="Step backwards"
				aria-label="Step backwards"
				onclick={() => cube.stepBackward()}
				disabled={!cube.hasSolution || cube.cursor === 0}
			>
				⏮
			</button>
			<button
				class="primary play"
				title={playReason}
				aria-label={cube.playing ? 'Pause' : 'Play'}
				onclick={() => cube.togglePlay()}
				disabled={!cube.playing && !cube.canPlay}
			>
				{cube.playing ? '⏸ Pause' : cube.hasSolution ? '▶ Play' : '▶ Solve & play'}
			</button>
			<button
				title="Step forwards"
				aria-label="Step forwards"
				onclick={() => cube.stepForward()}
				disabled={!cube.hasSolution || cube.remainingMoves === 0}
			>
				⏭
			</button>
			<button
				title="Restart the solution from the beginning"
				aria-label="Restart the solution"
				onclick={() => cube.jumpTo(0)}
				disabled={!cube.hasSolution || cube.cursor === 0}
			>
				⟲ Restart
			</button>
		</div>

		<label class="speed" for="playback-speed">
			<span class="label">Speed</span>
			<input
				id="playback-speed"
				name="playback-speed"
				type="range"
				min="100"
				max="800"
				step="20"
				value={cube.speed}
				oninput={(event) => cube.setSpeed(Number((event.target as HTMLInputElement).value))}
			/>
			<span class="value">{cube.speed} ms / move</span>
			<span class="ends" aria-hidden="true">
				<span>slower</span>
				<span>faster</span>
			</span>
		</label>

		{#if cube.hasSolution}
			<div class="summary">
				<span>{cube.moves.length} moves</span>
				<span>{cube.cursor} played</span>
				<span>{cube.remainingMoves} left</span>
				{#if cube.exceedsLimit}
					<span class="warn">longer than the {cube.maxDepth} move limit</span>
				{/if}
			</div>
			<div class="timeline" bind:this={timeline}>
				{#each cube.moves as move, index (index)}
					<button
						type="button"
						class={moveClass(index)}
						title="Jump to move {index + 1}"
						onclick={() => cube.jumpTo(index + 1)}
					>
						<span class="number">{index + 1}</span>
						<span class="token">{move}</span>
					</button>
				{/each}
			</div>
		{:else if cube.canPlay}
			<p class="hint">
				No solution yet: press <strong>Play</strong> to solve the cube and watch it, or press
				<strong>Solve cube</strong> to see the move list first.
			</p>
		{:else if cube.isPaintedSolved}
			<p class="hint">
				The cube is already solved, so there is nothing to play. Paint a scramble or press
				<strong>Scramble</strong>.
			</p>
		{:else}
			<p class="hint">This cube cannot be played yet - see the note in the Solve panel above.</p>
		{/if}
	</section>

	<footer class="meta">
		<span>
			{cube.solverVersion || 'solver not loaded'}
			{#if cube.tableBytes}· {(cube.tableBytes / 1024 / 1024).toFixed(1)} MB of tables{/if}
		</span>
		<span>
			{cube.isPaintedSolved ? 'cube is solved' : cube.isSolvable ? 'ready to play' : 'cube needs attention'}
		</span>
	</footer>
</div>

<style lang="scss">
	.controls {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.panel {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 14px;
		border-radius: 14px;
		border: 1px solid #1e2431;
		background: #0d1016;

		h2 {
			margin: 0;
			font-size: 0.72rem;
			font-weight: 700;
			letter-spacing: 0.14em;
			text-transform: uppercase;
			color: #7c869a;
		}
	}

	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;

		&.secondary button {
			flex: 1 1 auto;
		}
	}

	button {
		padding: 8px 12px;
		border-radius: 9px;
		border: 1px solid #262d3b;
		background: #151a23;
		color: #d8dee9;
		font: inherit;
		font-size: 0.82rem;
		cursor: pointer;
		transition:
			background 120ms ease,
			border-color 120ms ease,
			transform 90ms ease;

		&:hover:not(:disabled) {
			background: #1b2230;
			border-color: #33405a;
		}

		&:active:not(:disabled) {
			transform: translateY(1px);
		}

		&:disabled {
			opacity: 0.45;
			cursor: not-allowed;
		}

		&.primary {
			background: linear-gradient(180deg, #a5edcf, #6fd9af);
			border-color: #4fb894;
			color: #05070c;
			font-weight: 700;

			&:hover:not(:disabled) {
				background: linear-gradient(180deg, #b9f4da, #a5edcf);
			}
		}
	}

	.swatches {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(78px, 1fr));
		gap: 6px;
	}

	.swatch {
		display: flex;
		align-items: center;
		gap: 7px;
		padding: 6px 8px;
		font-size: 0.74rem;

		.dot {
			width: 15px;
			height: 15px;
			border-radius: 4px;
			background: var(--swatch);
			border: 1px solid rgba(0, 0, 0, 0.45);
			box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
			flex: none;
		}

		.name {
			color: #aeb7c6;
			white-space: nowrap;
		}

		&.active {
			border-color: #a5edcf;
			background: #17222a;

			.name {
				color: #eafaf3;
			}
		}
	}

	.depth {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-size: 0.74rem;
		color: #7c869a;

		input {
			width: 58px;
			padding: 6px 8px;
			border-radius: 8px;
			border: 1px solid #262d3b;
			background: #11151d;
			color: #d8dee9;
			font: inherit;
			font-size: 0.78rem;
		}
	}

	// Sticker counts per colour: anything that is not nine is flagged, so a
	// miscounted cube is obvious before you even press Solve.
	.counts {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.count {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 3px 7px;
		border-radius: 999px;
		border: 1px solid #262d3b;
		background: #11151d;
		font-size: 0.7rem;
		font-variant-numeric: tabular-nums;
		color: #8b94a7;

		.dot {
			width: 11px;
			height: 11px;
			border-radius: 3px;
			background: var(--swatch);
			border: 1px solid rgba(0, 0, 0, 0.45);
		}

		&.bad {
			border-color: rgba(251, 191, 36, 0.55);
			background: rgba(120, 53, 15, 0.22);
			color: #fbbf24;
			font-weight: 600;
		}

		&.unknown {
			--swatch: #33363d;
			border-style: dashed;
			border-color: #3b4252;
		}
	}

	.validation {
		display: flex;
		align-items: flex-start;
		gap: 7px;
		margin: 0;
		padding: 8px 10px;
		border-radius: 9px;
		border: 1px solid #2a3140;
		background: #11151d;
		font-size: 0.72rem;
		line-height: 1.5;
		color: #9aa4b8;

		.tick {
			flex: none;
			width: 14px;
			text-align: center;
			font-weight: 700;
			color: #7c869a;
		}

		&.ok {
			border-color: rgba(165, 237, 207, 0.45);
			background: rgba(13, 60, 56, 0.25);
			color: #b9f4da;

			.tick {
				color: #a5edcf;
			}
		}

		&.impossible {
			border-color: rgba(248, 113, 113, 0.45);
			background: rgba(80, 20, 20, 0.3);
			color: #fca5a5;

			.tick {
				color: #f87171;
			}
		}
	}

	.speed {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		grid-template-rows: auto auto;
		align-items: center;
		gap: 2px 10px;
		font-size: 0.74rem;
		color: #7c869a;

		.label {
			grid-column: 1;
			grid-row: 1;
		}

		.value {
			grid-column: 3;
			grid-row: 1;
			font-variant-numeric: tabular-nums;
			white-space: nowrap;
		}

		// The ends caption sits under the track, so the direction is obvious.
		.ends {
			grid-column: 2;
			grid-row: 2;
			display: flex;
			justify-content: space-between;
			font-size: 0.6rem;
			letter-spacing: 0.04em;
			text-transform: uppercase;
			color: #5b657a;
		}

		input[type='range'] {
			grid-column: 2;
			grid-row: 1;
			width: 100%;
			accent-color: #a5edcf;
			// The value is milliseconds per move, so small is fast. Rendering the
			// control right-to-left puts the *slow* end on the left and the *fast*
			// end on the right, which is the way people expect a speed slider to
			// read, while the value itself stays truthful.
			direction: rtl;
		}
	}

	.transport button {
		min-width: 46px;
		font-size: 0.9rem;

		&.play {
			flex: 1 1 auto;
		}
	}

	.summary {
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		font-size: 0.74rem;
		color: #8b94a7;
		font-variant-numeric: tabular-nums;

		.warn {
			color: #fbbf24;
		}
	}

	.timeline {
		display: flex;
		gap: 6px;
		padding-bottom: 4px;
		overflow-x: auto;
		scrollbar-width: thin;

		.move {
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: 1px;
			min-width: 42px;
			padding: 5px 7px;
			border-radius: 8px;
			font-variant-numeric: tabular-nums;

			.number {
				font-size: 0.56rem;
				color: #5b657a;
				line-height: 1;
			}

			.token {
				font-size: 0.82rem;
				font-weight: 600;
				line-height: 1.1;
			}

			&.done {
				border-color: #1d3a35;
				background: #101c1a;
				color: #6f8f88;
			}

			&.next {
				border-color: #4b5563;
				color: #e2e8f0;
			}

			&.current {
				border-color: #a5edcf;
				background: #12262a;
				color: #b9f4da;
				box-shadow: 0 0 0 1px rgba(165, 237, 207, 0.35);

				.number {
					color: #a5edcf;
				}
			}
		}
	}

	.hint {
		margin: 0;
		font-size: 0.7rem;
		line-height: 1.5;
		color: #7c869a;
	}

	.error {
		margin: 0;
		padding: 8px 10px;
		border-radius: 9px;
		border: 1px solid rgba(248, 113, 113, 0.4);
		background: rgba(127, 29, 29, 0.22);
		color: #fca5a5;
		font-size: 0.74rem;
		line-height: 1.5;
	}

	.meta {
		display: flex;
		justify-content: space-between;
		flex-wrap: wrap;
		gap: 8px;
		padding: 0 4px;
		font-size: 0.64rem;
		color: #5b657a;
		font-variant-numeric: tabular-nums;
	}
</style>
