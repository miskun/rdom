# Contributing to rdom

Thanks for your interest. This is a small project and we keep it boring, correct, testable, and explicit. The full guide for both humans and AI coding agents lives in [`CLAUDE.md`](CLAUDE.md) (a.k.a. [`AGENTS.md`](AGENTS.md) — the symlink). Read that first.

## TL;DR for humans

1. **Read the code and the tests.** rdom tracks the web platform (WHATWG DOM, CSS, UI Events) by default. [`specs/DIVERGENCES.md`](specs/DIVERGENCES.md) lists every deliberate departure; [`specs/DESIGN.md`](specs/DESIGN.md) covers the architecture.
2. **TDD.** Write the failing test first, then the smallest implementation that makes it pass. `cargo test -p <crate>` for fast feedback; `cargo test --workspace` before pushing.
3. **Pass the full gate before committing.** All three must be clean:
   ```bash
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
   CI runs the same three on `[ubuntu-latest, macos-latest, windows-latest]`. The toolchain is pinned via `rust-toolchain.toml`, so local dev and CI agree on `rustfmt` / `clippy` versions.
4. **Code and docs move together.** A behavior change that affects [`DIVERGENCES.md`](specs/DIVERGENCES.md) or [`DESIGN.md`](specs/DESIGN.md) updates them in the same commit.
5. **Keep commits scoped.** One step per commit. Don't ship `fix: drop unused …` follow-up commits — those are evidence the pre-commit gate was skipped.

## Architecture in one breath

`rdom-core` is the renderer-agnostic substrate. `rdom-style` is the CSS data model. `rdom-css` parses CSS strings. `rdom-parser` parses HTML templates. `rdom-tui` is the terminal backend (cascade + layout + paint + runtime + native HTML built-ins). The substrate ships native HTML elements and zero opinionated components; higher-level component libraries live in downstream consumer crates.

See [`CLAUDE.md`](CLAUDE.md) §"Substrate First, Backend Second" for the durable rules.

## For AI coding agents

This repo follows the [agents.md](https://agents.md) convention. `AGENTS.md → CLAUDE.md` is a symlink, so Claude Code / Codex / Cursor / Aider / any other agents.md-aware tool reads the same content.

Operational checklists live in [`.claude/skills/`](.claude/skills/) and are invokable as slash commands:

| Skill | What it does |
|---|---|
| [`/commit`](.claude/skills/commit.md) | Pre-commit hygiene gate. Walks the four-command gate, composes the commit message, enforces the "no `fix:` follow-up" rule. |
| [`/push`](.claude/skills/push.md) | Pre-push + post-push checklist. Verifies clean entry point so `/clear` is safe after every push. |
| [`/publish`](.claude/skills/publish.md) | Crates.io release checklist. Bump-decision audit, metadata audit, README sanity, gates, dry-run, the publish loop (with index propagation waits), post-publish wrap-up. |

## Reporting issues

Open an issue at <https://github.com/miskun/rdom/issues>. For bugs, include a minimal reproduction — ideally a failing test in the affected crate. The "Real Fixes Only" principle in `CLAUDE.md` applies to both bug reports and fixes: identify the root cause, don't paper over symptoms.

## License

By contributing, you agree that your contributions will be licensed under the MIT License — see [`LICENSE`](LICENSE).
