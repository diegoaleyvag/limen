// Downloading a trial artifact. `buildTrialDownloadBlob` and `trialDownloadFilename` are pure
// (no DOM triggering), so a test can construct the exact bytes that would be saved and diff them
// directly against the engine's raw string, without needing a real browser download to fire.

export function trialDownloadFilename(
  scenarioId: string,
  strategyId: string,
  requestedTokens: number,
): string {
  return `limen-trial-${scenarioId}-${strategyId}-${requestedTokens}.json`;
}

/** Builds the exact `Blob` that will be saved: constructed directly from the untouched raw
 * string the engine returned, never from `JSON.parse` followed by `JSON.stringify`. */
export function buildTrialDownloadBlob(rawTrialResultJson: string): Blob {
  return new Blob([rawTrialResultJson], { type: "application/json" });
}

/** Triggers a real browser download of `blob` named `filename`, via a transient anchor element.
 * Kept separate from `buildTrialDownloadBlob` so tests can verify the byte-exact content without
 * needing to simulate an actual click/save. */
export function triggerDownload(filename: string, blob: Blob): void {
  const url = URL.createObjectURL(blob);
  try {
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    anchor.rel = "noopener";
    document.body.appendChild(anchor);
    anchor.click();
    document.body.removeChild(anchor);
  } finally {
    URL.revokeObjectURL(url);
  }
}

export function downloadTrialResult(
  scenarioId: string,
  strategyId: string,
  requestedTokens: number,
  rawTrialResultJson: string,
): void {
  const filename = trialDownloadFilename(scenarioId, strategyId, requestedTokens);
  const blob = buildTrialDownloadBlob(rawTrialResultJson);
  triggerDownload(filename, blob);
}
