<script lang="ts">
	/**
	 * The 3D cube: rendering, click-to-paint raycasting and the pivot animation
	 * that turns one layer at a time.
	 *
	 * The cube is 26 individual `BoxGeometry` meshes on the integer grid, each
	 * with six materials (three.js' material order is `+x, -x, +y, -y, +z, -z`).
	 *
	 * ## How a mesh is kept in sync with the cube
	 *
	 * A mesh stands for one *slot* of the cube, not for one cubie. Its position
	 * and rotation come from `placementsFromColors(displayed state)`, and its six
	 * sticker colours come from the same state, so position and colour are always
	 * applied together. A face turn is animated by attaching the nine meshes of a
	 * layer to a temporary group and rotating that group by a multiple of 90
	 * degrees; when the turn lands, the meshes are pinned back onto the grid and
	 * the state is re-applied. Because every mesh is an identical box, moving
	 * them between slots is invisible - only the stickers matter.
	 */
	import { onMount } from 'svelte';
	import * as THREE from 'three';
	import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
	import {
		BOX_FACE_NORMALS,
		CORNER_SLOTS,
		EDGE_SLOTS,
		INTERIOR_CSS,
		applyMat3,
		cubieFaceColors,
		placementsFromColors,
		stickerIndexAt,
		stickerHex,
		type CubePlacements,
		type Mat3,
		type Vec3
	} from '../cube-geometry';
	import type { CubeState, CubeTransition } from '../CubeState.svelte.js';
	import { getSolver } from '../solver';

	let { cube }: { cube: CubeState } = $props();

	/** Edge length of one sub cube; the gap shows the black core between them. */
	const CUBIE_SIZE = 0.94;
	/**
	 * Size of the black box that fills the middle cell of the 3x3x3 grid.
	 *
	 * Sub cubes are `CUBIE_SIZE` wide on a grid of spacing 1, so the gap between
	 * two of them is 0.06 and their inner faces sit 0.53 from the centre. A core
	 * any larger than that pokes *into* the sub cubes: it slices through the layer
	 * that is turning and leaves the cube looking hollow, which is what a 2.84
	 * wide core did. Just under 1.06 still blocks the 0.06 wide channels that run
	 * straight through the cube - so the page never shows through a gap - while
	 * never touching a sub cube.
	 */
	const CORE_SIZE = 1.04;

	const INTERIOR_HEX = Number.parseInt(INTERIOR_CSS.slice(1), 16);
	const IDENTITY: Mat3 = [1, 0, 0, 0, 1, 0, 0, 0, 1];

	/** The six fixed centre cubies, which never move. */
	const CENTRE_POSITIONS: readonly Vec3[] = [
		[1, 0, 0],
		[-1, 0, 0],
		[0, 1, 0],
		[0, -1, 0],
		[0, 0, 1],
		[0, 0, -1]
	];

	interface Cubie {
		readonly mesh: THREE.Mesh;
		readonly materials: THREE.MeshStandardMaterial[];
		readonly kind: 'corner' | 'edge' | 'centre';
		/** Slot this mesh currently represents, `0..7` / `0..11` / `0..5`. */
		readonly slot: number;
	}

	let container: HTMLDivElement;
	let cubies: Cubie[] = [];
	let scene: THREE.Scene;
	let camera: THREE.PerspectiveCamera;
	let renderer: THREE.WebGLRenderer;
	let controls: OrbitControls;
	let raycaster: THREE.Raycaster;
	let pointer = new THREE.Vector2();
	let resizeObserver: ResizeObserver | undefined;

	/** Placements of the displayed cube, recomputed whenever the state changes. */
	const placements = $derived<CubePlacements>(placementsFromColors(cube.displayColors));

	/** The turn currently animating, if any. */
	let tween: {
		group: THREE.Group;
		axis: 'x' | 'y' | 'z';
		angle: number;
		duration: number;
		elapsed: number;
		onDone: () => void;
	} | null = null;

	/** Pointer bookkeeping, so orbiting never paints a sticker. */
	let pointerStart: { x: number; y: number; time: number; button: number } | null = null;
	/** Sticker under the pointer, for the hover highlight. */
	let hovered: { mesh: THREE.Mesh; materialIndex: number } | null = null;

	onMount(() => {
		buildScene();
		applyPlacements(placements);
		applyColors(placements, cube.displayColors);

		resizeObserver = new ResizeObserver(() => resize());
		resizeObserver.observe(container);

		return () => {
			resizeObserver?.disconnect();
			renderer.setAnimationLoop(null);
			controls.dispose();
			renderer.dispose();
			container.removeChild(renderer.domElement);
		};
	});

	function buildScene() {
		scene = new THREE.Scene();
		scene.background = null;

		camera = new THREE.PerspectiveCamera(42, 1, 0.1, 100);
		camera.position.set(4.6, 4.2, 5.6);
		camera.lookAt(0, 0, 0);

		renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
		renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
		renderer.domElement.classList.add('cube-canvas');
		renderer.domElement.addEventListener('pointerdown', onPointerDown);
		renderer.domElement.addEventListener('pointerup', onPointerUp);
		renderer.domElement.addEventListener('pointermove', onPointerMove);
		renderer.domElement.addEventListener('pointerleave', onPointerLeave);
		renderer.domElement.addEventListener('contextmenu', (event) => event.preventDefault());
		container.appendChild(renderer.domElement);

		controls = new OrbitControls(camera, renderer.domElement);
		controls.enableDamping = true;
		controls.dampingFactor = 0.12;
		controls.enablePan = false;
		controls.minDistance = 5.5;
		controls.maxDistance = 16;
		controls.rotateSpeed = 0.9;

		raycaster = new THREE.Raycaster();

		scene.add(new THREE.AmbientLight(0xffffff, 1.5));
		const key = new THREE.DirectionalLight(0xffffff, 2.4);
		key.position.set(6, 9, 7);
		scene.add(key);
		const fill = new THREE.DirectionalLight(0xffffff, 1.1);
		fill.position.set(-7, -4, -6);
		scene.add(fill);
		const rim = new THREE.DirectionalLight(0xffffff, 0.7);
		rim.position.set(-4, 6, -8);
		scene.add(rim);

		const geometry = new THREE.BoxGeometry(CUBIE_SIZE, CUBIE_SIZE, CUBIE_SIZE);
		scene.add(
			new THREE.Mesh(
				new THREE.BoxGeometry(CORE_SIZE, CORE_SIZE, CORE_SIZE),
				new THREE.MeshStandardMaterial({ color: 0x0b0c0e, roughness: 0.9, metalness: 0 })
			)
		);

		cubies = [];
		for (let slot = 0; slot < 8; slot++) {
			cubies.push(createCubie(geometry, 'corner', slot, CORNER_SLOTS[slot].position));
		}
		for (let slot = 0; slot < 12; slot++) {
			cubies.push(createCubie(geometry, 'edge', slot, EDGE_SLOTS[slot].position));
		}
		for (let slot = 0; slot < 6; slot++) {
			cubies.push(createCubie(geometry, 'centre', slot, CENTRE_POSITIONS[slot]));
		}
		for (const cubie of cubies) scene.add(cubie.mesh);

		resize();
		renderer.setAnimationLoop(renderFrame);

		if (import.meta.env.DEV) {
			// Handy while developing: inspect the scene from the browser console.
			(globalThis as unknown as Record<string, unknown>).__cubeView = {
				scene,
				camera,
				cubies,
				placements: () => placements,
				tween: () => tween,
				pick: (clientX: number, clientY: number) => pickSticker({ clientX, clientY } as PointerEvent)
			};
		}
	}

	/** Placement of one mesh slot in a given state. */
	function placementFor(cubie: Cubie, state: CubePlacements) {
		if (cubie.kind === 'corner') return state.corners[cubie.slot];
		if (cubie.kind === 'edge') return state.edges[cubie.slot];
		return {
			slot: cubie.slot,
			cubie: 0,
			position: CENTRE_POSITIONS[cubie.slot],
			rotation: IDENTITY,
			uncertain: false
		};
	}

	function matrix4From(m: Mat3): THREE.Matrix4 {
		return new THREE.Matrix4().set(m[0], m[1], m[2], 0, m[3], m[4], m[5], 0, m[6], m[7], m[8], 0, 0, 0, 0, 1);
	}

	/** Turn a quaternion that is already a multiple of 90 degrees into a matrix. */
	function mat3FromQuaternion(quaternion: THREE.Quaternion): Mat3 {
		const matrix = new THREE.Matrix4().makeRotationFromQuaternion(quaternion);
		const e = matrix.elements;
		return [
			Math.round(e[0]),
			Math.round(e[4]),
			Math.round(e[8]),
			Math.round(e[1]),
			Math.round(e[5]),
			Math.round(e[9]),
			Math.round(e[2]),
			Math.round(e[6]),
			Math.round(e[10])
		];
	}

	/** Put every mesh exactly where the state says it belongs, without animating. */
	function applyPlacements(state: CubePlacements) {
		for (const cubie of cubies) {
			const placement = placementFor(cubie, state);
			if (!placement) continue;
			const [x, y, z] = placement.position;
			cubie.mesh.position.set(x, y, z);
			cubie.mesh.quaternion.setFromRotationMatrix(matrix4From(placement.rotation));
			cubie.mesh.updateMatrix();
		}
	}

	/** Paint the six materials of every mesh from the same state. */
	function applyColors(state: CubePlacements, colors: readonly number[]) {
		for (const cubie of cubies) {
			const placement = placementFor(cubie, state);
			if (!placement) continue;
			const faceColors = cubieFaceColors(placement, colors);
			for (let face = 0; face < 6; face++) {
				// `stickerHex` already distinguishes an empty sticker from the
				// inside of the cube, so `null` needs no special case here.
				cubie.materials[face].color.setHex(stickerHex(faceColors[face]));
			}
		}
	}

	/** Everything the scene needs to match the cube state, applied together. */
	function syncScene() {
		applyPlacements(placements);
		applyColors(placements, cube.displayColors);
	}

	let lastTransitionId = -1;
	let lastColorsKey = '';
	/** Guards against a finished animation overwriting a newer one. */
	let animationToken = 0;

	/**
	 * One effect drives the whole scene.
	 *
	 * A turn is animated, and the state is only re-applied once it has landed:
	 * painting the slots while the meshes are mid-turn would put the stickers of
	 * the new state onto the old arrangement. Everything else - a jump, a reset,
	 * a repaint - is applied immediately.
	 */
	$effect(() => {
		const transition = cube.transition;
		const colors = cube.displayColors;
		const colorsKey = colors.join('');
		const newTransition = transition.id !== lastTransitionId;
		const newColors = colorsKey !== lastColorsKey;
		if (!newTransition && !newColors) return;

		lastTransitionId = transition.id;
		lastColorsKey = colorsKey;

		if (newTransition && transition.move && (transition.kind === 'move' || transition.kind === 'undo')) {
			void runMove(transition);
			return;
		}
		finishTween();
		syncScene();
	});

	/** Animate one turn, then pin the meshes back onto the state. */
	async function runMove(transition: CubeTransition) {
		const token = ++animationToken;
		finishTween();

		const solver = getSolver();
		const moveIndex = transition.move === null ? undefined : solver?.moveIndex(transition.move);
		const turn =
			moveIndex === undefined || moveIndex === null ? null : solver?.moveTurn(moveIndex);
		if (!turn) {
			syncScene();
			return;
		}
		await animateTurn(turn, transition.kind === 'move' ? 1 : -1);
		if (token !== animationToken) return;
		syncScene();
	}

	/**
	 * Attach the nine meshes of a layer to a temporary group and rotate it.
	 *
	 * The group sits at the origin, so rotating it by a multiple of 90 degrees
	 * turns exactly that layer. The rotation always takes the shortest way round:
	 * a move like `F'` is three quarter turns in the solver's encoding, but
	 * spinning the layer 270 degrees looks wrong, so it is played as -90 degrees
	 * instead.
	 */
	function animateTurn(turn: [number, number, number], direction: 1 | -1): Promise<void> {
		const axis = (['x', 'y', 'z'] as const)[turn[0]];
		const layer = turn[1];
		const quarters = (((turn[2] % 4) + 4) % 4) as 0 | 1 | 2 | 3;
		const shortest = quarters === 3 ? -1 : quarters;
		const angle = (shortest * direction * Math.PI) / 2;

		const group = new THREE.Group();
		scene.add(group);
		for (const cubie of cubies) {
			if (Math.round(cubie.mesh.position[axis]) === layer) group.attach(cubie.mesh);
		}

		return new Promise<void>((resolve) => {
			tween = {
				group,
				axis,
				angle,
				duration: Math.max(120, Math.min(600, cube.speed * 0.75)),
				elapsed: 0,
				onDone: () => {
					group.rotation[axis] = angle;
					group.updateMatrixWorld(true);
					for (const child of [...group.children]) {
						scene.attach(child);
						roundMesh(child as THREE.Mesh);
					}
					scene.remove(group);
					resolve();
				}
			};
		});
	}

	/**
	 * Snap a mesh back onto the integer grid and to the nearest 90 degree
	 * orientation, so hundreds of turns cannot accumulate float drift.
	 */
	function roundMesh(mesh: THREE.Mesh) {
		mesh.position.set(
			Math.round(mesh.position.x),
			Math.round(mesh.position.y),
			Math.round(mesh.position.z)
		);
		const matrix = new THREE.Matrix4().makeRotationFromQuaternion(mesh.quaternion);
		const elements = matrix.elements;
		for (let i = 0; i < 16; i++) elements[i] = Math.round(elements[i]);
		mesh.quaternion.setFromRotationMatrix(matrix);
		mesh.updateMatrix();
	}

	/** Complete a running animation immediately. */
	function finishTween() {
		if (!tween) return;
		const pending = tween;
		tween = null;
		pending.onDone();
	}

	/** One sub cube with its six face materials. */
	function createCubie(
		geometry: THREE.BoxGeometry,
		kind: Cubie['kind'],
		slot: number,
		home: Vec3
	): Cubie {
		const materials = BOX_FACE_NORMALS.map(
			() =>
				new THREE.MeshStandardMaterial({
					color: INTERIOR_HEX,
					roughness: 0.42,
					metalness: 0.04
				})
		);
		const mesh = new THREE.Mesh(geometry, materials);
		mesh.position.set(home[0], home[1], home[2]);
		return { mesh, materials, kind, slot };
	}

	// -- painting -----------------------------------------------------------

	/**
	 * Raycast the pointer against the 26 sub cubes and translate the hit into a
	 * facelet index.
	 *
	 * A sticker is identified by the grid point of the sub cube plus the *world*
	 * normal of the hit face, which is exactly how the facelet tables are keyed.
	 */
	function pickSticker(event: PointerEvent) {
		const rect = renderer.domElement.getBoundingClientRect();
		pointer.set(
			((event.clientX - rect.left) / rect.width) * 2 - 1,
			-((event.clientY - rect.top) / rect.height) * 2 + 1
		);
		raycaster.setFromCamera(pointer, camera);
		const hits = raycaster.intersectObjects(
			cubies.map((cubie) => cubie.mesh),
			false
		);
		const hit = hits[0];
		if (!hit || !hit.face) return null;
		const mesh = hit.object as THREE.Mesh;
		const materialIndex = hit.face.materialIndex ?? Math.floor((hit.faceIndex ?? 0) / 2);
		const position: Vec3 = [
			Math.round(mesh.position.x),
			Math.round(mesh.position.y),
			Math.round(mesh.position.z)
		];
		const worldNormal = applyMat3(
			mat3FromQuaternion(mesh.quaternion),
			BOX_FACE_NORMALS[materialIndex]
		);
		const facelet = stickerIndexAt(position, worldNormal);
		return facelet === undefined ? null : { facelet, mesh, materialIndex };
	}

	function onPointerDown(event: PointerEvent) {
		if (event.button !== 0 && event.button !== 2) return;
		pointerStart = {
			x: event.clientX,
			y: event.clientY,
			time: performance.now(),
			button: event.button
		};
	}

	function onPointerUp(event: PointerEvent) {
		const start = pointerStart;
		pointerStart = null;
		if (!start || start.button !== event.button) return;
		// Orbiting drags the pointer, so only a short, still press paints.
		if (Math.hypot(event.clientX - start.x, event.clientY - start.y) > 6) return;
		if (performance.now() - start.time > 800) return;
		// Never pick mid turn: the meshes are inside the pivot group.
		if (tween) return;
		const pick = pickSticker(event);
		if (!pick) return;
		clearHover();
		if (event.button === 2 || event.shiftKey) cube.eraseSticker(pick.facelet);
		else cube.paintSticker(pick.facelet, cube.activeColor);
	}

	function onPointerMove(event: PointerEvent) {
		if (pointerStart || tween) return;
		const pick = pickSticker(event);
		if (!pick) {
			clearHover();
			return;
		}
		if (hovered && hovered.mesh === pick.mesh && hovered.materialIndex === pick.materialIndex) {
			return;
		}
		clearHover();
		hovered = pick;
		(pick.mesh.material as THREE.MeshStandardMaterial[])[pick.materialIndex].emissive.setHex(
			0x2a2f3a
		);
	}

	function onPointerLeave() {
		clearHover();
	}

	function clearHover() {
		if (!hovered) return;
		const materials = hovered.mesh.material as THREE.MeshStandardMaterial[];
		materials[hovered.materialIndex]?.emissive.setHex(0x000000);
		hovered = null;
	}

	// -- rendering ----------------------------------------------------------

	const timer = new THREE.Timer();
	if (typeof document !== 'undefined') timer.connect(document);

	function renderFrame(timestamp?: number) {
		timer.update(timestamp);
		const delta = timer.getDelta();
		if (tween) {
			tween.elapsed += delta * 1000;
			const t = Math.min(1, tween.elapsed / tween.duration);
			// ease in and out, so a turn starts and stops smoothly
			const eased = t < 0.5 ? 2 * t * t : 1 - 2 * (1 - t) * (1 - t);
			tween.group.rotation[tween.axis] = tween.angle * eased;
			if (t >= 1) {
				const finished = tween;
				tween = null;
				finished.onDone();
			}
		}
		controls.update();
		renderer.render(scene, camera);
	}

	function resize() {
		const width = container.clientWidth || 1;
		const height = container.clientHeight || 1;
		renderer.setSize(width, height, false);
		camera.aspect = width / height;
		camera.updateProjectionMatrix();
	}
