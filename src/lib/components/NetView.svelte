<script lang="ts">
	/**
	 * The unfolded net of the cube, for quick painting and inspection.
	 *
	 * It shows the *displayed* cube, so it stays in sync with the 3D view while
	 * a solution is playing. Stickers are painted with the active palette colour;
	 * right clicking or shift clicking clears a sticker.
	 *
	 * The layout is the classic Kociemba net - the same one the solver's facelet
	 * string is written in:
	 *
	 * ```text
	 *            U
	 *      L     F     R     B
	 *            D
	 * ```
	 *
	 * Row 0 of `U` is the row furthest from the viewer (the back of the cube) and
	 * row 0 of `D` is the row nearest to the viewer, which is what makes the
	 * unfolded cross fold up without mirroring anything.
	 */
	import { CENTRE_STICKERS } from '../CubeState.svelte.js';
	import type { CubeState } from '../CubeState.svelte.js';
	import { FACE_COLORS, stickerCss, type FaceId } from '../cube-geometry';

	let { cube }: { cube: CubeState } = $props();

	/** Which face goes where in the 4x3 net. */
	const NET_LAYOUT: readonly { face: FaceId; row: number; column: number }[] = [
		{ face: 0, row: 1, column: 2 }, // U
		{ face: 4, row: 2, column: 1 }, // L
		{ face: 2, row: 2, column: 2 }, // F
		{ face: 1, row: 2, column: 3 }, // R
		{ face: 5, row: 2, column: 4 }, // B
		{ face: 3, row: 3, column: 2 } // D
	];

	/** Is the net open? */
	let open = $state(true);

	const cells = $derived(cube.displayColors);

	function label(face: FaceId, position: number): string {
		const color = cells[face * 9 + position];
		const name = FACE_COLORS[color]?.name ?? 'not painted';
		return `${FACE_COLORS[face].letter} face, sticker ${position + 1}, ${name}`;
	}

	function paint(event: MouseEvent, index: number) {
		if (event.shiftKey) {
			cube.eraseSticker(index);
			return;
		}
		cube.paintSticker(index, cube.activeColor);
	}
</script>

<section class="net-view" class:collapsed={!open}>
	<header>
		<button class="toggle" onclick={() => (open = !open)} aria-expanded={open}>
			<span class="chevron" class:open>▸</span>
			2D net
		</button>
		<span class="legend">white up · yellow down · green front · blue back · red right · orange left</span>
	</header>

	{#if open}
		<div class="net">
			{#each NET_LAYOUT as entry (entry.face)}
				<div
					class="face"
					style="grid-row: {entry.row}; grid-column: {entry.column};"
					data-face={FACE_COLORS[entry.face].letter}
				>
					<span class="face-label">{FACE_COLORS[entry.face].letter}</span>
					<div class="stickers">
						{#each Array(9) as _, position (position)}
							{@const index = entry.face * 9 + position}
							{@const fixed = CENTRE_STICKERS.includes(index)}
							<button
								type="button"
								class="sticker"
								class:fixed
								class:unknown={(cells[index] ?? 6) >= 6}
								style:background={stickerCss(cells[index])}
								disabled={fixed}
								title={fixed ? 'Centre stickers are fixed' : label(entry.face, position)}
								aria-label={label(entry.face, position)}
								onclick={(event) => paint(event, index)}
								oncontextmenu={(event) => {
									event.preventDefault();
									cube.eraseSticker(index);
								}}
							></button>
						{/each}
					</div>
				</div>
			{/each}
		</div>
		<p class="note">
			Click a sticker to paint it with the selected colour. Right click or shift click clears a
			sticker. The top row of <strong>U</strong> is the back of the cube and the top row of
			<strong>D</strong> is the front, so the cross folds up exactly like the real cube.
		</p>
	{/if}
</section>

<style lang="scss">
	.net-view {
		display: flex;
		flex-direction: column;
		gap: 12px;
		padding: 14px;
		border-radius: 14px;
		border: 1px solid #1e2431;
		background: #0d1016;

		header {
			display: flex;
			flex-wrap: wrap;
			align-items: baseline;
			gap: 10px;
		}

		.toggle {
			display: inline-flex;
			align-items: center;
			gap: 6px;
			padding: 0;
			border: 0;
			background: none;
			color: #e2e8f0;
			font: inherit;
			font-weight: 600;
			cursor: pointer;

			.chevron {
				display: inline-block;
				transition: transform 140ms ease;
				color: #b9f4da;

				&.open {
					transform: rotate(90deg);
				}
			}
		}

		.legend {
			font-size: 0.68rem;
			color: #7c869a;
		}

		.note {
			margin: 0;
			font-size: 0.7rem;
			line-height: 1.5;
			color: #7c869a;

			strong {
				color: #cbd5e1;
			}
		}
	}

	.net {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		grid-template-rows: repeat(3, auto);
		gap: 10px;
		max-width: 520px;
	}

	.face {
		position: relative;
		padding: 6px;
		border-radius: 10px;
		background: #14181f;
		border: 1px solid #232936;

		.face-label {
			position: absolute;
			top: -7px;
			left: 8px;
			padding: 0 4px;
			background: #0d1016;
			color: #7c869a;
			font-size: 0.62rem;
			font-weight: 700;
			letter-spacing: 0.08em;
		}
	}

	.stickers {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 3px;
		aspect-ratio: 1;
	}

	.sticker {
		position: relative;
		width: 100%;
		height: 100%;
		padding: 0;
		border: 1px solid rgba(8, 10, 14, 0.7);
		border-radius: 4px;
		cursor: pointer;
		transition:
			transform 90ms ease,
			box-shadow 90ms ease;

		&:hover:not(:disabled) {
			transform: scale(1.08);
			box-shadow: 0 0 0 2px rgba(185, 244, 218, 0.55);
			z-index: 1;
		}

		&.unknown {
			background-image: repeating-linear-gradient(
				45deg,
				rgba(255, 255, 255, 0.06) 0 4px,
				transparent 4px 8px
			);
		}

		&.fixed {
			cursor: not-allowed;
			border-style: dashed;
			border-color: rgba(148, 163, 184, 0.35);
		}
	}
</style>
