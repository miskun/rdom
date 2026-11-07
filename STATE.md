# STATE — project ledger

Current focus, release track, open risks, and the last few decisions for `rdom`. Kept short on purpose:
the dated journal from 2026-05/06 lives in [`specs/HISTORY-2026-05.md`](specs/HISTORY-2026-05.md), the
per-release notes in [`CHANGELOG.md`](CHANGELOG.md), the architecture in
[`specs/DESIGN.md`](specs/DESIGN.md), deliberate web departures in
[`specs/DIVERGENCES.md`](specs/DIVERGENCES.md), and open debt in [`specs/TECH_DEBT.md`](specs/TECH_DEBT.md).

## Current focus

**CAPTURE — an agent-friendly renderer, inspector and design loop** ([`specs/CAPTURE.md`](specs/CAPTURE.md)),
a new published crate, `rdom-capture`, over `rdom-tui`: headless capture sessions with a deterministic
environment, paint traces and the inspector, edits without a rebuild, images from rdom's own rasterizer,
audits and motion. Nine phases, each closed by a chief-architect review recorded in the spec's log; the
release (a new `rdom-capture` and whatever substrate crates the phases change) follows the last phase.
Phases 1–4 closed (2026-11-07; phase 4's seven blocking review findings fixed — the cause of an inspector-started
animation now lives on the running instance). Phase 5 closed (its four blocking review findings fixed). Phase 6 closed (its six blocking review findings fixed). Phase 7 (audits) built and under its architect review; phase 8 (motion) next.
0.6.0 – 0.6.5 shipped (CSS completeness and the `rdom-tui` patch line); routing stays at 0.7.0.

## Release track

| Version | Date | Scope |
|---|---|---|
| 0.1.0 | 2026-05-19 | Initial release: DOM substrate, cascade, flexbox, runtime, builtins, UA sheet, CSS + HTML parsers; editing parity 2026-05-20 |
| 0.2.0 | 2026-06-02 | All five crates: showcase, event surface bundle, `calc()`, BFC, native ARIA tree, layered border model |
| 0.3.0 – 0.3.4 | 2026-06-02/03 | Substrate honesty for `rdom-extensions`; focus / `:where()` / `drop_subtree` fixes (`rdom-core` 0.3.4 → 0.3.5 at 0.3.11) |
| 0.3.5 – 0.3.14 | 2026-06-03 → 06-06 | `rdom-tui`-only patch line driven by `rdom-virtualtable`: table column sizing, stale-layout fixes, half-block borders, drag-autoscroll and its robustness follow-ups. Latest published: **`rdom-tui` 0.3.14**, tag `rdom-tui-v0.3.14` |
| 0.4.0 | **released 2026-09-24**, tag `v0.4.0`, all five crates on crates.io | HARDENING-2026-09: generational `NodeId`, spec-correct dispatch and document position, whole-literal CSS numbers, at-rule recovery, CSS-wide keywords, `pointer-events`, scrollable text leaves, flex §9.7, and the rest of the program. Breaking notes in CHANGELOG "0.4.0" |
| 0.5.0 | **shipped 2026-09-29** | STABILIZE-2026-09: empty TECH_DEBT open list, "stable and complete"; form validation, `:focus-visible`, live `<style>`. Breaking notes in CHANGELOG "0.5.0" |
| 0.6.0 | **shipped 2026-10-18** | CSS-COMPLETE-2026-10: every CSS feature meaningful in a terminal; acid test |
| 0.6.1 – 0.6.5 | 2026-10-29 → 11-05 | `rdom-tui`-only patch line: the layout flush, the client box, borders follow paint order, out-of-flow children leave the collapse group, flex shrink overflow |
| rdom-capture 0.1 | **in progress** | CAPTURE: headless capture sessions, inspector, edits, images, audits, motion |
| 0.7.0 | planned | Client-side routing primitive (slid from 0.6.0) |
| later | — | Async tasks during event handlers; `TABLE-TFC-1` real table formatting context |

## Open risks

- **`SHOWCASE-EVT-1`** — the showcase's event surface (`AppContext`) exposes only redraw / quit /
  dispatch; consumers needing more reach into the App.
- **`ITERM2-MOUSE-MOTION-1`** — iTerm2 motion reporting quirk, external.

## Follow-ups

[`STABILIZE-2026-09`](specs/STABILIZE-2026-09.md) closed with 0.5.0: every open TECH_DEBT row was fixed, moved to DIVERGENCES, or documented as external. Its log is the record. The items it started from —
`CSS-VARS-SCOPE-1`, `FORM-DEFAULTS-1`, the file splits, the allocation items, `CARET-REVEAL-STALE-LAYOUT-1`,
`POINTER-EVENTS-IFC-1`, `PARSER-VOID-TAGS-1`, `PARSER-ENTITIES-1`, `PROC-TUI-DEV-DEP-1`, `PROC-TOOLCHAIN-PIN-1`.

## Recent decisions

- **2026-11-07 — CAPTURE decisions** (spec §Decisions): rdom's own rasterizer and PNG encoder, no image
  or font dependencies; the xterm.js emulator view is optional; events caused by the inspector are
  suppressed, with cause tagging so delivering them later is additive. `rdom-capture` uses serde /
  serde_json (already in the lockfile) for its JSON contract and protocol.

- **2026-09-24 — Batch 3 slices landed without gates.** Unit + workspace tests gate each commit; the
  architect / API passes run once at the end of the batch (recorded as an open risk above).
- **2026-09-24 — `unset` is resolved at parse time** from `property_dispatch::inherits`; the cascade's
  `inherit_inheritable_from` is pinned to that table by a per-property test (`STYLE-INHERITS-TWO-SOURCES-1`,
  closed in STABILIZE).
- **2026-09-24 — Custom properties are per element** (CSS Variables 1) since STABILIZE `CSS-VARS-SCOPE-1`;
  the earlier `:root`-only rule is gone.
- **2026-09-21 — Generational `NodeId`.** Slot reuse stays; the handle carries a generation so a stale
  id is rejected everywhere instead of aliasing the slot's next occupant.
- **2026-09-21 — Web-faithful dispatch and document position.** Two-pass target dispatch, flag reset
  at dispatch end, `compareDocumentPosition` bits oriented like the web; consumers that compensated for
  the inversion must flip their checks (CHANGELOG "Breaking — rdom-core").

## Decision archive

Older decisions worth preserving past their immediate context. New decisions land in §Recent decisions; rotate down here when they're no longer "recent."

(empty)
