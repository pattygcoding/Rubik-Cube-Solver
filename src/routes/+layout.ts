/**
 * The site is one static page, so it is prerendered at build time: GitHub Pages
 * serves plain HTML and assets, and nothing here needs a browser during the
 * build. The two parts that do need one - the WebAssembly engine and the three.js
 * scene - both start in `onMount`.
 */
export const prerender = true;
