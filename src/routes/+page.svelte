<script lang="ts">
	/**
	 * Rubik's Cube solver: paint a cube, let the Rust/WebAssembly two-phase
	 * engine solve it, then step through the solution.
	 */
	import { onMount } from 'svelte';
	import Controls from '#lib/components/Controls.svelte';
	import CubeView from '#lib/components/CubeView.svelte';
	import NetView from '#lib/components/NetView.svelte';
	import { CubeState } from '#lib/CubeState.svelte.js';

	/** One store for the whole page; the components share it through props. */
	const cube = new CubeState();

	onMount(() => {
		void cube.init();
	});
</script>

<svelte:head>
	<title>Rubik's Cube Solver - Rust + WebAssembly two-phase search</title>
	<meta
		name="description"
		content="Paint a scrambled Rubik's Cube in 3D or on its unfolded net and solve it with a two-phase solver compiled from Rust to WebAssembly."
	/>
</svelte:head>

<div class="page">
	<header class="hero">
		<div class="titles">
			<div class="title-row">
				<!--
					The home mark: the same house glyph, pointed at the same place and
					styled the same way, as the Connect Four, Turing Machines, Scripting
					Library and Blazor sites carry, so every pattygcoding.com project
					returns to the portfolio from the same corner. It opens the main site
					in a new tab, because leaving this page is not what the visitor came
					for.
				-->
				<a
					class="home-mark"
					href="https://pattygcoding.com"
					target="_blank"
					rel="noopener"
					title="pattygcoding.com"
					aria-label="Back to pattygcoding.com"
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="16"
						height="16"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
						<polyline points="9 22 9 12 15 12 15 22" />
					</svg>
				</a>
				<h1>Rubik's Cube Solver</h1>
			</div>
			<p>
				Paint your scramble - click the stickers on the cube or in the unfolded net - then let a
				Herbert Kociemba style <strong>two-phase search</strong>, compiled from Rust to
				WebAssembly, find the moves.
			</p>
		</div>
		<div class="pipeline" aria-hidden="true">
			<span>Rust</span>
			<i>→</i>
			<span>wasm-bindgen</span>
			<i>→</i>
			<span>Svelte 5</span>
			<i>→</i>
			<span>three.js</span>
		</div>
	</header>

	<main>
		<section class="stage">
			<CubeView {cube} />
			<NetView {cube} />
		</section>
		<aside class="sidebar">
			<Controls {cube} />
		</aside>
	</main>

	<footer class="page-footer">
		<span>
			Phase 1 drives the cube into <code>&lt;U, D, R2, L2, F2, B2&gt;</code> guided by the
			twist × slice and flip × slice pruning tables; phase 2 finishes inside that subgroup with
			the corner permutation × slice permutation and edge permutation × slice permutation tables.
		</span>
	</footer>
</div>

<style lang="scss">
	$bg: #06090e;
	$panel: #0d1016;
	$line: #1e2431;
	$text: #d8dee9;
	$muted: #7c869a;
	$accent: #a5edcf;

	:global(body) {
		margin: 0;
		background:
			radial-gradient(circle at 12% -10%, rgba(165, 237, 207, 0.12), transparent 45%),
			radial-gradient(circle at 88% 0%, rgba(105, 140, 200, 0.14), transparent 40%),
			$bg;
		color: $text;
		font-family:
			'Inter',
			ui-sans-serif,
			system-ui,
			-apple-system,
			'Segoe UI',
			Roboto,
			sans-serif;
		-webkit-font-smoothing: antialiased;
	}

	.page {
		max-width: 1380px;
		margin: 0 auto;
		padding: 28px 20px 40px;
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.hero {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		justify-content: space-between;
		gap: 16px;

		h1 {
			margin: 0;
			font-size: clamp(1.5rem, 1.1rem + 1.6vw, 2.2rem);
			letter-spacing: -0.02em;
			background: linear-gradient(92deg, #eafaf3, #8fb0e0);
			-webkit-background-clip: text;
			background-clip: text;
			color: transparent;
		}

		p {
			margin: 0;
			max-width: 62ch;
			font-size: 0.86rem;
			line-height: 1.6;
			color: $muted;

			strong {
				color: #cbd5e1;
				font-weight: 600;
			}
		}
	}

	/*
		The mark and the title share a row, the way the Scripting Library banner pairs
		the same mark with its brand, and the gap below the row is the space the
		title used to carry as a margin of its own.
	*/
	.title-row {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 6px;
		min-width: 0;
	}

	/*
		The home mark back to pattygcoding.com: a dark chip with a hairline border
		and a muted icon that turns mint on hover, which is the treatment the other
		pattygcoding.com projects give the same link in the same corner.  The chip
		colour is the one the pipeline chip beside it uses, so the two read as a
		matched pair on the page background.
	*/
	.home-mark {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		/* `border-box` so the 32px square includes the hairline border, exactly as
		   Tailwind's preflight makes `w-8 h-8` behave on the other sites. */
		box-sizing: border-box;
		width: 32px;
		height: 32px;
		border-radius: 6px;
		border: 1px solid $line;
		background: rgba(13, 16, 22, 0.75);
		color: $muted;
		transition:
			color 0.15s ease,
			border-color 0.15s ease;

		&:hover,
		&:focus-visible {
			color: $accent;
			border-color: rgba(165, 237, 207, 0.4);
		}
	}

	.pipeline {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		border-radius: 999px;
		border: 1px solid $line;
		background: rgba(13, 16, 22, 0.75);
		font-size: 0.68rem;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: $muted;

		span {
			color: #cbd5e1;
		}

		i {
			color: $accent;
			font-style: normal;
		}
	}

	main {
		display: grid;
		grid-template-columns: minmax(0, 1.65fr) minmax(320px, 1fr);
		gap: 20px;
		align-items: start;
	}

	.stage {
		display: flex;
		flex-direction: column;
		gap: 16px;
		min-width: 0;

		:global(.cube-view) {
			height: clamp(380px, 52vh, 620px);
		}
	}

	.sidebar {
		display: flex;
		flex-direction: column;
		gap: 16px;
		min-width: 0;
	}

	.page-footer {
		padding: 12px 14px;
		border-radius: 12px;
		border: 1px solid $line;
		background: $panel;
		font-size: 0.7rem;
		line-height: 1.6;
		color: $muted;

		code {
			padding: 1px 5px;
			border-radius: 5px;
			background: #141a24;
			color: #8fb0e0;
			font-size: 0.68rem;
		}
	}

	@media (max-width: 1000px) {
		main {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
