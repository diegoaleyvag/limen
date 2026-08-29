import { describe, expect, it } from "vitest";
import { buildTrialDownloadBlob, trialDownloadFilename, triggerDownload } from "./download";

describe("trialDownloadFilename", () => {
  it("builds a stable, descriptive filename from the three trial parts", () => {
    expect(trialDownloadFilename("incident-investigation", "recency@1", 250)).toBe(
      "limen-trial-incident-investigation-recency@1-250.json",
    );
  });
});

describe("buildTrialDownloadBlob", () => {
  it("wraps the exact untouched raw string, byte-for-byte, never a re-serialized copy", async () => {
    const raw = '{"z":1,"a":2,"result_digest":"sha256:deadbeef"}';
    const blob = buildTrialDownloadBlob(raw);
    expect(blob.type).toBe("application/json");
    const text = await blob.text();
    expect(text).toBe(raw); // byte-exact: same key order, same whitespace, same everything
  });

  it("preserves exact bytes even for non-ASCII content", async () => {
    const raw = '{"text":"café ☕"}';
    const blob = buildTrialDownloadBlob(raw);
    const text = await blob.text();
    expect(text).toBe(raw);
  });
});

describe("triggerDownload", () => {
  it("creates a transient anchor, clicks it, and cleans up the object URL", () => {
    const createdUrls: string[] = [];
    const revokedUrls: string[] = [];
    const originalCreate = URL.createObjectURL;
    const originalRevoke = URL.revokeObjectURL;
    URL.createObjectURL = () => {
      const url = `blob:mock-${createdUrls.length}`;
      createdUrls.push(url);
      return url;
    };
    URL.revokeObjectURL = (url: string) => {
      revokedUrls.push(url);
    };

    let clicked = false;
    const originalClick = HTMLAnchorElement.prototype.click;
    HTMLAnchorElement.prototype.click = function mockClick(this: HTMLAnchorElement) {
      clicked = true;
      expect(this.download).toBe("test.json");
      expect(this.href).toContain(createdUrls[0]);
    };

    try {
      triggerDownload("test.json", new Blob(["hello"], { type: "application/json" }));
    } finally {
      URL.createObjectURL = originalCreate;
      URL.revokeObjectURL = originalRevoke;
      HTMLAnchorElement.prototype.click = originalClick;
    }

    expect(clicked).toBe(true);
    expect(revokedUrls).toEqual(createdUrls);
  });
});
