# STATE — project ledger

Current focus, release track, open risks, and the last few decisions for `rdom`. Kept short on purpose:
the dated journal from 2026-05/06 lives in [`specs/HISTORY-2026-05.md`](specs/HISTORY-2026-05.md), the
per-release notes in [`CHANGELOG.md`](CHANGELOG.md), the architecture in
[`specs/DESIGN.md`](specs/DESIGN.md), deliberate web departures in
[`specs/DIVERGENCES.md`](specs/DIVERGENCES.md), and open debt in [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md).

## Current focus

**[`specs/HARDENING-2026-09.md`](specs/HARDENING-2026-09.md)** — the program opened by the 2026-09-21
full-project review: fix the substrate invariants the review found broken, audit every DIVERGENCES and
TECH_DEBT entry against the code, and pay the debt down in crate-grouped batches so version bumps batch.

| Batch | Crates | Status |
|---|---|---|
| 1 | `rdom-core` | done, gated (architect + API), follow-ups landed |
| 2 | `rdom-style`, `rdom-css` | done, gated, follow-ups landed |
| 3 | `rdom-tui` | done, gated (architect + API); blocking follow-ups landed (flex pass budget, scrolled-IFC hit test, activation-behavior hook for checkboxes, UA `select[size]`, interval self-clear, dialog focus return, clock sync) |
| 4 | `rdom-parser` | done (R10: text `<`, RAWTEXT / RCDATA, entities, DOCTYPE); gates pending |

Release plan: 0.4.0 across every crate whose source changed (all five — `rdom-core` changed, and every
other crate pins it). Nothing has been published from this program yet.

## Release track

| Version | Date | Scope |
|---|---|---|
| 0.1.0 | 2026-05-19 | Initial release: DOM substrate, cascade, flexbox, runtime, builtins, UA sheet, CSS + HTML parsers; editing parity 2026-05-20 |
| 0.2.0 | 2026-06-02 | All five crates: showcase, event surface bundle, `calc()`, BFC, native ARIA tree, layered border model |
| 0.3.0 – 0.3.4 | 2026-06-02/03 | Substrate honesty for `rdom-extensions`; focus / `:where()` / `drop_subtree` fixes (`rdom-core` 0.3.4 → 0.3.5 at 0.3.11) |
| 0.3.5 – 0.3.14 | 2026-06-03 → 06-06 | `rdom-tui`-only patch line driven by `rdom-virtualtable`: table column sizing, stale-layout fixes, half-block borders, drag-autoscroll and its robustness follow-ups. Latest published: **`rdom-tui` 0.3.14**, tag `rdom-tui-v0.3.14` |
| 0.4.0 | planned | HARDENING-2026-09: generational `NodeId`, spec-correct dispatch and document position, whole-literal CSS numbers, at-rule recovery, CSS-wide keywords, `pointer-events`, scrollable text leaves, flex §9.7, and the rest of the program. Breaking notes in CHANGELOG "Unreleased" |
| 0.5.0 | planned | Client-side routing primitive (slid from 0.4.0) |
| later | — | Async tasks during event handlers; `TABLE-TFC-1` real table formatting context |

## Open risks

- **Batch 4 gates have not run yet** (rdom-parser is small; run both passes before the 0.4.0 publish).
- **`CARET-REVEAL-STALE-LAYOUT-1`** and **`POINTER-EVENTS-IFC-1`** are accepted for 0.4.0 (see TECH_DEBT).
- **`PROC-TOOLCHAIN-PIN-1`** — the toolchain floats on `stable`; a new stable can break `-D warnings` on
  every PR.
- **`SHOWCASE-EVT-1`** — the showcase's event surface (`AppContext`) exposes only redraw / quit /
  dispatch; consumers needing more reach into the App.
- **`ITERM2-MOUSE-MOTION-1`** — iTerm2 motion reporting quirk, external.

## Follow-ups

Run the Batch 4 gates, then the 0.4.0 release with `/publish` (all five crates; migration notes are in
CHANGELOG "Unreleased"). After the release: the deferred TECH_DEBT items from this program —
`CSS-VARS-SCOPE-1`, `FORM-DEFAULTS-1`, the file splits, the allocation items, `CARET-REVEAL-STALE-LAYOUT-1`,
`POINTER-EVENTS-IFC-1`, `PROC-TUI-DEV-DEP-1`, `PROC-TOOLCHAIN-PIN-1`.

## Recent decisions

- **2026-09-24 — Batch 3 slices landed without gates.** Unit + workspace tests gate each commit; the
  architect / API passes run once at the end of the batch (recorded as an open risk above).
- **2026-09-24 — `unset` is resolved at parse time** from `property_dispatch::inherits`; the cascade's
  `INHERITS_MASK` must agree and a test pins the two (`STYLE-INHERITS-TWO-SOURCES-1`).
- **2026-09-24 — Custom properties stay `:root`-only for now**, but silently dropping them is over:
  every other scope warns. Per-element scope is a Batch 3 cascade item.
- **2026-09-21 — Generational `NodeId`.** Slot reuse stays; the handle carries a generation so a stale
  id is rejected everywhere instead of aliasing the slot's next occupant.
- **2026-09-21 — Web-faithful dispatch and document position.** Two-pass target dispatch, flag reset
  at dispatch end, `compareDocumentPosition` bits oriented like the web; consumers that compensated for
  the inversion must flip their checks (CHANGELOG "Breaking — rdom-core").

## Decision archive

Older decisions worth preserving past their immediate context. New decisions land in §Recent decisions; rotate down here when they're no longer "recent."

(empty)
