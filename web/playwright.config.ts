import { defineConfig, devices } from "@playwright/test";

// Chromium only, per the verification plan: real WASM loading, keyboard operability,
// accessibility structure, color-independent states, mobile stacking, reduced motion,
// byte-exact downloads, and same-origin-only network access, all against a real served
// production build (`vite build` then `vite preview`) -- never the dev server, so this exercises
// exactly the static assets that ship.
const PORT = 4173;
// `vite preview`'s default bind resolves "localhost" to the IPv6 loopback address on this host;
// using the literal `localhost` hostname (not the IPv4 `127.0.0.1` literal) here avoids a
// same-machine connection failure that would otherwise look like a hung/dead server.
const BASE_URL = `http://localhost:${PORT}`;

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: [["list"]],
  timeout: 30_000,
  use: {
    baseURL: BASE_URL,
    trace: "retain-on-failure",
  },
  webServer: {
    // Always rebuilds first, so the served `dist` reflects the current source exactly (a stale
    // `dist` from a previous run could otherwise hide real regressions).
    command: `npm run build && npm run preview -- --port ${PORT} --strictPort`,
    url: BASE_URL,
    reuseExistingServer: false,
    timeout: 60_000,
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
