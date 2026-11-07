# CAPTURE — an agent-friendly renderer, inspector and design loop

**Status:** IN PROGRESS — phases 1–4 done (§Phase log). v2 (2026-11-07). v1 (`2da9c01c`) was reviewed by a chief architect and a chief TUI designer;
v2 incorporates every finding (§Review record at the end). Three open questions are decided (§Decisions).

## Decisions

| Date | Question | Decision | Why |
|---|---|---|---|
| 2026-11-07 | PNG rendering: `resvg`/`tiny-skia` + font, or our own | **Our own minimal rasterizer and PNG encoder** (§Rasterizer) | A cell grid needs no text layout; procedural box drawing is crisper than fonts; no dependencies; byte-identical on every OS |
| 2026-11-07 | xterm.js emulator view | **Optional**: cargo feature + browser at run time, not in CI by default (§Views) | Real-terminal reference without making a browser a build or CI dependency |
| 2026-11-07 | Animation events caused by inspector changes | **Suppress now, designed so `deliver` can be added later** (§Events caused by the inspector) | Keeps the inspector invisible to app code; cause tagging and a single policy point make delivery an additive change |

## Why

rdom paints to a terminal, and nothing else can see that paint. Snapshot tests can, but only through test-only
helpers, and only as glyph text plus a background-key layer. An agent or a person improving how an app looks
and behaves is missing three things:

- **Seeing** any page at any moment, outside a test and outside a live terminal: as structured data, as an image,
  in a real terminal emulator's rendering.
- **Understanding** it: why this cell is this glyph in this color, through every step from the CSS source to
  the emitted bytes, and which line to edit.
- **Changing** it cheaply: try an edit, a token value, a viewport, a terminal profile, a forced state, other
  content, and see the result in milliseconds, without recompiling.

This initiative adds no CSS features. It's an instrument: every feature that already exists or is added later
(gradients, border animations, glyph effects) can be seen, explained and measured with it, with no extra work per
feature.

## Thesis: better than browser integration for agents

Browser tooling gives agents screenshots of a nondeterministic pixel canvas and a debugging protocol too large
to use well, so agents guess. rdom's advantages are structural: the screen is a small discrete grid (120×40 is
4,800 cells), the clock is ours, and we own every step from the CSS source to the bytes. The design uses them:

1. **Complete causality.** Any cell can be explained back to source lines, including blending, border joins,
   animations and color reduction (§Paint trace, §Derivation chain).
2. **Exact reproducibility.** The same inputs give byte-identical captures, so every diff is signal (§Deterministic
   environment).
3. **Counterfactuals.** "What if" questions, asked in batches, answered with diffs and audit changes, with no manual
   capture (§What-if).
4. **Answers sized to the question.** Every answer has a detail level and a token budget; truncation is always
   explicit (§Session protocol).
5. **A persistent session.** The app is built and driven once; queries are cheap (§Session protocol).
6. **Record by hand, replay exactly.** A human reproduces a state in a real terminal; the agent replays it headless
   (§Record and replay).
7. **Terminal reality as a variable.** Palettes, capabilities, cell shape and a real emulator's rendering are a
   matrix axis, not a fixed assumption (§Terminal profiles).

## The design loop

The primary user is an agent asked to review an app's UI/UX and to make it better. A human designer runs the
same loop.

| Step | The agent asks | The session answers with |
|---|---|---|
| **Orient** | What's on this screen? | A landmarks outline: roles, names, on-screen rectangles, focus, a few hundred tokens |
| **Observe** | What does it look like? | A PNG at true cell proportions, and the exact text grid |
| **Understand** | Why is this cell like this? Where do I edit? | `explain`: paint trace + derivation chain + source lines |
| **Audit** | What's objectively wrong? | Findings on look, UX, tokens and motion, each with inputs and thresholds |
| **Stress** | What breaks with real content? | Content rewrites (long, empty, CJK, emoji, RTL, big numbers) and what moved, truncated or overflowed |
| **Hypothesize** | Which of these N edits is best? | `whatif`: per candidate, a diff summary, audit changes and a thumbnail |
| **Cover** | Does it hold up across sizes, schemes, terminals, states? | A variant matrix on one contact sheet |
| **Motion** | Is it smooth, does it flicker, does it hold up over SSH? | A film: frames at fixed virtual-clock steps, timelines, motion findings |
| **Target** | Does it match the design? | A diff against a text mockup with a color legend |
| **Land** | What's the real change? | Edits written back as a unified diff against the CSS sources |
| **Hand off** | Can a human check this? | A static HTML report: before/after, findings, inspector |

## Principles

- **One contract.** `Capture` (JSON, versioned schema) is the single source of truth. Every view, audit, diff and query
  reads it; every producer (session, live attach, replay) emits it.
- **Causal truth over convenience.** An explanation is the actual sequence of operations that produced the cell,
  never a guess that looks plausible. Where rdom can't explain a step, it says so.
- **Deterministic by contract.** Determinism is enforced by tests, not assumed.
- **Query first, never dump first.** The default answer is the smallest one that answers the question.
- **Invisible to the app.** Inspection, forced states and edits never show up as events, mutation records or
  `matches()` results in app code.
- **Edits land in source.** A runtime edit is a preview; its result is a patch against the file it came from.
- **Zero cost when off.** Recording costs one branch per paint write when no recorder is attached; `frame_cost_tests`
  stay green.
- **Explainable findings.** Every finding names its check, inputs, threshold and what it didn't check.

## Architecture

