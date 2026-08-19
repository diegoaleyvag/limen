# Product

## Register

product

## Users

Engineers and researchers who work with LLM context windows and want to reason precisely about
context-selection tradeoffs: what a strategy keeps, what it drops, and what that costs in terms of
facts, citations, and contradictions. They arrive already knowing what a token budget is and what
a context window is; they do not need the product to explain LLMs to them. Their context of use is
a focused desk session, not a glance-and-go dashboard: they pick a scenario, pick a budget, pick two
strategies to compare, and read closely.

The job to be done: "given this task and this budget, what does strategy A keep that strategy B
drops, and does that difference matter?" The product succeeds when a user can answer that question
by reading the screen, not by trusting a headline number.

## Product Purpose

Limen is a deterministic lab for comparing context-selection strategies under fixed token budgets.
It runs five mechanical, versioned strategies against three authored scenarios entirely offline, in
the browser, via a compiled Rust/WASM engine: there is no server, no live model call, and no network
request of any kind at runtime.

Limen does not claim to benchmark real model quality. It isolates the context-construction decision
so its tradeoffs are visible and reproducible. Metrics evaluate selected context against annotated
expected facts; they do not claim downstream LLM answer accuracy. Every result is a byte-exact,
re-hashable artifact: the same scenario, strategy, and budget always produce the same JSON, and that
JSON, not a paraphrase of it, is what a user downloads.

Success looks like a user trusting the tool enough to cite a specific number from it ("recency
dropped the root-cause fact because it's the oldest item in the scenario") rather than a vague
impression ("recency seemed worse").

## Brand Personality

Precise, analytical, instrument-like. A lab bench, not a demo stage. Three words: exact, legible,
unornamented. The tone throughout (copy, layout, motion) should read like the documentation for a
measuring instrument: it tells you what it measured and how, then stops.

## Anti-references

- Flashy AI-product aesthetics: gradient hero sections, glowing orbs, "magic" sparkle iconography,
  anything that visually implies the tool is a generative AI product.
- Hero-metric SaaS templates: a big number, a small label underneath, a supporting-stats row, a
  gradient accent behind it all. This product has no single "score" to hero; it has a comparison.
- Any implication that a strategy, or the tool itself, "understands," is "smart," or exhibits
  "intelligence." Every strategy is a fixed, auditable mechanism and must be described as one:
  "keeps," "extracts sentences containing a digit," "ranks by lexical overlap with the query,"
  "template-based." This is a hard content constraint, not a style preference: it is the same
  integrity policy that makes the metrics themselves trustworthy.
- Generic dashboard chrome: card grids of identical KPI tiles, decorative icons with no semantic
  function, motion that plays on load rather than in response to a user action.

## Design Principles

- **Show the mechanism, not a verdict.** Every strategy names exactly what it does ("keeps items
  from the start until the budget runs out"), never what it's good at. Judgment stays with the
  user; the product supplies evidence.
- **Every number carries its denominator.** Fact recall is never "7"; it is "7 / 9 required
  facts retained." Budget is never "180 used" alone; it is "180 / 300 used, 120 remaining."
  Comparisons show the value and the delta, never the delta alone.
- **State is never color-only.** Included, partial, and dropped are each a distinct icon and word
  before they are a color. A user who cannot perceive the color difference must still be able to
  read the outcome.
- **Native controls, no invented affordances.** A `<select>` is a `<select>`. A range gets a native
  slider plus a native number input kept in sync. Nothing here needs a custom dropdown or a modal
  to feel considered; familiarity is the affordance.
- **The artifact is the ground truth.** What a user downloads is the literal string the engine
  returned, not a re-serialized copy. The UI renders a view of the data; it never becomes a second
  source of truth for it.

## Accessibility & Inclusion

Target WCAG 2.1 AA as the floor. Every control must be fully keyboard-operable with a visible focus
state that is never removed without an equally visible replacement. Every status that is conveyed by
color must also be conveyed by text or an icon glyph, so the product remains legible to color-blind
users and in grayscale printouts alike. Motion must respect `prefers-reduced-motion` and must never
be the only signal that something changed: an `aria-live="polite"` region announces result updates
and errors for screen-reader users. At narrow (mobile/tablet) widths the two-column comparison
stacks vertically with a persistent strategy label, so scroll position never costs a user their
sense of which column they are reading.
