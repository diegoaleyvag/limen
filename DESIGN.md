---
name: Limen
description: A deterministic lab bench for comparing context-selection strategies under fixed token budgets.
colors:
  paper: "oklch(97% 0.006 82)"
  paper-panel: "oklch(94% 0.008 82)"
  paper-sunk: "oklch(91% 0.009 82)"
  hairline: "oklch(85% 0.012 82)"
  hairline-strong: "oklch(76% 0.014 82)"
  ink: "oklch(23% 0.014 82)"
  ink-muted: "oklch(46% 0.012 82)"
  ink-faint: "oklch(64% 0.01 82)"
  accent: "oklch(46% 0.15 250)"
  accent-hover: "oklch(40% 0.16 250)"
  accent-active: "oklch(35% 0.16 250)"
  accent-subtle: "oklch(94% 0.02 250)"
  success: "oklch(45% 0.13 145)"
  success-subtle: "oklch(94% 0.03 145)"
  warning: "oklch(50% 0.15 70)"
  warning-subtle: "oklch(94% 0.04 70)"
  danger: "oklch(50% 0.19 25)"
  danger-subtle: "oklch(94% 0.03 25)"
typography:
  xl:
    fontFamily: "-apple-system, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: "normal"
  lg:
    fontFamily: "-apple-system, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "1.25rem"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "normal"
  md:
    fontFamily: "-apple-system, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "1.0625rem"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "normal"
  base:
    fontFamily: "-apple-system, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "0.9375rem"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "normal"
  sm:
    fontFamily: "-apple-system, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "normal"
  xs:
    fontFamily: "-apple-system, \"Segoe UI\", system-ui, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: "0.01em"
  mono:
    fontFamily: "ui-monospace, \"SF Mono\", \"Cascadia Code\", Consolas, monospace"
    fontSize: "0.8125rem"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "normal"
rounded:
  sm: "4px"
  md: "8px"
spacing:
  1: "0.25rem"
  2: "0.5rem"
  3: "0.75rem"
  4: "1rem"
  5: "1.5rem"
  6: "2rem"
  7: "3rem"
  8: "4rem"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "oklch(98% 0.006 82)"
    typography: "{typography.sm}"
    rounded: "{rounded.sm}"
    padding: "0.5rem 0.875rem"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
    textColor: "oklch(98% 0.006 82)"
    typography: "{typography.sm}"
    rounded: "{rounded.sm}"
    padding: "0.5rem 0.875rem"
  button-secondary:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    typography: "{typography.sm}"
    rounded: "{rounded.sm}"
    padding: "0.5rem 0.875rem"
  badge-included:
    backgroundColor: "{colors.success-subtle}"
    textColor: "{colors.success}"
    typography: "{typography.xs}"
    rounded: "{rounded.sm}"
    padding: "0.125rem 0.5rem"
  badge-partial:
    backgroundColor: "{colors.warning-subtle}"
    textColor: "{colors.warning}"
    typography: "{typography.xs}"
    rounded: "{rounded.sm}"
    padding: "0.125rem 0.5rem"
  badge-dropped:
    backgroundColor: "{colors.paper-sunk}"
    textColor: "{colors.ink-muted}"
    typography: "{typography.xs}"
    rounded: "{rounded.sm}"
    padding: "0.125rem 0.5rem"
---

# Design System: Limen

## 1. Overview

**Creative North Star: "The Instrument Panel"**

Picture an engineer at a bright desk during working hours, laptop propped next to a notebook, running
the same scenario through two strategies side by side to see which one drops the fact that matters.
They need consistent color perception for status icons and low eye strain across a long comparison
session: a bright, neutral workbench light theme delivers that, while a dark theme would shift the
perceived hue and contrast of the very status colors the product depends on, and a "safe default"
light theme chosen without that reasoning would be an accident, not a decision. Limen commits to
light, permanently, for this reason.

