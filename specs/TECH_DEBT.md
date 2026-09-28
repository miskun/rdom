# TECH_DEBT — open debt + accepted simplifications

Things rdom owes the codebase. The [`STABILIZE-2026-09`](STABILIZE-2026-09.md) program is paying every open item down before 0.5.0; each row's disposition is in that file. Every item has a stable ID so it can be referenced from PRs, commit messages, and code comments without quoting the whole entry.

For the durable architectural divergences (web-platform departures shipped on purpose, intended to stay), see [`DIVERGENCES.md`](DIVERGENCES.md). This file is for the *temporary* simplifications and known follow-ups.

## Open

- **`DISPATCH-RESULTS-2` — 46 runtime dispatches still discard their `Result`.** `P7G-DISPATCH-RESULTS-1` fixed the four added in Phases 6–7 (form `submit` / `reset`, validation `invalid`, `write_offsets`' `scroll`) through `tui_event::dispatch_to_live`, which maps the one error a fresh event can meet (a target dropped by an earlier listener) to the caller's documented outcome and panics on any other. The older `let _ = dom.dispatch_tui_event(...)` sites (focus / blur ceremony, keyboard and mouse routing, toggle / range / number / select / details / dialog / tree / label builtins, editing `beforeinput` / `input`, undo, timers, resize, implicit detach events, the `AppContext` / accessor dispatch, transition events) predate the phase; each needs its outcome decided the same way.

## Accepted simplifications (forever-state)

These won't be paid down — they reflect deliberate architectural choices.

- **`D-M1-1` — `from_css` is a free function in `rdom-css`, not `impl Stylesheet`.** Original draft proposed inherent methods on `Stylesheet`. Couldn't ship — `rdom-tui` already depends on `rdom-css` for the inline-style cascade rung; making the inverse import work would require either a cycle or splitting `Stylesheet` to a third crate.

## How to use this file

- **Adding an item.** ID format: `D-Mn-N` for milestone-specific deviations (where Mn is the milestone the work shipped in), `DRY-N` for refactor opportunities, `SUB-N` for substrate gaps, `OPS-N` for infrastructure, `M5-*` / `OPACITY-*` / `FOCUS-*` for topical groups.
- **Referring to an item.** Use the ID — `D-M2-2` etc. — in commit messages and PR descriptions.
- **Retiring an item.** Delete it. The historical context lives in the commit that retired it (`git log -S "D-M2-2"`).
