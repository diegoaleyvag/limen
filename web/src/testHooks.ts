// Test-only hooks exposed on `window`, used exclusively by the Playwright e2e suite
// (`web/e2e/**`) to read back the exact raw trial-result string currently held in memory for a
// byte-exact download verification -- there is no other reliable way for an external test
// process to obtain "the exact string the WASM call returned" without either re-invoking the
// engine a second time (which risks masking a real divergence rather than proving byte-exactness)
// or scraping rendered DOM text (which is lossy/reformatted). This module never influences
// production behavior: it only exposes a getter over state the app already holds, makes no
// network calls, and is a no-op if nothing has read it.

import type { TrialOutcome } from "./types";

export interface LimenTestHooks {
  getLastRaw: (column: "a" | "b") => string | null;
}

declare global {
  interface Window {
    __limenTestHooks__?: LimenTestHooks;
  }
}

/** Installs `window.__limenTestHooks__`, backed by the given accessor for each column's latest
 * `TrialOutcome`. Safe to call in any environment (no-op beyond a plain property assignment). */
export function installTestHooks(
  getLatestOutcome: (column: "a" | "b") => TrialOutcome | null,
): void {
  if (typeof window === "undefined") return;
  window.__limenTestHooks__ = {
    getLastRaw: (column) => getLatestOutcome(column)?.raw ?? null,
  };
}
