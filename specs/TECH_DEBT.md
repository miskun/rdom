# TECH_DEBT — open debt + accepted simplifications

Things rdom owes the codebase. The [`STABILIZE-2026-09`](STABILIZE-2026-09.md) program is paying every open item down before 0.5.0; each row's disposition is in that file. Every item has a stable ID so it can be referenced from PRs, commit messages, and code comments without quoting the whole entry.

For the durable architectural divergences (web-platform departures shipped on purpose, intended to stay), see [`DIVERGENCES.md`](DIVERGENCES.md). This file is for the *temporary* simplifications and known follow-ups.

## Open

- **`PERF-ROUTE-REDRAW-1` — a mouse route's redraw still re-cascades the whole tree.** `P7G-PAINT-ONLY-FRAME-1` split frame work into paint / layout / cascade (`runtime::app::redraw::Redraw`), but the public `RouteOutcome` has a single `redraw_requested` that carries both a listener's `request_redraw` (which may follow a direct `TuiExt` style write, so it must cascade) and the router's own tracked work (a hover change, a wheel scroll, a scrollbar press — layout + paint would do). The App cannot tell them apart, so those mouse events keep the whole-tree cascade. Fix with the 0.5.0 `#[non_exhaustive]` pass: a separate field for the router's own work.

## Accepted simplifications (forever-state)

These won't be paid down — they reflect deliberate architectural choices.

- **`D-M1-1` — `from_css` is a free function in `rdom-css`, not `impl Stylesheet`.** Original draft proposed inherent methods on `Stylesheet`. Couldn't ship — `rdom-tui` already depends on `rdom-css` for the inline-style cascade rung; making the inverse import work would require either a cycle or splitting `Stylesheet` to a third crate.
- **`D-M1-2` — inline-style seeding is a free function called explicitly**, not an auto-run in `App::build`. Apps call `seed_inline_styles(&mut dom)` between `parse_into` and `App::new` (later `style="…"` writes are live through the CSSOM observer). `<style>` elements no longer need a call: the `App` keeps their sheets live (`P7-LIVE-STYLE-1`); `extend_from_style_tags` remains as the snapshot form for a cascade without an `App`.

## How to use this file

- **Adding an item.** ID format: `D-Mn-N` for milestone-specific deviations (where Mn is the milestone the work shipped in), `DRY-N` for refactor opportunities, `SUB-N` for substrate gaps, `OPS-N` for infrastructure, `M5-*` / `OPACITY-*` / `FOCUS-*` for topical groups.
- **Referring to an item.** Use the ID — `D-M2-2` etc. — in commit messages and PR descriptions.
- **Retiring an item.** Delete it. The historical context lives in the commit that retired it (`git log -S "D-M2-2"`).
