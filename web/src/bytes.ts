// UTF-8 byte-offset helpers. Every byte offset the engine returns (`EvidenceSpan`,
// `included_byte_start`/`included_byte_end`) is a UTF-8 *byte* offset (see
// `crates/limen-core/src/tokenizer.rs`), which only coincides with a JS string index for
// pure-ASCII text. The three embedded scenarios are pure ASCII by authoring convention, but these
// helpers convert correctly regardless, so a future non-ASCII scenario could never silently
// mis-slice text in this UI. This is byte/index-encoding conversion only: it makes no decision
// about tokens, selection, or budget, so it does not duplicate any of `limen-core`'s logic.
const encoder = new TextEncoder();

export function utf8ByteLength(text: string): number {
  return encoder.encode(text).length;
}

/** Returns the substring of `text` covering UTF-8 byte range `[byteStart, byteEnd)`. */
export function sliceByUtf8ByteRange(text: string, byteStart: number, byteEnd: number): string {
  let bytesSeen = 0;
  let result = "";
  for (const ch of text) {
    const chBytes = encoder.encode(ch).length;
    if (bytesSeen >= byteStart && bytesSeen < byteEnd) result += ch;
    bytesSeen += chBytes;
    if (bytesSeen >= byteEnd) break;
  }
  return result;
}
