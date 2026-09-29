# TECH_DEBT — open debt + accepted simplifications

Things rdom owes the codebase. The [`STABILIZE-2026-09`](STABILIZE-2026-09.md) program paid every open item down for 0.5.0; each row's disposition is in that file. Every item has a stable ID so it can be referenced from PRs, commit messages, and code comments without quoting the whole entry.

For the durable architectural divergences (web-platform departures shipped on purpose, intended to stay), see [`DIVERGENCES.md`](DIVERGENCES.md); for permanent architectural choices, see the decision archive in [`DESIGN.md`](DESIGN.md#decision-archive). This file is for the *temporary* simplifications and known follow-ups.

## Open

None.

## Accepted simplifications

None. A review-gate finding accepted as a risk is recorded here until it is paid down; a choice that is permanent goes to the [`DESIGN.md` decision archive](DESIGN.md#decision-archive) instead (`D-M1-1` moved there in 0.5.0).

## How to use this file

- **Adding an item.** ID format: `D-Mn-N` for milestone-specific deviations (where Mn is the milestone the work shipped in), `DRY-N` for refactor opportunities, `SUB-N` for substrate gaps, `OPS-N` for infrastructure, `M5-*` / `OPACITY-*` / `FOCUS-*` for topical groups.
- **Referring to an item.** Use the ID — `D-M2-2` etc. — in commit messages and PR descriptions.
- **Retiring an item.** Delete it. The historical context lives in the commit that retired it (`git log -S "D-M2-2"`).
