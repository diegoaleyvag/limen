# Strategies

Limen ships five deterministic, versioned context-selection strategies. Each implements the
`SelectionStrategy` trait (`crates/limen-core/src/strategy/mod.rs`): given a task query and an
ordered list of source items (`StrategyInput`) plus a token `Budget`, it produces a
`SelectionOutput` containing exactly one `ItemSelectionRecord` per input item (sorted by
`order_index` ascending) and a `TraceStep`-per-item decision trace (in the strategy's own
processing order, which may differ from `order_index` order).

All five are intentionally simple and mechanical -- none of them call a model, and none of them
"understand" the text. That is the point: they are cheap, auditable baselines a lab can compare
real systems against, and their weaknesses are exactly as interesting as their behavior when they
work.

They split into two families:

- **Linear scan with a hard cutoff** (`full-input-truncation@1`, `recency@1`,
  `structured-extraction@1`, `hierarchical-summary@1`): walk items in some fixed order, keep
  including whole units (whole items, or, for `structured-extraction@1`, whole sentences) until
  one doesn't fit, then unconditionally drop everything from that point on. No later, smaller unit
  is ever reconsidered.
- **Greedy bin-packing by score** (`retrieval-ranking@1`): rank every item by an integer
  relevance score, then attempt *every* item in ranked order regardless of earlier misses. A
  smaller, lower-ranked item can end up included after a larger, higher-ranked one was skipped.

Notation used below: **N** = number of items in the scenario; **T** = total byte length of all
items' `text` combined; **S** = total number of sentences across all items (only relevant to
`structured-extraction@1`). Tokenizing any string of byte length `L` is `O(L)` (`limen-lex@1.0.0`
is a single linear scan; see `crates/limen-core/src/tokenizer.rs`).

## Action codes

Exact action-code strings emitted in `TraceStep.action`, per strategy (stable, used by the UI and
later documentation phases -- do not rename without updating both):

| Strategy | Action codes |
|---|---|
| `full-input-truncation@1` | `included_full`, `included_partial`, `dropped_over_budget` |
| `recency@1` | `included_full`, `included_partial`, `dropped_too_old` |
| `structured-extraction@1` | `extracted_included`, `extracted_partial`, `dropped_no_extractable_content`, `dropped_over_budget` |
| `hierarchical-summary@1` | `templated_included`, `dropped_over_budget` |
| `retrieval-ranking@1` | `included_full`, `dropped_below_budget` |

---

## `full-input-truncation@1` -- Full input truncation

**Processing order:** ascending `(order_index, source_id)` (a fully specified total order;
`source_id` is a documented tie-break that never actually fires, since `order_index` is already
unique per scenario).

**Algorithm:**
1. For each item in order, while no earlier item has failed to fit: if the item's verbatim `text`
   fits entirely within the remaining budget, include it whole (`included_full`).
2. The first item that does *not* fully fit is truncated at the last safe token boundary (tokenize
   the item, accumulate tokens while the running count stays within the remaining budget, cut at
   the `byte_end` of the last token taken): if at least one token fits, it is `Partial`
   (`included_partial`); if zero tokens fit, it is `Dropped` (`dropped_over_budget`). This is the
   one and only cutoff point.
3. Every item at or after the cutoff is unconditionally `Dropped` (`dropped_over_budget`) -- no
   bin-packing, no reconsideration.

**Complexity:** `O(N log N + T)` -- one sort of the items, one tokenizer pass over each item's text
(the tokenizer pass on the single cutoff item is the only one that needs the token-boundary
truncation logic; every other item is a whole-item fit check, still `O(len(item))`).

**Pedagogical purpose:** the naive baseline every other strategy is implicitly compared against --
"just concatenate everything until the window is full." It establishes what a context window looks
like with *zero* selection intelligence.

**Known weaknesses:** ignores relevance entirely; a single long early item can crowd out many
short, highly relevant later items; whatever is cut is gone with no attempt to save "the important
part" of that item or salvage cheaper-but-useful later items.

## `recency@1` -- Recency

**Processing order:** descending `(order_index, source_id)` -- newest item first. `order_index` is
the *only* recency signal used; this strategy never parses dates/timestamps out of `text`.