```
app (downstream)            implements CaptureApp: build(env, fixture) -> App, fixture list
   │
rdom-capture (new, published)
   ├─ env         Environment: size, depth, caps, scheme, media prefs — set, never probed
   ├─ session     persistent session: JSON-RPC over stdio; CLI and MCP are thin clients
   ├─ schema      Capture types + JSON schema (versioned)
   ├─ query       outline, explain, inspect, reverse queries, budgets
   ├─ edit        declaration/token edits, write-back diffs, CSS dev loader
   ├─ variants    matrix, whatif, stress, terminal profiles
   ├─ audits      look, UX, tokens, motion
   ├─ motion      film, timelines, cast
   ├─ replay      record/replay of input + injections
   ├─ views       text, outline, png, html inspector, xterm.js reference render
   └─ attach      live attach client + server glue
   │
rdom-tui  (one public module: rdom_tui::capture)
   ├─ PaintRecorder      optional sink for paint operations
   ├─ derivation tracer  re-runs the cascade for one node with a tracer
   ├─ boxes              per-node geometry
   ├─ forced states      cascade-only pseudo-class overrides
   ├─ declaration edits  in-place, reversible
   └─ source locations   rules carry file:line (CSS) or #[track_caller] (Rust)
   │
rdom-core  (unchanged)
```

`rdom-tui` exposes the data; `rdom-capture` turns it into a product. The deterministic environment needed no
`rdom-tui` change: the `App`'s existing builders (`with_color_depth`, `with_sgr_capabilities`,
`with_color_scheme`, `with_media_preferences`, `with_caret_blink`, `with_clipboard`, `with_url_opener`) and its
virtual clock (`App::advance`) cover it, so `Environment` lives in `rdom-capture` (phase 1). `rdom-tui` doesn't depend on
`rdom-capture`. All new `rdom-tui` API sits behind one entry point, `App::capture(&CaptureRequest) -> CaptureData`,
plus the handful of hooks above. Result types are open (`#[non_exhaustive]`) and listed in `DESIGN.md` when they land.

## Deterministic environment

A capture is a function of (app build, fixtures, environment, input log, edits, virtual time). Same inputs, same
bytes.

The `Environment` fixes:

- **Time:** the scheduler's virtual clock is the only clock a frame reads. The session drives it from the first
  frame (`App::advance(0)` at open), so the wall clock never enters. `rdom-showcase`'s `wall_clock` test lists every
  production read of the wall clock with its reason — the scheduler's epoch, `App::run`'s live loop, the live
  input reader and the startup color-scheme query, the debug trace — and fails on a new one. (The reads at
  `runtime/animation/{mod,custom}.rs` the review named are in `#[cfg(test)]` helpers, not production.)
- **Terminal:** size, color depth, SGR capabilities, color scheme and keyboard protocol are set, never probed.
- **Ordering:** no output depends on hash-map iteration order.
- **Process:** environment variables the runtime reads are part of the `Environment`, not read live.
- **System:** the clipboard and the URL opener are in-memory; a session never touches the user's system.
- **Enforced, not trusted:** the session applies the environment after `CaptureApp::build` returns, overriding
  what the app set itself, and refuses an app whose terminal is not the viewport's size.

Contract tests (`rdom-showcase` `capture_session`): every acid page and every demo captured twice in one process,
and a script over every acid page and a demo with input run by two processes of `rdom-showcase-capture`; the
bytes must match. An acid page's capture equals the acid harness's paint. A capture records its `Environment` in
`meta`.

## Getting an app into a session

Real apps aren't just a DOM: they have Rust state, channels, background work, data from disk or the network, and
tick callbacks. The app opts in with one trait:

```rust
pub trait CaptureApp {
    /// Build the app for a session, in `fixture`'s state. No terminal is touched;
    /// `env.app(dom, sheet)` builds a headless App with the environment applied.
    fn build(&self, env: &Environment, fixture: Option<&str>)
        -> Result<App<TestBackend>, Box<dyn Error>>;
    /// After the first frame: what a page's load scripts do. Default: nothing.
    fn after_first_frame(&self, app: &mut App<TestBackend>, fixture: Option<&str>) {}
    /// Named states the agent can choose ("empty", "1k-rows", "error-state"), each with
    /// an optional viewport hint. When not empty, an unknown fixture is refused.
    fn fixtures(&self) -> Vec<FixtureInfo> { Vec::new() }
}
```

and ships one binary target: `fn main() { rdom_capture::serve(MyApp) }`. Everything after that is protocol, with no
more app-specific code.

External work (threads, async tasks, network) reaches the DOM through `AppHandle::inject`. In a session, injections
are queued and delivered at defined points in virtual time, so a replay delivers them identically.

### Record and replay

An app running in a real terminal with recording on (`App::with_recorder(path)` or `RDOM_RECORD=<path>`) writes
the input log: terminal input events, resizes, injections (by name and payload, when the app makes them
serializable), and their times. `session.replay(log)` drives the headless app through the same sequence. A human
reproduces a state by hand once; the agent replays, scrubs and inspects it as often as it likes.

Replay divergence is detected, not hidden: the log carries a grid hash per frame, and replay reports the first frame
that differs.

## The `Capture` contract (schema `rdom-capture/1`)

A capture is layered. The grid and outline are always present; everything else is requested by need. Series of
captures store deltas.

