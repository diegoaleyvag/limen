import { describe, expect, it } from "vitest";
import { sliceByUtf8ByteRange, utf8ByteLength } from "./bytes";

describe("utf8ByteLength", () => {
  it("matches string length for pure ASCII", () => {
    expect(utf8ByteLength("hello world")).toBe(11);
  });

  it("counts multi-byte UTF-8 scalars correctly", () => {
    // 'é' is 2 UTF-8 bytes but 1 JS UTF-16 code unit.
    expect(utf8ByteLength("café")).toBe(5);
    // '☕' (U+2615) is 3 UTF-8 bytes.
    expect(utf8ByteLength("☕")).toBe(3);
  });

  it("returns 0 for the empty string", () => {
    expect(utf8ByteLength("")).toBe(0);
  });
});

describe("sliceByUtf8ByteRange", () => {
  it("slices a pure-ASCII range identically to a plain JS substring", () => {
    const text = "Hello, world!";
    expect(sliceByUtf8ByteRange(text, 0, 5)).toBe("Hello");
    expect(sliceByUtf8ByteRange(text, 7, 12)).toBe("world");
  });

  it("correctly slices around a multi-byte UTF-8 scalar", () => {
    const text = "café"; // c=1 a=1 f=1 é=2 bytes -> byte offsets: c[0,1) a[1,2) f[2,3) é[3,5)
    expect(sliceByUtf8ByteRange(text, 0, 3)).toBe("caf");
    expect(sliceByUtf8ByteRange(text, 3, 5)).toBe("é");
    expect(sliceByUtf8ByteRange(text, 0, 5)).toBe("café");
  });

  it("returns an empty string for a zero-width or out-of-range slice", () => {
    const text = "hello";
    expect(sliceByUtf8ByteRange(text, 2, 2)).toBe("");
    expect(sliceByUtf8ByteRange(text, 100, 200)).toBe("");
  });
});
