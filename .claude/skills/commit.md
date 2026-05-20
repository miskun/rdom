---
name: commit
description: Pre-commit hygiene gate for the rdom project. Run before any push-bound commit; codifies the fmt+clippy+test gate plus commit-composition rules.
---

# /commit — pre-commit hygiene gate

Use this whenever you're about to make a commit destined to be pushed to `origin/main`. The gate is **mandatory** per `CLAUDE.md` §"Commit Discipline".

## The gate (all three must pass)

```bash
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run all three. **If any fails, fix it, then re-run the entire gate.** Don't move past a failure with "I'll fix it in the next commit."

### Doc-only commits

Files under `specs/`, `CLAUDE.md`, `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, and `.claude/` skip `cargo test --workspace` because no code changed. Steps 1 and 2 still run — doc-only commits sometimes touch tests-as-doctests or update example code blocks that compile.

If the commit mixes doc and code, run the full gate.

## Anti-pattern: post-hoc `fix:` commits

If clippy catches an unused import or a style error after a commit has landed, **don't ship a `fix: drop unused…` follow-up commit**. That's evidence the gate was skipped. Instead:

- If the bad commit is **unpushed**: `git commit --amend` after fixing.
- If it's **pushed**: an immediate fixup commit is acceptable, but call it out in the next end-of-turn summary so the user notices the gate slipped.

## Composition

After the gate passes:

1. **Stage scoped files.** Prefer named paths over `git add -A` / `git add .` when the worktree may have unrelated dirt. If the worktree IS clean and all changes are part of the commit, `git add -A` is fine.
2. **Compose the message.**
   - Subject: under 70 chars, imperative voice (`Add foo`, `Rewrite bar`, not `Added foo`).
   - Body: WHY the change exists. Reference `TECH_DEBT.md` IDs (`D-M3-1`) when applicable.
   - Use `git log --oneline -n 5` to match the project's recent style.
3. **Don't mix unrelated changes.** If you find yourself writing "Also: …" in the commit body, the changes probably want separate commits.
4. **Use a HEREDOC** for multi-line messages to keep formatting clean:
   ```bash
   git commit -m "$(cat <<'EOF'
   Subject line under 70 chars

   Body explaining the why, with line wrapping at ~72 chars
   for readability in `git log`.
   EOF
   )"
   ```

## Output

After committing, run `git log --oneline -n 3` and report the commit hash + subject to the user. Don't push from this skill — that's `/push`.

## Why this is a skill, not a CLAUDE.md essay

The rule (mandatory gate, no follow-up fix-commits) lives in `CLAUDE.md` so it loads every session. The operational checklist lives here so a session that wants to walk through it interactively can `/commit` and follow the steps. CLAUDE.md states the contract; this skill executes it.