```jsonc
{
  "schema": "rdom-capture/1",
  "meta": {
    "rdom_tui": "0.6.5", "time_ms": 300, "viewport": [120, 40],
    "env": {"color_depth": "truecolor", "scheme": "dark", "caps": "full", "reduced_motion": false},
    "profile": "xterm-dark",
    "fixtures": "1k-rows", "edits": ["e1"], "forced_states": [{"node": "settings", "state": "hover"}],
    "replay": {"log": "rec/bug-42.jsonl", "frame": 118},
    "grid_hash": "b3:9f2c…"
  },
  "grid": { "rows": [[{"x": 0, "text": "┌ Settings ", "fg": "#c0c0c0", "bg": "#1e1e1e",
                       "mods": ["bold"]}]] },
  "owners": { "rows": [[{"x": 0, "w": 1, "part": "border-join", "joined": ["12"]},
                        {"x": 1, "w": 1, "node": "12", "part": "text"}]] },
  "outline": [
    {"node": 12, "key": "id:settings", "role": "region", "name": "Settings", "rect": [0, 0, 60, 20],
     "focus": "within", "children": [ … ]}
  ],
  "nodes":  { "12": { "tag": "section", "key": "id:settings", "path": "main > section#settings.panel", … } },
  "boxes":  { "12": { "border": [0,0,60,20], "padding": [1,1,58,18], "content": [2,1,56,18],
                      "scroll": {"offset": [0,4], "size": [56,40]}, "clip": [0,0,60,20], "layer": 0 } },
  "styles": { "12": { "color": "#c0c0c0", "padding": "0 1", … } },
  "rules":  { "41": { "selector": ".panel", "sheet": "app", "origin": "author", "layer": null,
                      "specificity": [0,1,0], "source": {"file": "src/style.css", "line": 41, "col": 1} } },
  "tokens": { "--fg": {"value": "#c0c0c0", "defined": [{"rule": 3}], "used_by": 14} }
}
```

- `owners` (phase 2) is a layer of its own, not a field of the grid's runs: its runs tile each row by owner — the
  last operation on the frame that wrote the cell: its node and part (`background`, `border-join` with the nodes
  whose borders met, `text`, `pseudo:before|after|marker|first-letter`, `ua-chrome`, `caret`, `selection`,
  `highlight`, `scrollbar`, `outline`, `group`, `backdrop`, `tree-guide`, `canvas`, …). It's a convenience derived
  from the paint trace, not the explanation. Kept apart, the grid's runs, and so its hash, don't depend on whether
  owners were asked for.
- Every layer that names elements brings the `nodes` table (tag, `id`, classes, parent, path); `boxes` places
  every rendered element (border, padding, content, scrollport and scroll state, fragments).
- Colors are resolved sRGB; where the author wrote something else, `spec` carries it (`"ansi:4"`, `"oklch(…)"`,
  `"var(--fg)"`).
- `source` is the CSS parser's file, line and column, or `#[track_caller]` for rules built in Rust; `null` when
  unknown, never invented.

### Node identity

Arena ids are only valid within one capture. Across captures (diffs, films, replays), nodes match by, in order:
`id` attribute → `data-key` attribute → structural fingerprint (tag, classes, ancestor path, sibling index among
same-keyed siblings). Every node's `key` says which method produced it, and every diff reports how each pair was
matched.

### Paint trace

The paint trace of a cell is the ordered list of paint operations that touched it:

```jsonc
{"cell": [12, 4], "ops": [
  {"layer": 0, "node": 3,  "part": "background", "op": "fill",  "out": {"bg": "#1e1e1e"}},
  {"layer": 0, "node": 12, "part": "border",     "op": "glyph", "out": {"sym": "─", "fg": "#c0c0c0"}},
  {"layer": 0, "node": 12, "part": "border",     "op": "join",  "in": {"sym": "─"}, "out": {"sym": "┬"},
   "with": [{"node": 19, "part": "border"}]},
  {"layer": 2, "node": 40, "part": "background", "op": "blend", "alpha": 0.6,
   "in": {"bg": "#1e1e1e"}, "out": {"bg": "#2c2a3a"}},
  {"op": "quantize", "depth": "256", "in": {"bg": "#2c2a3a"}, "out": {"bg": "ansi256:236"}}
]}
```

As built (phase 2): the recorder rides on the `Buffer` the paint pass already threads everywhere (an explicit
field, no thread-locals). The buffer's write funnels — `cell_mut`, the coverage `mark`s every border write
already makes, compositing and color mapping — note the cell; the paint pass names whose paint is running with
`Buffer::painting(node, part, …)` around each element part; each time that changes the recorder compares the
written cells with what it last saw and records one operation per change (the cell after it, and the border
state for a contribution). The paint logic itself is unchanged: tracing observes it. Layers (`opacity`, filters,
translucent colors) get a recorder one level deeper whose operations are adopted when the layer composites back,
followed by the `blend`. The joiner's operation names the nodes whose contributions won each direction. Quantization
happens in the backend, not the paint, so it is not an operation: the grid's `*_shown` colors carry it. Recording
can be limited to a rectangle. Cost with no recorder: one branch per write — `paint_only` (rdom-tui `runtime`
bench) before the recorder (`ba5ccda0`) and after it: 87.3 → 87.1 µs at 80×24, 425 → 421 µs at 200×60, no
change detected (p > 0.3; both runs at load ≈ 5 on 12 cores, so the absolute numbers are not quiet-machine
costs, the comparison is). With a recorder: 1.3–3× a plain paint, ≤ 1.7 ms for any 120×50 acid page in release
(measured busy, load 5–8 — indicative only). `PaintTrace` indexes its operations by cell; `painter_at` looks
through layers to the paint a cell shows (a text in an `opacity` group is its element's, `via` the group).

### Derivation chain

For one node and property, the steps from declarations to used value:

```
color @ section#settings.panel
  cascaded   var(--fg)       .panel                 src/style.css:41   (beat: section  ua:12)
  var        #c0c0c0         --fg on :root          src/style.css:3
  animated   #a8a8a8         transition color 200ms, t=120ms, ease-out (from #808080)
  used       #a8a8a8
  painted    see paint trace (blend, quantize)
```

Steps: cascaded (winner and the declarations it beat) → `var()` substitution → inherited (from which ancestor) →
animated (transition or keyframes, progress, easing) → used → painted. Derivation is computed on request by
re-running the cascade for that node with a tracer, so nothing is stored per frame. A test checks that the
traced cascade's result equals the real cascade's value for every property on every acid node.

## Session protocol

A persistent session: newline-delimited JSON-RPC over stdio. The CLI (`rdom-capture <verb>`) and an MCP adapter are
thin clients. The Rust parameter and result types are the single source of every method; `describe` returns the
method list with each one's parameters, so a client discovers the protocol from the session. (A generated JSON
Schema would need a schema dependency for little gain over `describe` and the typed Rust; revisit if a non-Rust
client needs one.)