Limen looks like the face of a measuring instrument, not a software dashboard: warm, matte paper-tone
surfaces (never a stark white, never a cold gray), fine hairline rules instead of drop shadows, dense
and legible tabular data, and exactly one accent color, a precise blueprint-blue the color of ink on
a drafting table, reserved for the things that are currently true: the primary action, the current
selection, the active state. Everything else is ink on paper. The product explicitly rejects flashy
AI-product aesthetics (gradients, glow, sparkle iconography), the hero-metric SaaS template (one big
number, small label, gradient card), and any visual implication that a strategy "understands"
anything: every strategy is a fixed mechanism, and the interface presents it as one.

**Key characteristics:** warm paper neutrals tinted with OKLCH, not gray; one accent color at low
coverage; status conveyed by icon and word first, color second; flat surfaces with hairline borders,
no shadows except one functional exception; a fixed rem type scale sized for dense tabular reading,
not display headlines.

## 2. Colors

The palette is warm and matte: every neutral is OKLCH-tinted toward hue 82 (a warm, paper-like
undertone, closer to unbleached drafting paper than to blue-gray "tech" neutrals), and the one accent
sits at hue 250, a deliberately different family so it is never confused with a tinted neutral.

### Primary

- **Blueprint** (`oklch(46% 0.15 250)`): the single accent. Used only for primary action buttons, the
  current scenario/strategy selection state, and active-control treatments (a focused range thumb, a
  checked native control). Never used decoratively, never used for a status that isn't "this is the
  thing currently acted on."

### Neutral

- **Workbench Paper** (`oklch(97% 0.006 82)`): the base canvas. The main content surface.
- **Panel Paper** (`oklch(94% 0.008 82)`): the second neutral layer, used for column headers, the
  sticky mobile strategy label, and the app toolbar. One step deeper than the canvas, never a shadow.
- **Sunk Paper** (`oklch(91% 0.009 82)`): inset wells such as the decision-trace disclosure body and
  the dropped-item badge fill. Reads as "recessed," matching a flat, non-shadowed elevation model.
- **Hairline** (`oklch(85% 0.012 82)`): the default rule/border color between rows, cards, and panels.
- **Hairline Strong** (`oklch(76% 0.014 82)`): heavier structural rules, such as the line under a
  table header row or the budget-boundary marker in the timeline.
- **Ink** (`oklch(23% 0.014 82)`): primary text. Warm near-black, never pure `#000`.
- **Ink Muted** (`oklch(46% 0.012 82)`): secondary text: captions, helper text, `why_it_matters` copy.
- **Ink Faint** (`oklch(64% 0.01 82)`): placeholder text, disabled control labels, the dropped-status
  badge text.

### Semantic (status vocabulary, always paired with an icon and a word, never color alone)

- **Included / Success** (`oklch(45% 0.13 145)` on `oklch(94% 0.03 145)`): an item fully retained.
- **Partial / Warning** (`oklch(50% 0.15 70)` on `oklch(94% 0.04 70)`): an item truncated at the budget
  boundary.
- **Dropped**: rendered in Ink Faint on Sunk Paper, deliberately *not* the danger color, since being
  dropped by a strategy is an expected, legitimate outcome, not a failure.
- **Danger** (`oklch(50% 0.19 25)` on `oklch(94% 0.03 25)`): reserved for real application errors
  (an unknown scenario/strategy id, a WASM call rejection), never for a routine "dropped" status.

### Named Rules

**The One Ink Rule.** There is exactly one accent hue (250) in the entire product. If a second
"exciting" color shows up anywhere outside the semantic status set above, it is a mistake, not a
feature.

**The No Pure Neutral Rule.** No surface, border, or text color is ever literal `#fff`, `#000`, or an
untinted gray. Every neutral carries the same hue-82 tint at varying lightness/chroma, so the whole
surface reads as one warm material, not "default browser gray."

## 3. Typography

