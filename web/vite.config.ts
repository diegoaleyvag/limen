import { defineConfig } from "vitest/config";

// No `vite-plugin-wasm` and no bundler-target build here: `crates/limen-wasm`'s `--target web`
// output is imported the same way any other ES module + co-located static asset is. Vite's own
// asset pipeline recognizes the `new URL('limen_wasm_bg.wasm', import.meta.url)` pattern inside
// the generated `limen_wasm.js` glue and fingerprints/copies the `.wasm` file automatically, for
// both `vite` (dev) and `vite build` (static `dist/`).
export default defineConfig({
  build: {
    target: "es2022",
    sourcemap: true,
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.{test,spec}.ts"],
    css: false,
  },
});