Every answering method takes `detail: outline | summary | full` (default `summary`) and an optional `budget` in
approximate tokens. When an answer is cut, it says so (`"truncated": {"outline": 212}`, and `"over_budget": true`
when nothing more can be cut): ask again narrower, at a lower detail or with a larger budget for the rest.

| Method | Does |
|---|---|
| `open` | Build the app (`fixtures`, `env`, `profile`) |
| `input` | Keys, text, mouse, paste, resize; returns what changed (cells and nodes), not a full capture |
| `advance` | Move the virtual clock |
| `replay` | Drive from a recorded log, to a frame or the end |
| `capture` | Return or write a capture, choosing layers and views |
| `outline` | The landmarks outline |
| `explain` | A cell (`at`), a node (`select`), or a property: paint trace + derivation chain + sources |
| `query` | Reverse queries: who uses a color, a token or a rule; what paints in a rectangle; nodes matching a selector |
| `diff` | Two captures, or two times: changed cells, nodes, boxes, computed properties, with provenance and match method |
| `edit` | Change a declaration in place (by rule id), set a token, add a rule; reversible; returns the visual diff |
| `force` | Force pseudo-classes on nodes (`:hover`, `:focus-visible`, `:active`, `:checked`, `:disabled`, `:invalid`) |
| `patch` | Turn the active edits into unified diffs against their source files |
| `whatif` | N candidate edit sets, each scored (see §What-if) |
| `matrix` | A cross product of variants to a contact sheet |
| `stress` | Content rewrites and what broke |
| `audit` | Run checks, return findings |
| `film` | Frames at a fixed step over a window, timelines, motion findings |
| `mockup` | Compare a capture with a text mockup |
| `report` | Write the static HTML report for a human |
| `reset` | Drop edits and forced states |

## Edits

- **In place.** An edit changes a declaration of an existing rule (by rule id), sets a custom property, or adds a
  rule in a given sheet and layer. It follows the same cascade as the original, so it can do what the original
  could (`!important`, layers, inline styles). Each edit has an id and an inverse.
- **Tokens first.** Setting a token is the highest-leverage edit. `query` shows each token's users, and `whatif`
  previews a token change across the whole page.
- **Write-back.** `patch` turns edits into unified diffs against CSS source files. For rules built in Rust, it returns
  the source location and the changed declaration; it doesn't rewrite Rust code.
- **CSS dev loader.** `rdom_capture::css!("src/style.css")` is `include_str!` in release builds and a file read
  with a reload watch in debug builds and sessions, so a saved CSS file shows up without a rebuild.

### Invisible to the app

Forced states affect cascade matching only. App code never sees them: no input events, no focus/blur, no
mutation-observer records, and `matches(":hover")` reports the real state. Edits change stylesheets, not the DOM.
Each rule has a contract test.

### Events caused by the inspector

An inspector change can start animations, and animations fire events (`transitionrun`, `transitionstart`,
`transitionend`, `animationstart`, `animationiteration`, `animationend`, and their cancel events). Whether app code
sees those events is a policy, `inspector_events`, decided at one point and never scattered across code paths:

- **Cause tagging (built now).** Every style change carries a cause: `App` (the app's own DOM and stylesheet
  changes, input, timers) or `Inspector` (an edit, a forced state, a `stress` rewrite). Animations started by a
  style change inherit its cause, and so do their events. When one change has both causes, `App` wins: an event
  the app would have seen anyway is never hidden.
- **Policy at dispatch (one place).** The animation-event dispatcher consults the policy for `Inspector`-caused
  events. Animations themselves always run and paint either way; only event delivery to app code is governed.
- **v2 behavior.** Sessions and live attach both use `suppress`: suppressed events are counted and listed in
  `meta.suppressed_events` (type, node, cause), so the agent knows the app might have reacted. The protocol reserves
  `open { inspector_events: "suppress" | "deliver" }` and `attach { inspector_events: … }`; v2 accepts only `suppress`
  and refuses `deliver` with a clear error, so turning it on later is additive.
- **Later: `deliver`.** Hands `Inspector`-caused events to app code like any others, for apps whose look depends on
  reacting to them (a component that adds a class on `transitionend`). Needs its own contract tests (delivery order,
  no events after `reset`) before it's accepted.

Contract tests now: an `Inspector`-caused transition paints its frames and fires no app-visible event; an
`App`-caused transition on the same node during an inspector edit still fires; `meta.suppressed_events` lists
exactly the suppressed ones; `deliver` is refused.

## Views

| View | For | Notes |
|---|---|---|
| `text` | agents, tests | Glyph grid plus optional key layers (bg, fg, top node) with legends. The current snapshot format, made public |
| `outline` | agents | Landmarks with rectangles; the first thing to read |
| `png` | agents with image input, reports | rdom's own rasterizer (§Rasterizer) at the profile's cell size and proportions; optional overlays (boxes, a grid, tab order, hit targets, a highlighted node, a change heatmap) |
| `html` | humans | `report.html` / `inspect.html`: images, findings, filmstrip, and an Elements-style inspector (hover a cell → explain). Static, no server |
| `cast` | motion, wire checks | asciicast v2 of the real emitted bytes |
| `emulator` | ground truth | The emitted bytes rendered by xterm.js in a headless browser, screenshotted. **Optional**: a cargo feature plus a browser found at run time; when missing, the method fails with a clear error, never a silent fallback to `png`. Not in CI by default; the reference for "what a real terminal shows" |

