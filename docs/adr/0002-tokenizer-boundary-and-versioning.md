# 0002. Tokenizer boundary and versioning

## Context

Every trial's budget accounting, and several strategies' cutoff behavior, depend on counting
"tokens" in emitted context text. Limen needs a token-counting rule that is: exactly reproducible
across native Rust and compiled WASM (no locale-, platform-, or library-version-dependent
behavior); simple enough to specify and test exhaustively; and stable enough that a shipped
`TrialResult` remains meaningfully interpretable indefinitely, even if a later phase wants a
different or more realistic tokenization scheme.

Real third-party tokenizers (e.g. a BPE tokenizer matching a specific model family) would tie
Limen's results to that tokenizer's own vocabulary, version, and update cadence, none of which is
this foundation's concern -- Limen is explicitly not attempting to reproduce any specific model's
real token accounting (see the product's own non-claims in `README.md`/`docs/METRICS.md`).

## Decision

Limen bundles its own minimal, fully deterministic tokenizer, `limen-lex@1.0.0`
(`crates/limen-core/src/tokenizer.rs`), with an exact, small rule set:

- A maximal contiguous run of ASCII `[A-Za-z0-9]` bytes is one token.
- Every other individual non-whitespace ASCII byte (punctuation, symbols, stray control bytes) is
  its own one-byte token.
- Every individual non-ASCII Unicode scalar value is its own one token, regardless of its UTF-8
  byte length -- a multi-scalar grapheme cluster (e.g. a ZWJ-joined emoji sequence) therefore
  produces multiple tokens, one per scalar; this tokenizer has no concept of grapheme clusters.
- ASCII whitespace (space, tab, CR, LF) separates tokens and is never itself a token.
- Spans are UTF-8 byte offsets `[byte_start, byte_end)`, always landing on codepoint boundaries
  (this falls out of iterating `char_indices()` and always ending a span at a `char` boundary).

The tokenizer's identity travels with every result artifact: `TrialResult::tokenizer_id` is always
`crate::tokenizer::TOKENIZER_ID` (`"limen-lex@1.0.0"`), so a `TrialResult` is self-describing about
exactly which counting rule produced its token counts, independent of `engine_version`.

Budget accounting is always performed on the final *emitted* context text -- including any
strategy-added source-label prefixes or deterministic template wrapper text (e.g.
`hierarchical-summary@1`'s `"[{section_label}] {source_id}: "` prefix) -- never on raw,
unmodified source item text, so token counts reflect what a real model would actually be charged
for the assembled context, even though the tokenizer itself is a simplified stand-in.

## Consequences

- **Fully reproducible, testable token counts.** `tokenizer.rs`'s test suite (including
  multi-scalar-emoji and rich-mixed-text cases) exhaustively pins down the rule; there is no
  hidden dependency on ICU, locale data, or a third-party crate's own versioning.
- **Not a realistic token count for any specific real model.** This is a known, disclosed
  limitation, not a hidden one: Limen's budgets and metrics measure *relative* selection behavior
  under this tokenizer's counting rule, not what a specific production tokenizer would charge.
- **Versioning is a first-class, forward-compatible concern.** `TOKENIZER_ID` is a stable string
  constant baked into every result. A future `limen-lex@2.0.0` (or an entirely different
  tokenizer family) would be introduced as a new, separately-versioned module/constant, selected
  explicitly wherever tokenization happens -- never by silently changing what `"limen-lex@1.0.0"`
  means. Existing `TrialResult` artifacts (and their `result_digest`s) remain valid,
  self-describing, and reproducible forever, because they name the exact tokenizer version that
  produced them; a `@2` tokenizer simply produces artifacts tagged `"limen-lex@2.0.0"` alongside
  them, never retroactively.
- **No Unicode normalization.** Text is tokenized (and hashed, and byte-sliced) exactly as
  authored, with no NFC/NFKC normalization step; see `canonical.rs`'s module doc comment for why
  this is a deliberate choice shared with the digesting design (ADR 0003), not an oversight.

## Alternatives considered

- **A third-party BPE tokenizer (e.g. matching a specific open model's vocabulary).** Rejected for
  this foundation: it would tie every result to that tokenizer's own external versioning and
  vocabulary file, undermining the "same manifest+strategy+budget yields the same result digest"
  guarantee across time (a vocabulary update would silently change token counts) and would imply
  a realism claim ("this is what tokens cost for model X") this product explicitly disclaims.
- **Byte-length or whitespace-word-count as the budget unit.** Rejected: both are even further
  from any realistic notion of a token, and word-count in particular breaks down for
  punctuation-heavy or non-ASCII text in ways that would make budget behavior hard to reason
  about precisely.
- **Grapheme-cluster-aware tokenization for non-ASCII text.** Rejected for this foundation's scope:
  meaningfully more complex to implement and specify correctly, and not needed by the three
  authored (primarily ASCII) scenarios; documented as a known simplification instead
  (`tokenizer.rs`'s own doc comment on multi-scalar sequences).
