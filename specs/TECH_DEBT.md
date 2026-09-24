# TECH_DEBT — open debt + accepted simplifications

Things rdom owes the codebase. The [`STABILIZE-2026-09`](STABILIZE-2026-09.md) program is paying every open item down before 0.5.0; each row's disposition is in that file. Every item has a stable ID so it can be referenced from PRs, commit messages, and code comments without quoting the whole entry.

For the durable architectural divergences (web-platform departures shipped on purpose, intended to stay), see [`DIVERGENCES.md`](DIVERGENCES.md). This file is for the *temporary* simplifications and known follow-ups.

## Open

### Style crate — from the STABILIZE-2026-09 Phase 3+4 gates

- **`STYLE-DISPATCH-SPLIT-1` — `property_dispatch.rs` (~2 300 lines) mixes the field table, `set`, `serialize` and their tests.** Split into `property_dispatch/{table,set,serialize}.rs` (tests alongside); no behavior change.
- **`STYLE-VALUES-SPLIT-1` — `parse/values.rs` (~1 800 lines) holds every value parser.** Split per value type (`color`, `length`, `calc` entry, `transition`, `content`); no behavior change. Also over the few-hundred-line bar: `layout.rs`, `ua.rs`, `tui_style.rs`, `stylesheet.rs`, and in `rdom-tui` `cascade/apply.rs` and `cascade/tests.rs`.

### Deferred from HARDENING-2026-09 Batch 3


### Paint pipeline

### UA stylesheet

### Substrate gaps

### Events


### Forms


## Accepted simplifications (forever-state)

These won't be paid down — they reflect deliberate architectural choices.

- **`D-M1-1` — `from_css` is a free function in `rdom-css`, not `impl Stylesheet`.** Original draft proposed inherent methods on `Stylesheet`. Couldn't ship — `rdom-tui` already depends on `rdom-css` for the inline-style cascade rung; making the inverse import work would require either a cycle or splitting `Stylesheet` to a third crate.
- **`D-M1-2` — `<style>` extraction + inline-style seeding are free functions called explicitly**, not auto-runs in `App::build`. Same cycle constraint as `D-M1-1`. Apps call `extend_from_style_tags(&dom, &mut sheet)` and `seed_inline_styles(&mut dom)` between `parse_into` and `App::new`.

## How to use this file

- **Adding an item.** ID format: `D-Mn-N` for milestone-specific deviations (where Mn is the milestone the work shipped in), `DRY-N` for refactor opportunities, `SUB-N` for substrate gaps, `OPS-N` for infrastructure, `M5-*` / `OPACITY-*` / `FOCUS-*` for topical groups.
- **Referring to an item.** Use the ID — `D-M2-2` etc. — in commit messages and PR descriptions.
- **Retiring an item.** Delete it. The historical context lives in the commit that retired it (`git log -S "D-M2-2"`).