**Body & UI Font:** `-apple-system, "Segoe UI", system-ui, sans-serif` (the native stack; no remote
font ever loads)
**Data/Mono Font:** `ui-monospace, "SF Mono", "Cascadia Code", Consolas, monospace`, used for token
counts, byte offsets, digests, and downloaded filenames.

**Character:** one system sans doing every job from page title to table cell, the way an instrument's
own printed labels use one typeface throughout. Density over drama: this is a fixed rem scale for
consistent on-screen reading, never `clamp()`/fluid sizing, because the audience reads at a
consistent desktop DPI, not a marketing viewport.

### Hierarchy

- **XL** (600, 1.5rem/24px, 1.25 line-height): the page title ("Limen"), once, at the top.
- **LG** (600, 1.25rem/20px, 1.3): section titles: "Budget timeline," "Metrics," "Comparison."
- **MD** (600, 1.0625rem/17px, 1.4): column/subsection headers: a strategy's own name above its
  results column, table header cells.
- **Base** (400, 0.9375rem/15px, 1.5): default body and control text; capped at 75ch for prose blocks
  like `why_it_matters` explanations.
- **SM** (400, 0.8125rem/13px, 1.45): secondary/dense text: source item text previews, trace detail
  strings, table data cells.
- **XS** (500, 0.75rem/12px, 1.4, +0.01em tracking): labels, badges, timestamps, status pills. Always
  paired with the icon glyph for that status, never carrying meaning by size/color alone.
- **Mono** (400, 0.8125rem/13px, 1.5): any exact machine value, such as a digest, a byte span, or a
  token count, so numerals align and are visually distinct from prose.

### Named Rules

**The Ratio Tightens Rule.** Step ratios run roughly 1.08 to 1.2, tightest at the smallest sizes
(XS to SM) and widest near the top (LG to XL). This is standard type-scale practice, not an
inconsistency: small text needs finer gradation to stay legible; large text needs bigger jumps to
read as a real hierarchy step.

## 4. Elevation

Limen is flat by default. There are no drop shadows used for hierarchy anywhere in the product;
depth is conveyed entirely by the neutral layering in Colors (Workbench Paper, then Panel Paper, then
Sunk Paper) plus hairline borders. This matches the instrument-panel metaphor: a real gauge face has
printed layers and engraved rules, not floating drop-shadowed cards.

### Shadow Vocabulary

- **Sticky Label Lift** (`box-shadow: 0 1px 3px oklch(23% 0.014 82 / 0.12)`): the one functional
  exception. Applied only to the sticky mobile strategy-name label while it is pinned during scroll,
  so it reads as physically separated from the content scrolling beneath it. This is a functional
  affordance (it must look "on top of," not "beside"), not decoration.

### Named Rules

**The Flat-By-Default Rule.** If you are reaching for a `box-shadow` anywhere other than the sticky
mobile label, use a hairline border or a Panel/Sunk Paper background step instead.

## 5. Components

Every interactive component below implements the full state set: default, hover, focus, active,
disabled, and, where relevant, loading, error, and selected. Focus is always a visible outline; it is
never removed without an equally visible replacement.

### Buttons

- **Shape:** 4px corner radius (`{rounded.sm}`), a small, precise radius, not a pill.
- **Primary** (`download`): Blueprint background, near-white text, `0.5rem 0.875rem` padding, SM
  typography (13px/500). The product recomputes reactively as soon as a control changes (every
  trial is a sub-100ms in-memory computation, not a network wait), so there is no separate "run"
  step; the one primary action per column is downloading its result.
  - *Hover:* darkens to `accent-hover`.
  - *Active:* darkens further to `accent-active`.
  - *Focus:* a 2px Blueprint outline offset 2px from the button edge, visible on top of any hover
    state.
  - *Disabled:* Ink Faint text on Panel Paper, no hover/active response, `cursor: not-allowed`
    (used only if a column has no result yet to download, e.g. immediately after an error).
- **Secondary** (rare; this product prefers one obvious primary action per view): Workbench Paper
  background, Ink text, Hairline border, same padding/radius/focus treatment as primary.