SVG is not a first-class view. PNG and the HTML inspector cover its uses; it can be added later as another view of
the same data.

### Rasterizer

`png` is rendered by a small rasterizer in `rdom-capture`, with no image or font dependencies. A terminal frame is
a grid of cells with a known glyph, colors and attributes, which is a much simpler problem than general text
rendering:

- **Cells, not text layout.** Each cell is a `cell_w × cell_h` pixel rectangle from the profile (default 8×16;
  2× scale doubles it). Wide glyphs span two cells. There is no shaping, kerning or hinting.
- **One bitmap font for every glyph.** Glyphs come from unscii-16 (Viznut, public domain; `assets/unscii-16.hex`,
  provenance in `assets/NOTICE.md`): 8×16 cells, 2,997 narrow and 243 wide glyphs. It covers every range TUIs draw
  with: box drawing (light, heavy, double, rounded, dashed), block elements and shades, geometric shapes, Braille,
  the legacy-computing sextants and wedges, the Powerline separators, and Latin-1 / Latin Extended-A. Its box-drawing
  arms sit at one fixed position per weight on every edge (light rows 7–8 / columns 3–4, heavy 6–9 / 2–5, double
  6 + 8 / 1–2 + 4–5), so neighbouring cells join exactly; its Braille dots and blocks cover exactly what their code
  points name. Tests pin all three (`raster/tests.rs`), which is why v2's procedural drawing was not needed: the
  font already is the definition, and one source draws every glyph the same way. Not covered: CJK, most emoji, part
  of Cyrillic and the arrows block — those show as missing glyphs (below). Bold is a one-pixel smear (unscii has no
  bold face); italic shears the upper half one pixel right, within the cell; underline (single, double, curly,
  dotted, dashed, in its own color), strikethrough and overline are drawn by the rasterizer where the profile has
  them. Dim and reverse are already resolved into the cell's colors by rdom's paint.
- **Missing glyphs are explicit.** A glyph neither drawn procedurally nor in the font renders as a box with its code
  point in hex, and is listed in `meta.unrendered_glyphs`. Never a blank, never a guess.
- **Color.** Cells are filled with the resolved sRGB colors after the profile's palette and color reduction, so the
  image matches what audits measure. A color with alpha shows as its RGB, opaque — what rdom sends the terminal.
- **PNG encoding, ours too.** CRC-32, Adler-32 and the chunk writer are a few dozen lines. The image is indexed
  (palette PNG) when it has at most 256 colors, which a terminal frame almost always does. Version 1 uses a minimal
  deflate encoder: fixed Huffman codes with run-length matches, which compresses large solid areas and repeated rows
  well; each row takes the None / Sub / Up filter with the smallest sum. If sizes ever hurt, a better encoder
  replaces it behind the same function. A test decodes every kind of image with the independent `png` crate.
- **Deterministic.** Integer arithmetic only, no system fonts, no platform calls; the same capture gives the same
  bytes on every OS. Goldens are therefore stable, and the grid hash remains the main oracle.
- **Scope.** The rasterizer is a view, not a terminal emulator: it doesn't model a font's box-drawing gaps or line
  height. That's what the optional `emulator` view is for.

## Terminal profiles

A profile is: a 16-color palette (and optionally the 256 cube), whether bold also brightens, support for dim,
italic, strikethrough and underline styles, emoji and ambiguous-width policy, cell aspect ratio, and font
characteristics (box-drawing gaps, line height). Shipped profiles cover common dark and light defaults, a 16-color
console, a light theme with low contrast and a no-italic terminal. Apps add their own.

