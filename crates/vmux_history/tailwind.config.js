/** @type {import('tailwindcss').Config} */
// Run from this directory (`npm run build:css` after `dist/index.html` exists; see `build.rs`).
// Markup + utilities: `src/**/*.rs`, shell: `assets/index.html` (copied to `dist/` before Tailwind in builds).
module.exports = {
  content: [
    "./src/**/*.rs",
    "./assets/index.html",
    "./dist/index.html",
    // Shared webview helpers (`UiInputShell`, `Button`, …) live in vmux_ui; scan them so utilities are not purged.
    "../vmux_ui/src/**/*.rs",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
};