**Algorithm:** identical mechanics to `full-input-truncation@1` (whole-item verbatim inclusion,
safe token-boundary partial truncation at the one cutoff point, hard cutoff for everything beyond
it), but walked from newest to oldest instead of oldest to newest. The three action codes are
`included_full`, `included_partial`, and `dropped_too_old` (replacing
`full-input-truncation@1`'s generic `dropped_over_budget` for the cutoff and everything after it,
since here the reason really is "older than what the budget could keep"). `selection` is still
emitted sorted by `order_index` ascending; only the `trace` reflects the newest-first processing
order.

**Complexity:** `O(N log N + T)` -- same shape as `full-input-truncation@1`, just sorted the other
way.

**Pedagogical purpose:** models "keep the tail of the conversation/log" -- a very common naive
context-management heuristic in real chat/agent systems. Demonstrates that recency alone is a
cheap, order-only signal that requires no text understanding at all.

**Known weaknesses:** discards potentially-critical older evidence purely because it is old, even
when it is short and highly relevant (e.g. an early root-cause log line in an incident
investigation); provides no protection against recent-but-irrelevant "chatter" crowding out an
important old fact.

## `structured-extraction@1` -- Structured extraction

**Processing order:** ascending `(order_index, source_id)`.

**Algorithm:** a *transform* strategy -- it populates `output_text` rather than only
keeping/dropping verbatim text.
1. Split each item's `text` into sentences by scanning for `.`, `!`, `?`, or `\n` as delimiters
   (the delimiter stays attached to the sentence it ends; a trailing undelimited remainder is its
   own sentence). Sentences are atomic and are never split.
2. Keep only sentences containing at least one ASCII digit (`0`-`9`) -- the deterministic,
   mechanical stand-in for "looks like a number or date."
3. An item with zero kept sentences has nothing extractable: `Dropped`
   (`dropped_no_extractable_content`). This is *not* a budget cutoff -- the next item is still
   considered normally.
4. Otherwise, greedily grow the included prefix of *that item's own* kept sentences (in order)
   while the cumulative token count of the sentences joined so far (single space between them)
   stays within the remaining budget. All fit -> `Included` (`extracted_included`); some but not
   all fit -> `Partial` (`extracted_partial`) and this *is* the hard-cutoff point; none fit (but at
   least one kept sentence existed) -> `Dropped` (`dropped_over_budget`), which is also the
   cutoff. Every item after a budget-triggered cutoff is unconditionally `Dropped`
   (`dropped_over_budget`), whether or not it has extractable content.
5. `output_text` is the joined text of the kept sentences actually included; the record's
   `included_byte_start`/`included_byte_end` still point at the corresponding span of the
   *original* `text` (from the start of the first included kept sentence to the end of the last),
   preserving real provenance even though `output_text` is a filtered/joined extraction rather
   than that exact verbatim slice. Token counts for budget purposes are computed on `output_text`,
   never on the raw item text.

**Complexity:** `O(N log N + T)`. Splitting an item into sentences and tokenizing each kept
sentence once are both linear scans over that item's text; the cumulative-fit check is done by
precomputing each kept sentence's own token count once and prefix-summing (`O(S)` additions total
across all items), rather than by re-tokenizing the ever-growing joined string at every step
(which would cost `O(S * T)` in the worst case) -- these are provably equivalent here because
every join point between two kept sentences sits at an existing delimiter (already its own
punctuation token, or already whitespace), so joining with one more explicit space can never merge
or split a token.

**Pedagogical purpose:** demonstrates a strategy that changes *what* is kept, not just *which
whole items* are kept -- a crude, fully mechanical, explainable proxy for "extract the facts and
discard the prose."

**Known weaknesses:** the "contains a digit" rule is purely lexical: it shreds ordinary prose that
happens to lack digits even when it is important (e.g. a causal explanation with no numbers in
it), and it can equally keep a digit-bearing sentence that is numerically irrelevant (e.g. a
distractor date). It has no concept of which numbers matter more than others, and splitting on
`.`/`!`/`?`/`\n` alone will mis-segment abbreviations, decimals, and other punctuation-heavy text.

## `hierarchical-summary@1` -- Hierarchical summary (deterministic/template-based)

**This strategy is explicitly deterministic and template-based. It is never LLM-generated, and its
output must never be described or presented as such.** The only user-facing label for its output
is exactly **"Hierarchical summary (deterministic/template-based)"**. Every "summary" is a fixed
wrapper around a verbatim fragment of the source item's own text -- no paraphrasing, no generative
model, no semantic understanding.

