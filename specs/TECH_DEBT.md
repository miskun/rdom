# TECH_DEBT — open debt + accepted simplifications

Things rdom owes the codebase. The [`STABILIZE-2026-09`](STABILIZE-2026-09.md) program paid every open item down for 0.5.0; each row's disposition is in that file. Every item has a stable ID so it can be referenced from PRs, commit messages, and code comments without quoting the whole entry.

For the durable architectural divergences (web-platform departures shipped on purpose, intended to stay), see [`DIVERGENCES.md`](DIVERGENCES.md); for permanent architectural choices, see the decision archive in [`DESIGN.md`](DESIGN.md#decision-archive). This file is for the *temporary* simplifications and known follow-ups.

## Open

- **`ATOM-BOX-1` — an inline block inside a line paints only its content.** Found by C5G-INLINE-BLOCK-SHADOW. In an inline formatting context (`<p><i>a</i><span style="display:inline-block">b</span></p>`) the atom is one row tall and `paint_inline_layout` paints its `::before` / text / `::after` and (since that item) its outer shadows — not its background, border, padding or inset shadows. Beside bare text, where the block lays the run out through an anonymous box, the atom is painted twice: as a box at its layout rect (`recurse_children` → `paint_plain`, which paints its shadows) and its content again at the line position; with a border the two disagree (the box is three rows, the line one). The fix is one owner: the line box lays the atom out at its full block size (with C9-VERTICAL-ALIGN) and paints it as a box at its turn, and the anonymous-box path stops reaching it as a block child. Until then the shadow paints once, with whichever path paints the box (`paints_child_box`).

## Accepted simplifications

A review-gate finding accepted as a risk is recorded here until it is paid down; a choice that is permanent goes to the [`DESIGN.md` decision archive](DESIGN.md#decision-archive) instead (`D-M1-1` moved there in 0.5.0).

- **`SCOPE-MEMO-1` — `@scope` memo memory is O(N × depth) per scope in one cascade pass.** `ScopeMemo` (`rdom-tui/src/style/cascade/scope.rs`, `C1G-SCOPE-COST`) keeps, per (sheet, `@scope`, node) the scoping roots that have the node in scope, with their generations, so each root and limit test runs once per pass. A node's list extends its parent's one generation farther, so it is a fresh `Rc<[(NodeId, u32)]>` per node: the bound is, per `@scope` rule set, the number of nodes the scoped rules are tested against times the number of scoping roots above each — at most N × depth entries of 12 bytes (a 5 000-node tree 20 deep with a root at every level: 100 000 entries, 1.2 MB, plus an `Rc` header and a map slot per node), freed when the pass ends (the memo lives in the pass's `Scratch`). Accepted at the Phase 2 gate: real `@scope` roots are few (components, not every level), so the lists are short; sharing a parent's list (generations stored relative to the parent) would make it O(N) but costs an extra walk per proximity read. Revisit if a profile shows the memo.

## How to use this file

- **Adding an item.** ID format: `D-Mn-N` for milestone-specific deviations (where Mn is the milestone the work shipped in), `DRY-N` for refactor opportunities, `SUB-N` for substrate gaps, `OPS-N` for infrastructure, `M5-*` / `OPACITY-*` / `FOCUS-*` for topical groups.
- **Referring to an item.** Use the ID — `D-M2-2` etc. — in commit messages and PR descriptions.
- **Retiring an item.** Delete it. The historical context lives in the commit that retired it (`git log -S "D-M2-2"`).