### Native Form Controls (select, range, number input)

Never reinvented. A `<select>` renders as the platform's own control; the shared budget control pairs
a native `<input type="range">` with a native `<input type="number">`, kept in sync in both
directions, both wrapped in one `<label>`.

- **Style:** Hairline border, Workbench Paper background, Ink text, SM typography, 4px radius,
  `0.375rem 0.5rem` padding for the number input.
- **Focus:** the browser's native focus ring is preserved (never `outline: none` without a
  replacement); a Blueprint 2px outline is applied consistently across browsers for a uniform look.
- **Disabled:** Ink Faint text, Sunk Paper background.
- **Range thumb active/dragging:** thumb fills solid Blueprint, indicating "you are changing the
  budget right now."

### Status Badges (Included / Partial / Dropped)

- **Style:** XS typography, `0.125rem 0.5rem` padding, 4px radius, a leading icon glyph (a check
  mark, a half-fill mark, or a cross) followed by the status word. Background/text colors per the
  semantic palette in Colors.
- **Never:** a colored left border as the only signal. If a status needs to be visible at a glance
  down a long list, the badge's own background tint carries that, not a stripe on the row's edge.

### Timeline (budget usage, per column)

A semantic `<ol>`, one `<li>` per source item in `order_index` order, each sized proportionally (by
share of the requested budget) via its inline `flex-basis`/width, so the bar reads as a literal
budget map. The exhausted-budget point renders as a full-height vertical rule in Hairline Strong with
a small "budget boundary" label, not a color change alone. Each segment repeats its status badge
(icon plus word) inside itself; color is a background tint layered under that badge, never the only
distinguishing feature between segments.

### Comparison Table

A real `<table>`: rows are metrics, columns are Strategy A and Strategy B, a third cell shows the
delta. Header row uses Panel Paper with a Hairline Strong bottom rule. Numeric cells are right-aligned
in the Mono font so digits line up.

### Navigation / Layout

No top bar or side nav is needed for a single-screen comparison tool; the "navigation" is the
scenario/budget controls themselves, always visible at the top, never scrolling out of view on
desktop. On mobile, Strategy A's name becomes a Panel Paper sticky label (with the one Shadow
Vocabulary exception above) so scroll position never costs a user their sense of which column they
are reading.

## 6. Do's and Don'ts

### Do:

- **Do** pair every status with an icon glyph and a word before color ever carries meaning alone.
- **Do** use the Blueprint accent (`oklch(46% 0.15 250)`) only for primary actions, current
  selection, and active state, never decoratively, and never above roughly 10% of any screen's
  surface.
- **Do** show every metric's denominator (`7 / 9 required facts`, `180 / 300 tokens used`), never a
  bare numerator.
- **Do** keep the visible focus outline on every control; if the browser default clashes visually,
  replace it with an equally visible Blueprint outline, never remove it outright.
- **Do** wrap every transition in a `prefers-reduced-motion: no-preference` guard, and keep the ones
  that remain in the 150 to 250ms range.
- **Do** persist the exact raw `run_trial` string for downloads; render parsed copies for on-screen
  display only.

### Don't:

- **Don't** use flashy AI-product aesthetics: gradients, glow, sparkle icons, or any visual
  implication that a strategy "understands" or is "smart"/"intelligent" (see `PRODUCT.md`'s
  Anti-references).
- **Don't** build a hero-metric template (one big number, small label, gradient card): this product
  is a comparison, not a single score.
- **Don't** use a side-stripe/left-border as a status indicator on any list item, row, or card.
- **Don't** use gradient text, glassmorphism, or any decorative blur.
- **Don't** reach for a modal as a first resort; this product has no flow that needs one.
- **Don't** reinvent a dropdown, slider, or native form control for "flavor."
- **Don't** use pure `#fff`/`#000` or an untinted gray anywhere; every neutral carries the hue-82
  tint.
- **Don't** use em dashes in any UI copy. Use commas, colons, semicolons, periods, or parentheses.
