---
name: publish
description: Pre-publish checklist + publish sequence + post-publish wrap-up for the rdom workspace. Use whenever shipping any rdom crate to crates.io.
---

# /publish — crates.io release checklist

Use this whenever a rdom crate is going to crates.io. The five crates (`rdom-core`, `rdom-style`, `rdom-css`, `rdom-parser`, `rdom-tui`) publish independently but share a workspace version (`workspace.package.version` in the root `Cargo.toml`); changes in one often imply bumps in consumers.

**Every `cargo publish` is irreversible.** A crate/version pair, once on crates.io, can never be reused — only superseded by a higher version. The point of this skill is to catch every problem *before* that point.

## 1. Decide what's being published

Run `git log --oneline $(git describe --tags --abbrev=0)..HEAD` (or against the first commit if there is no tag yet) and ask:

- **Which crates changed?** Anything under `crates/<name>/` (source, examples, README, Cargo.toml) means *that crate* needs a bump.
- **Which crates depend on a changed crate?** Those need to bump too if the change is API-visible. The dep DAG is `rdom-core → {rdom-style, rdom-parser} → rdom-css → rdom-tui`. (`rdom-parser` only depends on `rdom-core`; `rdom-css` depends on `rdom-core` + `rdom-style`; `rdom-tui` depends on `rdom-core` + `rdom-style` + `rdom-css`.)
- **Is this an initial publish or a re-publish?** For the initial `0.1.0`, all five crates publish together at the workspace version. For later releases, only changed crates + their dependents need to bump.

Write down the bump plan as a one-line decision in the prep commit's message so future sessions can read it.

### Version bump rules

- **0.x.y** (pre-1.0, where we are): any breaking change bumps `x`; additive changes bump `y`. Patch-level fixes are also `y`.
- **Same version, two crates with different source?** Not allowed. Bump anything that changed, even a typo fix in a docstring — crates.io enforces version-source pairing, so a re-publish with the same version number fails outright.
- **Bumping `rdom-core`?** `rdom-style`, `rdom-css`, `rdom-parser`, `rdom-tui` all need to bump their **dep on rdom-core** in their `[dependencies]` table, AND bump their own version because their published `Cargo.toml` changed.

## 2. Per-crate metadata audit

For each crate that will publish, open its `Cargo.toml` and verify:

```toml
[package]
name = "rdom-..."
version.workspace = true              # or pin if it's diverging
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "..."                   # required; one sentence
readme = "README.md"                  # required for crates.io card
keywords = ["...", "...", "..."]      # ≤5, used by crates.io search
categories = ["..."]                  # valid crates.io categories only

[dependencies]
rdom-... = { path = "../rdom-...", version = "X.Y.Z" }   # BOTH path and version
```

**Mandatory checks:**

- [ ] `name` is set, matches directory.
- [ ] `version` resolves to the intended publish version (`cargo metadata --no-deps | jq '.packages[] | {name, version}'` shows the full table).
- [ ] `description` is present and accurate — appears on crates.io card and search.
- [ ] `license` is set (workspace inherits `MIT`).
- [ ] `repository` URL works (`curl -sI <url>` returns 200/301).
- [ ] `readme = "README.md"` is **declared** AND the file exists on disk.
- [ ] `keywords` and `categories` are set. Categories must be valid: <https://crates.io/category_slugs>.
- [ ] `[dependencies]` — every inter-crate dep has BOTH `path` (for local builds) AND `version` (for the published manifest). Path-only deps will fail `cargo publish` with `all dependencies must have a version requirement specified`.
- [ ] `[dev-dependencies]` and `[build-dependencies]` — same rule; add `version` everywhere. Even though dev-deps don't ship, cargo treats their absence as a publish-time error in some configurations.
- [ ] No `publish = false` on a crate that should publish.

If any crate is missing a field, fix it in the prep commit before going further.

## 3. README sanity

For each crate's `README.md`:

- [ ] Mentions only **shipped** features. No "coming soon," no "v0.2 will…" — those belong in `specs/DESIGN.md#roadmap`, not in a published crate's card.
- [ ] Code examples compile against the version being published. If you renamed a public API since the last release, every README block that uses it needs to update.
- [ ] Test counts and "X tests pass" claims match what `cargo test -p <crate>` actually reports.
- [ ] Cross-crate links (e.g. `[`rdom-tui`](../rdom-tui/)`) render correctly on crates.io. Relative paths to *other crates* don't resolve on crates.io's renderer — convert them to crates.io URLs (`https://crates.io/crates/rdom-tui`) for the published README, OR accept the broken link as a known trade-off. (We currently use relative paths; consumers click through GitHub.)
- [ ] Spec links (`../../specs/DESIGN.md`) likewise don't resolve on crates.io. The README should still be self-contained enough to read standalone.