**Processing order:** ascending `(order_index, source_id)`.

**Algorithm:** a *transform* strategy.
1. For each item, build a one-line snippet from `text`: the first occurrence of `.`, `!`, `?`, or
   `\n` (inclusive of the delimiter) if one starts within the first 200 bytes; otherwise `text`
   truncated at the largest safe token boundary at or before byte 200, with `"..."` appended
   (never splitting a UTF-8 codepoint).
2. Wrap it: `output_text = "[{section_label}] {source_id}: {snippet}"`.
3. This wrapped string is atomic and whole-or-nothing -- this strategy never produces `Partial`.
   Token counts are on the *wrapped* string (the label prefix counts against budget too). If it
   fits in the remaining budget: `Included` (`templated_included`), with
   `included_byte_start`/`included_byte_end` set to the *full* original item's `[0, text.len())`
   -- a full-item provenance pointer, even though only a summary was templated. If it does not
   fit: `Dropped` (`dropped_over_budget`), and this is the hard-cutoff point -- every later item is
   unconditionally dropped the same way.

**Complexity:** `O(N log N + T)` -- one linear scan per item to find the first delimiter (or the
200-byte token-boundary fallback), one tokenizer pass over the (short, bounded) wrapped string.

**Pedagogical purpose:** models a rollup/executive-summary view -- "show me one line per source,
labeled by where it came from" -- while staying fully deterministic and auditable, in deliberate
contrast to a real (non-deterministic, generative) summarizer. It demonstrates both the appeal
(dense, structured, well-labeled output) and the mechanical limits of a template-only approach.

**Known weaknesses:** it is explicitly *not* real summarization -- it has no way to identify the
actually-important sentence in an item, so if the key fact is not in the first ~200 bytes / first
sentence, it is silently lost even though the item is nominally "included." Being whole-or-nothing
per item, it also cannot partially salvage a near-fit the way the linear-scan-with-truncation
strategies can.

## `retrieval-ranking@1` -- Retrieval ranking

**Processing order:** by integer lexical score, descending (see below); this is the one strategy
that does not process items in (or against) `order_index` order.

**Algorithm:** greedy bin-packing by score, not a hard cutoff.
1. Tokenize `task_query`; keep only its ASCII-alphanumeric tokens, lowercased byte-for-byte (ASCII
   case-folding only, never locale-aware), *preserving duplicates* -- the query term list.
2. For each item, build the same kind of term list from its own `text`.
3. Score = the symmetric bag-overlap count: for every occurrence of a term in the query term list
   (duplicates counted separately), add 1 for every occurrence of that exact term anywhere in the
   item's term list. Integer-only; not normalized by item length.
4. Rank items by score descending; tie-break `order_index` ascending; final tie-break `source_id`
   ascending (documented total order; the final tie-break never actually fires since `order_index`
   is already unique). Every trace step records `score: Some(score)`, whether or not the item ends
   up included.
5. Walk the ranked list top to bottom, attempting *every* item (never stopping at the first miss):
   if it fits fully in the remaining budget, include it whole (`included_full`) and deduct its
   tokens; otherwise mark it `Dropped` (`dropped_below_budget`) and move on to the next ranked item
   without truncating and without cutting off the rest. Never produces `Partial`.

`selection` is emitted sorted by `order_index` ascending; only the `trace` reflects the
score-ranked processing order.

**Complexity:** `O(N log N + N * Q * I + T)`, where `Q` is the number of query terms and `I` is the
average number of item terms -- scoring is a direct nested scan over the two term lists (no
`HashMap`, per the crate's determinism rules), which is more than adequate at the scenario sizes
this lab targets but would need an actual inverted index or sorted-merge to scale to large corpora
or long queries.

**Pedagogical purpose:** the one strategy that tries to be *relevant* rather than merely
positional, and the one strategy that bin-packs instead of doing a hard linear cutoff -- it
directly demonstrates that a smaller, lower-ranked item can still make it into the context after a
larger, higher-ranked item was skipped, which none of the other four strategies can ever do.

**Known weaknesses:** depends entirely on exact (case-folded) lexical term overlap -- it has no
synonym/paraphrase awareness, so relevant content phrased differently from the query scores zero
and can be dropped ahead of an irrelevant item that happens to repeat query words verbatim; it is
also trivially gamed by keyword-stuffing.
