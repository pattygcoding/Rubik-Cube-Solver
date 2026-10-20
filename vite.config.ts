import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		sveltekit({
			// SCSS in <style lang="scss"> blocks, plus the rest of the usual
			// preprocessing (TypeScript in <script> is handled by Svelte itself).
			preprocess: vitePreprocess(),

			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			// The site is a single prerendered page published to GitHub Pages from a
			// build artifact, so the static adapter writes plain HTML plus assets into
			// `build/`. See `.github/workflows/deploy.yml` and the README section
			// "Deploying to GitHub Pages".
			adapter: adapter()
		})
	]
});