## 4. Gates

Run the full hygiene gate (`/commit` does this):

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All three must pass clean. **If any fails, fix and re-run the entire gate.** A failed gate before publish is a free do-over; a failed gate after publish is a permanent stain on the version history.

## 5. Dry-run

For the leaf crate (no inter-crate deps; for rdom that is **`rdom-core`**), run a full dry-run:

```bash
cargo publish -p rdom-core --dry-run
```

It should print `Packaged N files, X KiB compressed` and `aborting upload due to dry run`. No errors.

**Downstream crates can't fully dry-run before the leaf is on crates.io** — `cargo publish --dry-run` tries to *resolve* their inter-crate deps against crates.io, and an unpublished dep is a hard fail. This is inherent to first-time workspace publish.

Use `cargo package --list --no-verify` as the next-best signal that the manifest is otherwise valid:

```bash
for c in rdom-style rdom-css rdom-parser rdom-tui; do
    cargo package -p "$c" --no-verify --list --allow-dirty | tail -3
    echo "---"
done
```

If those succeed (any output, no error), the manifests are syntactically correct and the file inclusion is sane. Real verification happens once the leaf is published.

## 6. Commit the prep

Run `/commit`. One commit, scoped, e.g.:

```
prep 0.1.0: per-crate READMEs + path-dep version pins

- rdom-style + rdom-css: add launch-grade README.md
- All inter-crate deps: pin to version = "0.1.0" alongside path
```

Then push (`/push`) so the published commit is also the public commit. **Don't `--allow-dirty` the real publish.** Dry-run is the only place that's OK.

## 7. The publish loop

After `cargo login` (one-time per machine, user action with a crates.io API token from <https://crates.io/me>):

Publish in dep order, **waiting between each** for crates.io's sparse index to propagate (~30–60s; longer for the first publish of a brand-new crate name). Skipping the wait means the next `cargo publish` errors with `no matching package named '<dep>' found`.

```bash
cargo publish -p rdom-core      # leaf
sleep 60
cargo publish -p rdom-style     # depends on rdom-core
sleep 60
cargo publish -p rdom-css       # depends on rdom-core + rdom-style
sleep 60
cargo publish -p rdom-parser    # depends on rdom-core
sleep 60
cargo publish -p rdom-tui       # depends on everything
```

After each `cargo publish`, verify the crate appears at `https://crates.io/crates/<name>` before moving on. The sparse index updates within seconds; the web page can take a minute or two.

If a publish errors *after* the leaf already shipped, you cannot roll back. Options:

- **Fix and bump.** Add the fix to a new commit, bump *only the affected crate* to `0.1.1`, re-run from step 1 with a shorter audit (only the changed crates need re-check).
- **Yank.** `cargo yank -p <crate> --vers <ver>` prevents new downstream resolutions to that version but doesn't delete it. Use sparingly; yanks are visible on the crate's page.

## 8. Post-publish

After all five crates are live on crates.io:

1. **Tag the commit:** `git tag v0.1.0 && git push --tags`. Future `git describe --tags` resolves cleanly.
2. **Update `CHANGELOG.md`** if anything material changed between prep and release (rare on first publish).
3. Working tree clean, branch synced, ready for `/clear`.

## Anti-patterns

- **Same version, different source.** Bumping a crate's source without bumping its version means the next `cargo publish` fails with `crate version <X> is already uploaded`. Always bump if you changed anything.
- **Forgetting a transitive consumer.** Bumping `rdom-core` without bumping its dep version in `rdom-tui`'s `Cargo.toml` means rdom-tui at `0.1.0` will keep resolving to `rdom-core = "0.1.0"` from crates.io even after `0.2.0` ships. Audit `[dependencies]` blocks every time the upstream moves.
- **Skipping the index-propagation wait.** Index propagation isn't atomic. `cargo publish -p rdom-style` immediately after `rdom-core` ≈ 50% chance of failing. Sleep.
- **`--allow-dirty` on the real publish.** Means the uploaded tarball doesn't match any committed state. Always commit first, then publish from clean.
- **Marketing-voice README.** Crates.io is read by serious engineers evaluating a dependency. "Blazing fast" / "the modern way to…" / "future-proof" all hurt credibility. Match the project's voice: shipped features, honest tradeoffs, pointers to specs.

## Why this is a skill, not a CLAUDE.md essay

The non-negotiable rules (substrate-first architecture, browser-DOM fidelity, real fixes only) live in `CLAUDE.md` so they load every session. This skill is the **operational checklist** — a recipe you walk through interactively at the moment of release, not a contract that needs to load on every turn. Saves `CLAUDE.md` from accumulating publish-day operational drift.
