---
name: push
description: Pre-push and post-push checklist for the rdom project. Ensures every push lands at a clean entry point safe for /clear.
---

# /push — pre-push + post-push checklist

Use this whenever you're about to push to `origin/main`. Enforces the **clean-entry-point rule** in `CLAUDE.md` §"Commit Discipline" — every push must leave the repo in a state where `/clear` is safe.

## Pre-push (verify before pushing)

1. **Working tree clean.** `git status` reports nothing uncommitted, no untracked files. If the working tree is dirty, decide whether the changes belong in a commit or in `.gitignore` — don't push leaving them behind.
2. **Commit gate passed for every unpushed commit.** Run `git log origin/main..HEAD --oneline` to see what's about to be pushed. For each commit, the `/commit` skill's gate (fmt + clippy + tests) should have run. If you skipped the gate on any of them, run it now and amend / fixup before pushing.
3. **No half-built artifacts.** Look for stub files (`mod foo;` declared but file empty), placeholder TODOs that should be commits, drafts in unexpected places.

If any isn't true: **don't push**. Fix or commit, re-run the gate, then push.

## The push

```bash
git push
```

To origin, no force. **Never `--force` to `main` without explicit user authorization.** If the user has explicitly asked for a force-push (rare), the commit message of the most recent commit (or the current chat context) should make that authorization clear.

If pushing a wip-* branch, no force-push restriction — but call out in the user-facing summary that this is a wip branch, not main.

## Post-push verification

After `git push` completes:

1. `git status` → "On branch main / Your branch is up to date with 'origin/main' / nothing to commit, working tree clean".
2. `git log origin/main..HEAD --oneline` → empty.
3. No outstanding artifacts that would confuse a `/clear` + fresh session.

If all pass: report **safe to /clear** to the user, with the new origin SHA range (e.g., `823c947..d76a737 main -> main`).

If anything failed: explain what's not clean and let the user decide whether to push wip or hold.

## Why this is a skill, not a CLAUDE.md essay

The contract (clean entry point after every push) lives in `CLAUDE.md` so it loads every session. The operational checklist lives here so an interactive `/push` walks through it step by step. Saves CLAUDE.md from accumulating operational drift.