Profiles are a matrix axis, used by PNG, audits (contrast after the profile's palette and color reduction) and the
emulator view.

## Variants

- **`matrix`:** a cross product over viewport sizes, schemes, color depths, profiles, `prefers-reduced-motion`,
  forced states, fixtures and edit sets, rendered to one contact sheet, with per-cell findings.
- **`whatif`:** for each candidate edit set: a diff summary, the change in audit findings (new, fixed), a
  thumbnail, and the edit's patch. Candidates are evaluated from the same base state; the response ranks them by
  findings fixed minus findings introduced, and shows the counts, not just a score.
- **`stress`:** rewrites text content in chosen nodes (×2 and ×4 length, empty, a single long word, CJK, emoji,
  right-to-left, large numbers, many items) and reports, per rewrite, what truncated, overflowed, wrapped, shifted
  or overlapped. Rewrites are edits to text nodes in the session; they never reach the app's data.
- **`mockup`:** the agent (or a designer) writes the intended screen as text plus a color legend; the session reports
  cells that differ in glyph, color or alignment.

## Audits

A finding: `{check, severity, nodes, cells, inputs, threshold, message, not_checked}`. Checks run against a profile
and color depth. An app can declare its design system (palette tokens, spacing scale, type of focus indicator), and
checks use it where present; otherwise they infer from the page and say so.

**Look**
- **contrast:** text fg/bg (WCAG 2 ratio and APCA Lc) after color reduction and the profile's palette.
- **truncation and clipping:** text cut without an ellipsis; content outside every scrollport; zero-size boxes with
  text.
- **occlusion:** text painted and then fully covered.
- **alignment:** box edges that miss a shared column or row by one cell.
- **rhythm:** spacing values off the declared scale, or off the dominant one; aspect-aware (2 columns ≈ 1 row).
- **palette sprawl:** near-duplicate colors (ΔE OKLab), colors not from declared tokens.
- **color-depth loss:** distinct colors that collapse at 256 or 16 colors; borders that vanish.
- **profile breakage:** meaning carried only by italic, dim or a color the profile can't show.

**UX**
- **focus visibility:** `:focus-visible` changes at least one cell, with enough contrast.
- **keyboard reachability:** every interactive element reachable by Tab; tab order versus visual reading order.
- **hit targets:** clickable areas smaller than a threshold (default: 1 row × 3 columns).
- **feedback latency:** frames from an input to the first visible change, per interactive element.
- **landmarks:** regions with no accessible name; headings out of order.

**Motion** (on a film)
- **flicker:** a cell that changes and reverts within one step.
- **spinners:** period, width jitter, period against a comfortable range.
- **easing:** fitted against the declared timing function after snapping to whole cells; flags easing that can't
  be seen at that size.
- **cost:** cells changed and bytes emitted per frame; frames that exceed a bandwidth profile (local, SSH, slow link).
- **reduced motion:** with `prefers-reduced-motion`, nothing non-essential animates.

## Motion

`film` captures at a fixed virtual-clock step (default the app's animation frame) over a window, storing deltas.
It returns timelines for chosen cells or nodes (glyph, fg, bg over time, and the properties driving them), the motion
findings, a filmstrip PNG and a change heatmap, and optionally a `cast`. A bandwidth profile replays the emitted
bytes at a link's throughput and reports the frames a viewer on that link would actually see.

## Live attach

An app opts in (`App::with_inspector(path)` or `RDOM_INSPECT=<path>`) and serves the session protocol over a local
socket (Unix socket, 0600, path inside the app's working directory; named pipe on Windows) — never TCP. The `App` is
single-threaded, so the socket is polled from the event loop, between frames, with a time budget per request so a
client can't stall frames. Read-only by default; `edit` and `force` need a flag, appear in `meta`, and `reset` or
disconnect removes them. The inspector never writes to the terminal outside the normal frame. Events its changes
cause follow §Events caused by the inspector (suppressed in v2).

## Scale

- A capture's grid is run-length encoded; styles are deduplicated by hash; nodes, boxes, styles and traces are
  included only when asked.
- Films and replays store the first frame plus deltas.
- Budgets (≈ 6,000 nodes, 200×60): `capture` grid+outline under 50 ms; `explain` of one cell under 20 ms; full
  capture with every layer under 300 ms. Measured on a quiet machine; regressions are investigated.

## Testing strategy

- **Determinism:** byte-identical double captures (same process and separate processes) of every acid page.
- **Paint trace:** for every acid tile, replaying the trace's operations reproduces the painted cell exactly; the
  top operation's node is a node of that tile.
- **Derivation:** the traced cascade equals the real cascade for every property on every acid node.
- **Invisibility:** forced states and edits produce no events, focus changes, mutation records or `matches()`
  changes in app code.
- **Edits:** edit → patch → apply the patch to the source → reload → identical capture.
- **Replay:** a recorded acid interactive run replays to identical grid hashes.
- **Inspector events:** `Inspector`-caused events are suppressed and listed; `App`-caused ones are never hidden;
  `deliver` is refused.
- **Views:** text and outline goldens; few PNG goldens, byte-identical across OSes; the PNG encoder checked by
  decoding with an independent decoder (dev-dependency only); the grid hash is the main oracle.
- **Audits:** each check has a fixture that triggers it and one that doesn't.
- **Cost:** `frame_cost_tests` unchanged with no recorder; benchmarks for recording and capture budgets.

## Phases

Each phase lands green, test-first, with its exit test.

1. **Deterministic environment + `CaptureApp` + session skeleton.** Exit: double-capture determinism on the acid pages;
   `open` / `input` / `advance` / `capture` (grid, text view) over stdio.
2. **Paint recorder + traces + boxes.** Exit: trace replay reproduces every acid cell; no-recorder cost unchanged.
3. **Outline, explain, derivation, sources, query.** Exit: derivation equals the cascade on every acid node; rule
   sources point at real lines.
4. **Edits, forced states, cause tagging, patch, CSS dev loader.** Exit: invisibility tests and the
   inspector-event contract tests; the edit → patch → reload round trip.
5. **Rasterizer + PNG + terminal profiles + HTML report.** Exit: every procedural glyph range drawn and joined
   correctly (per-range fixtures); the PNG decodes with an independent decoder in a test; goldens for acid page 1
   under two profiles, identical on macOS, Linux and Windows CI.
6. **`whatif`, `matrix`, `stress`, `mockup`, `diff` with identity.** Exit: each on a showcase demo with known answers.
7. **Audits (look, UX, tokens).** Exit: positive and negative fixtures per check.
8. **Motion:** `film`, timelines, motion audits, `cast`, bandwidth profiles. Exit: acid motion tiles produce their
   spec'd timelines.
9. **Record/replay + live attach + emulator view.** Exit: a recorded showcase run replays identically; attach to a running
   showcase example.

Phases 1–3 make any page visible and explainable. Phase 4 removes the compile from the loop. Phase 6 turns design
into a search.

## Phase log

Each phase closes with a chief-architect review; its findings are fixed or recorded (here and in
`TECH_DEBT.md`) before the next phase starts.

### Phase 1 — deterministic environment, `CaptureApp`, session skeleton: DONE (2026-11-07)

Shipped: `rdom-capture` with `Environment`, `CaptureApp`, `Session`, the `rdom-capture/1` grid layer, the text
view and the JSON-RPC server; the showcase as a capture app (`rdom-showcase-capture`); the determinism tests
and the `wall_clock` lint.

Review (architect, fresh reviewer) — dispositions:

- **B1 `meta.env.viewport` stale after a resize** — fixed: a resize updates the session's environment; test.
- **B2 the wall clock leaks into frames drawn before the first `advance`** — fixed at the root in `rdom-tui`:
  `App::with_virtual_clock()`, applied by `Environment::enforce`; tests in both crates (a build that draws and
  starts a transition).
- **B3 ticks, the handle's redraw / quit, thread injections** — `App::advance` now ticks once per elapsed tick
  period and takes the handle's requests (`App::quit_requested`; `Change.quit_requested`); thread injections
  stay timing-dependent until phase 9 — accepted as `TECH_DEBT` `CAPTURE-INJECT-1`.
- Non-blocking, all fixed: the grid hash is a canonical cell encoding (pinned by a test), not the JSON form;
  the grid and hash are built once per step and reused by captures; `Change` and the grid agree on blank vs
  space; runs keep per-cell `glyphs` where re-splitting the text would shift cells; key layers key the colors
  shown after the color depth; `fixtures()` runs guarded; `advance` is bounded (`MAX_ADVANCE_MS`); a failed
  `open` leaves the open app open; `enforce` turns mouse re-arm off and `serve` moves fd 1 to stderr (Unix) so
  app prints can't corrupt the protocol; `meta.rdom_tui`; a `null` id is answered; typed text sends Enter / Tab
  / Shift as a terminal does; the showcase snapshots use the text view (format-only golden change, checked
  glyph- and color-identical).

### Phase 2 — paint recorder, traces, boxes: DONE (2026-11-07)

Shipped: `rdom_tui::capture` (`trace_paint`, `App::trace_paint`, `PaintOp` / `PaintPart` / `OpKind` /
`PaintTrace` with `top_at` / `painter_at` / `frame_mismatch`, `BorderState`, `box_model`); the capture layers
`owners` (with `via`), `boxes`, `nodes` (document order, escaped paths); the acid exit test; the `paint_only`
bench.

Review (architect, fresh reviewer) — dispositions:

- **B1 a clip path restored layer cells behind the recorder's back** — fixed: `clip_cells` goes through
  `Buffer::show_backdrop`, which notes the cell and takes back the backdrop's border owners; test.
- **B2 a recolor (tint, filter, layer blend) moved a border to the painter** — fixed: a direction changes hands only
  when its winning contribution does (`same_contribution`, color aside), and a layer's owners are adopted per
  direction by contribution after it composites; tests for a border in a group, in nested groups, under a
  translucent `::backdrop`, under a clip path.
- N1 the owners repaint is checked against the frame (`frame_mismatch`, `meta.warnings`); N2 owners look through
  layers (`painter_at`, `via`); N3 `box_model` gives an inline box no content box and its fragments' bounding
  box as its border box — which surfaced a real bug, fixed at the root: `bounding_rect()` / `client_rects()`
  returned a zero rect for a non-atomic inline box (now the fragments' bounding box, CSSOM View §6.1, one helper
  for both); N4 `nodes` / `boxes` keep document order (`NodeMap`); N5 `PaintTrace` indexes by cell and layers'
  operations are moved, not cloned — a recorder per translucent write still copies its region (accepted: only
  while tracing); N6 the contract carries `BorderState` (lines, hidden, quadrants), not the renderer's
  conflict-resolution records; N7 paths escape ids and classes, and the owners text view says when its keys run
  out; N8 measured before / after (above).
- Exit test strengthened: every acid page's App repaint matches its frame, every join names its elements, every
  operation names a live node; the replay check stays "last depth-0 operation = cell" — the layer chain is pinned
  by the unit tests (groups, nested groups, clip, translucency, regions).

### Phase 3 — outline, explain, derivation, sources, query: DONE (2026-11-07)

Shipped: rule source locations (`Rule::location`, `rdom_css::parse_at` / `from_css_at`, `#[track_caller]` Rust
rules); `rdom_tui::capture::explain_style` / `App::explain_style` (`StyleTrace`); the `outline`, `explain` and
`query` methods with detail levels and token budgets.

As built, against the plan:

- The derivation shares the cascade's code: the gather (`element::gather_element`), the starting style and
  `var()` preparation (`element::start_style`), the style parent (`details::inherited_style`'s rule) and the ladder
  iteration (`Declarations::sources_of`). The explanation lists the applied declarations per longhand and custom
  property with their value as applied (`var()` substituted in properties; custom properties as written, with their
  computed value from `explain`); `explain` resolves CSS-wide keywords (`inherit`, `initial`, `unset`, `revert` —
  `revert-layer` is reported, not resolved), inheritance up the cascade's own parent chain (a `<details>` slot is
  named, not traced), registered `inherits: false`. Not yet (`TECH_DEBT` `CAPTURE-EXPLAIN-1`): pseudo-elements' and
  the slot's declarations, `revert-layer`, computed / used values.
- Exit tests: `rdom-tui`'s `explain_tests` records the blocks the cascade's settled ladder run applies to each
  element and requires the explanation to report exactly those, in order, at the same importance, on a page with
  layers, `!important`, inline styles, `revert`, hints, `var()` from an ancestor and from a `<details>` slot, and
  `rtl` with a flow-relative `var()` declaration (it fails on the review's slot drift); on every acid page every
  element's explanation names the same rules (selector, specificity, location) as the cascade's recorded matches;
  every rule's location points at its selector list (`every_rules_line_holds_its_selector`).
- Review (architect) — B1 (exit test weaker than claimed) and B2 (the explanation's own setup had drifted at the
  `<details>` slot) fixed as above; non-blocking: keywords, registered inheritance, custom-property computed values,
  the inheritance walk (cascade parent, memoized), `is_shorthand` on inline-axis logical longhands, unknown
  property names refused, `query` text offsets after wide glyphs and its per-hit scans, `fit` termination and
  `over_budget`, the outline's accname order, `header` / `footer` scoping, `img alt`, button-input values — fixed;
  live-DOM matching and the showcase's unnamed CSS recorded (`CAPTURE-EXPLAIN-1`, `CAPTURE-DOGFOOD-1`). The
  promised `next` cursor is replaced by the answer's `truncated` record (ask again narrower or with a larger budget).
- The outline's roles follow HTML-AAM's implicit roles and the `role` attribute, its names accname's order for
  these elements; it is a navigation outline, not an accessibility tree.

### Phase 4 — edits, forced states, cause tagging, patch, CSS dev loader: DONE (2026-11-07)

Shipped: `Stylesheet::set_rule_declaration` / `remove_rule_declaration` (rdom-style), `Dom::set_forced_state`
honored by rendering passes only (rdom-core), `App::inspect` / `force_state` / `edit_stylesheet` /
`stylesheet_at` / `take_suppressed_events` with cause tagging in the animation registry (rdom-tui),
`rdom_css::css_file!` / `load_css_file`, and the session's `edit` / `force` / `reset` / `patch` / `reload`.

As built, against the plan:

- Edits target the rule an element's property wins by, a rule by its place, or a new rule in the App's last
  sheet; a `<style>` element's rules are not editable (its sheet comes from its text) — the edit says so. `reset`
  restores each edited sheet from a snapshot taken before its first edit (exact), not by inverse edits.
- Cause tagging lives in the animation registry, keyed by what an animation animates — the public event structs
  are unchanged (non-breaking). An inspector's frame (`App::inspect`) holds the inspector's change alone because a
  session settles every step; animations it starts are the inspector's until an app-caused start of the same key
  takes it back. Policy `suppress` only: `open { inspector_events: "deliver" }` is refused as reserved.
- `patch` collapses repeated edits per rule and property, applies them last-in-text first, appends new rules to
  the sheet's one named file, and lists every edit no file can take (UA rule, Rust-built rule, unnamed text) with
  why; `apply` writes the files. `reload` replaces each App sheet parsed from one named file and drops its edits.
- A forced state applies to the element's own selector matching; `::before:hover`-style pseudo-element states
  read the real pointer (not forced yet).
- Tests: forced `:hover` restyles with no mutation record and `matches()` false; the inspector's transition events
  withheld and listed while the app's own are delivered; a sheet edit restyles; edit → patch → reload gives the
  same grid hash; `reset` gives back the first frame; a UA edit is reported unpatchable; the CSS patcher and the
  unified diff have their own unit tests.

## Open questions

- **Injection serialization:** how much of `AppHandle::inject` traffic can be made replayable without burdening apps;
  opaque closures are recorded as "unreplayable" and replay stops there with a clear message.
- **Protocol stability:** which methods and fields are guaranteed in `rdom-capture/1` before the crate's first release.
- **Windows attach:** named-pipe parity, or file-based capture only at first.

## Non-goals

- New CSS features.
- Emulating a specific terminal pixel-for-pixel (the emulator view is a reference, not a promise).
- A visual editor. Changes land in the app's source as patches.

## Review record (v1 → v2)

v1 (`2da9c01c`) was reviewed on 2026-11-07. Every finding and where v2 addresses it:

### Chief architect

| # | Finding (v1) | Severity | v2 |
|---|---|---|---|
| A1 | "Cell → node → computed color → rule" misses blending, joins, inheritance, animation and color reduction; one owner per cell is false | Blocking | §Paint trace, §Derivation chain; `top` is derived |
| A2 | Determinism assumed, not enforced (`Instant::now()` leaks, probing, hash order, env) | Blocking | §Deterministic environment with contract tests |
| A3 | Real apps have state, threads, data; `App::new` grabs the terminal; scenario files can't reproduce that | Blocking | §Getting an app into a session (`CaptureApp`, fixtures, queued injections), §Record and replay |
| A4 | Override by appended sheet can't beat `!important`, inline styles, layers; doesn't land as a fix; `include_str!` still needs a rebuild | Blocking | §Edits: in place, write-back patches, CSS dev loader |
| A5 | Forced states and overrides on a live app leak into app code | Blocking | §Invisible to the app, with contract tests |
| A6 | Arena ids recycle; path matching breaks on reorder | Blocking | §Node identity with explicit match methods |
| A7 | Full captures and films too large | Non-blocking | Layered captures, deltas, dedup (§Scale) |
| A8 | Public API sprawl in `rdom-tui` | Non-blocking | One `rdom_tui::capture` module, `App::capture` entry point |
| A9 | PNG goldens brittle | Non-blocking | Grid hash as main oracle; few pinned PNG goldens |
| A10 | Live attach threading, permissions, frame stalls | Non-blocking | §Live attach: polled in the loop, 0600, budget per request |

### Chief TUI designer

| # | Finding (v1) | Severity | v2 |
|---|---|---|---|
| D1 | One fixed palette and font; square cells mislead spacing; no real-terminal reference | Blocking | §Terminal profiles (palette, capabilities, aspect), `emulator` view, aspect-aware rhythm |
| D2 | Covers look but not UX | Blocking | Outline first; tab order, hit targets, reachability, latency (§Audits: UX) |
| D3 | No content stress | Blocking | `stress` |
| D4 | Tokens barely present | Non-blocking | `tokens` layer, token edits, design-system-aware audits |
| D5 | Motion only measured, not judged; no reduced motion; no bandwidth | Non-blocking | Motion audits, reduced-motion axis, bandwidth profiles |
| D6 | No way to work from a design | Non-blocking | `mockup` |

### Thesis-level additions (from "better than browser integration")

Batched counterfactuals (`whatif`), budgeted answers, reverse queries, a persistent session with a schema-first
protocol, record and replay, exact identity-aware diffs.

### Dropped from v1

- The "pushed override sheet", replaced by in-place edits.
- SVG as a first-class view, replaced by PNG plus the HTML inspector.
- One-shot CLI verbs as the main interface, replaced by the persistent session; the CLI is now a client.