</script>

<div class="cube-view" bind:this={container}>
	<div class="hint">
		<span><strong>Click</strong> a sticker to paint it</span>
		<span><strong>Right click</strong> or shift click to erase</span>
		<span><strong>Drag</strong> to orbit, <strong>scroll</strong> to zoom</span>
	</div>
	<div class="badge" class:solved={cube.isSolved} class:playing={cube.playing}>
		{#if cube.playing}
			playing {cube.cursor}/{cube.moves.length}
		{:else if cube.hasSolution}
			{cube.isSolved ? 'solved' : `${cube.cursor}/${cube.moves.length} moves`}
		{:else}
			no solution
		{/if}
	</div>
</div>

<style lang="scss">
	.cube-view {
		position: relative;
		width: 100%;
		height: 100%;
		min-height: 340px;
		border-radius: 14px;
		background:
			radial-gradient(circle at 30% 20%, rgba(185, 244, 218, 0.08), transparent 55%),
			radial-gradient(circle at 75% 80%, rgba(105, 140, 200, 0.12), transparent 60%),
			#0d1016;
		overflow: hidden;

		:global(canvas.cube-canvas) {
			display: block;
			width: 100%;
			height: 100%;
			cursor: crosshair;
			touch-action: none;
		}
	}

	.hint {
		position: absolute;
		inset: auto auto 12px 12px;
		display: flex;
		flex-direction: column;
		gap: 2px;
		font-size: 0.7rem;
		line-height: 1.4;
		color: #8b94a7;
		pointer-events: none;

		strong {
			color: #cbd5e1;
			font-weight: 600;
		}
	}

	.badge {
		position: absolute;
		top: 12px;
		right: 12px;
		padding: 4px 10px;
		border-radius: 999px;
		background: rgba(15, 19, 26, 0.8);
		border: 1px solid #232936;
		color: #9aa4b8;
		font-size: 0.7rem;
		font-variant-numeric: tabular-nums;
		letter-spacing: 0.02em;
		pointer-events: none;

		&.solved {
			border-color: rgba(165, 237, 207, 0.5);
			color: #b9f4da;
		}

		&.playing {
			border-color: rgba(105, 140, 200, 0.6);
			color: #8fb0e0;
		}
	}
</style>
