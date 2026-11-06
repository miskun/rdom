# CAPTURE — an agent-friendly renderer and inspector

**Status:** PROPOSED (2026-11-07). The contract is up for review; nothing is implemented yet.

## Why

rdom paints to a terminal, and nothing else can see that paint. Snapshot tests can (`rdom-showcase/tests/common`, the
acid references), but only through test-only helpers, and only as glyph text plus a background-key layer. An
agent or a person improving how an app *looks* has three things missing:

- **A way to see any page at any moment**, outside a test and outside a live terminal: as structured data, as an
  image and as a page in a browser.
- **A way to understand it.** Which node painted this cell, which rule gave it this color, where that rule is
  written, and what it overrode.
- **A way to change things cheaply.** Try a style change, a viewport size, a color depth, a forced `:hover`, and see
  the result in milliseconds, without recompiling the app.

This initiative adds no CSS features. It's an instrument: once it exists, every feature that already exists or is
added later (gradients, border animations, glyph effects) can be seen, explained and measured with it, with no
extra work per feature.

## The designer's loop (what the API is for)

The primary user is an agent asked to review an app's UI/UX and to make it look better. A human designer does
the same loop. Each step needs something specific:

| Step | The agent asks | The API answers with |
|---|---|---|
| **Observe** | What does this screen look like? | An image I can look at (PNG: agents take raster images, not SVG source) *and* an exact text grid I can reason over |
| **Understand** | What is this region, why is it this color, what's its box? | Cell → owner node → box model → computed style → winning declaration → **source file:line** |
| **Audit** | What's objectively wrong? | Machine findings: contrast, truncation, clipping, misalignment, spacing and palette sprawl, invisible focus, color-depth loss |
| **Hypothesize + try** | What if the border were dimmer, the gap 2? | A CSS override applied to a session, re-captured, with no rebuild |
| **Compare** | Did it get better, and did anything else change? | A before/after pair plus a diff: cells changed, by node, by property |
| **Cover** | Does it hold up at 80×24, in light mode, in 16 colors, focused, hovered, with long text? | A variant matrix rendered to one contact sheet |
| **Motion** | Is the spinner smooth, does the fade ease, does anything flicker? | A film: captures at fixed virtual-clock steps, plus numeric timelines |
| **Land** | Where do I make the change for real? | Provenance pointing at the rule's source, and the override as a diff against it |
| **Hand off** | Can a human see what I did? | A static HTML report: before/after, findings, inspector |

What makes an agent effective here, in order of importance:

1. **Understanding traced back to source.** Pixels alone send an agent guessing. "Cell (12,4) is `#5a5a5a` because
   `.panel > .title { color: var(--muted) }` at `app.css:41` beat `h2 { color: … }` at `ua.css`" lets it edit the
   right line the first time.
2. **No compile in the loop.** A Rust rebuild per tweak makes design iteration take minutes. Overrides, forced
   states and variants are all runtime operations on a session.
3. **Text first, image second.** An exact grid plus findings is precise and cheap. The image is for the overall
   impression (balance, density, hierarchy), which text can't show.
4. **Determinism.** The same scenario at the same virtual time gives the same capture, byte for byte. Otherwise
   before/after diffs are noise.
5. **Fast.** One capture in well under 100 ms for a typical page, so a session can sweep many variants.

## Principles

- **One contract.** A `Capture` (JSON, versioned schema) is the single source. Text, SVG, PNG and HTML are views
  derived from it. Every producer (in-process session, live attach) emits it, and every consumer (exporters,
  inspector, audits, diff) reads only it.
- **The capture describes paint, it doesn't imitate a terminal.** It records what rdom painted, with a stated
  palette and font. Real-terminal differences (fonts, ambiguous-width glyphs, the 16-color theme) are known
  limits, recorded in `meta`, not hidden.
- **Explainable findings.** Every audit finding names its rule, its inputs (cells, nodes, values), its threshold, and
  what it didn't check.
- **Zero cost when not capturing.** Owner attribution and provenance are recorded only on a capture paint. The
  normal frame path is unchanged; `frame_cost_tests` must stay green.
- **Safe live attach.** Attaching to a running app is opt-in, read-only by default, local-only, and can't disturb
  terminal state. Overrides on a live app are explicit, reversible and listed.

## The `Capture` contract (schema v1, draft)

```jsonc
{
  "schema": "rdom-capture/1",
  "meta": {
    "rdom_tui": "0.6.5",
    "time_ms": 300,              // virtual clock since session start
    "viewport": [120, 40],
    "color_depth": "truecolor",  // truecolor | 256 | 16
    "color_scheme": "dark",
    "palette": "xterm-default",  // how the 16 ANSI colors map to RGB in views
    "overrides": ["o1"],         // the overrides active in this capture
    "forced_states": [{"node": 17, "state": "hover"}],
    "scenario": "settings.toml#step-3"
  },
  "grid": {
    // run-length rows: consecutive cells with equal attributes share a run
    "rows": [
      [{"x": 0, "text": "┌ Settings ", "fg": "#c0c0c0", "bg": "#1e1e1e", "mods": ["bold"],
        "ul": null, "owner": {"node": 12, "part": "border"}}]
    ]
  },
  "nodes": {
    "12": {
      "tag": "section", "id": "settings", "classes": ["panel"], "attrs": {"aria-label": "Settings"},
      "parent": 3, "children": [13, 14],
      "state": ["focus-within"],
      "path": "main > section#settings.panel",
      "boxes": {
        "border": [0, 0, 60, 20], "padding": [1, 1, 58, 18], "content": [2, 1, 56, 18],
        "scroll": {"offset": [0, 4], "size": [56, 40]},
        "clip": [0, 0, 60, 20],
        "layer": 0
      },
      "style": {
        "color": {
          "value": "#c0c0c0",
          "from": {"rule": 41, "decl": "color: var(--fg)"},
          "overridden": [{"rule": 7, "decl": "color: CanvasText"}]
        }
      }
    }
  },
  "rules": {
    "41": {"selector": ".panel", "sheet": "app", "origin": "author", "layer": null,
           "specificity": [0, 1, 0], "source": {"file": "src/style.css", "line": 41}},
    "7":  {"selector": "section", "sheet": "ua", "origin": "ua", "source": {"file": "ua", "line": null}}
  },
  "vars": {"--fg": {"value": "#c0c0c0", "defined_on": 1, "rule": 3}}
}
```

Contract points:

- `owner.part` is one of `content`, `border`, `background`, `outline`, `scrollbar`, `caret`, `selection`,
  `pseudo:before`, `pseudo:after`, `pseudo:marker`, `ua-chrome`, `highlight:<name>`. A cell painted by several layers
  records the topmost; `stack` (opt-in) lists every layer that painted it, in paint order.
- `style` is complete for computed values and sparse for provenance: by default only properties that differ from
  the initial value carry `from`/`overridden`. `--full-style` includes everything.
- `source` comes from the CSS parser (file + line of the rule) or, for rules built in Rust
  (`Stylesheet::rule`), from `#[track_caller]` (`file.rs:line`). A rule with no known source says so (`null`),
  and never invents one.
- Colors are always emitted as resolved sRGB, with the original form beside it when it differs (`"spec": "ansi:4"`,
  `"spec": "oklch(…)"`), so audits work on real values and views can show what the author wrote.
- Node ids are the arena `NodeId`s, valid inside one capture. Diffs match nodes across captures by `path`, then by
  id.

## Views

| View | For | Notes |
|---|---|---|
| `text` | agents, tests | The existing snapshot format, made public: glyph grid + background-key layer, plus optional fg-key and owner-key layers. Owner keys are labeled with node paths. |
| `png` | agents with image input, reports | Rasterized from the grid at fixed cell size with a bundled monospace font that covers box-drawing and blocks. Optional 2× scale. Optional overlays: box outlines, a grid, a highlighted node. |
| `svg` | humans, docs | One `<rect>` per background run, `<text>` with an explicit x per cell (no browser font-metric drift). |
| `html` | humans, hand-off | `inspect.html`: the SVG plus an Elements-panel inspector (hover a cell → node, box, styles, rules, source). Loads one capture or a series (filmstrip + scrubber). Static, self-contained, no server. |
| `cast` | motion review | asciicast v2 of the real emitted ANSI bytes, so the wire path (SGR, frame diff) is checked too, not just the model. |

## Operations

A Rust library (`Session`) and a CLI built on it. The app's code builds the `App`, so the CLI is a function the
app calls from a small `examples/capture.rs`: `rdom_capture::cli::main(|| build_app())`. After that, everything
is CLI verbs and files, with no app-specific code.

| Verb | Does |
|---|---|
| `shot` | Run a scenario (or none) and write a capture plus the chosen views |
| `inspect` | Query a capture: `--at x,y`, `--select "<css>"`, `--node <id>`; prints node, box, style, provenance |
| `override` | Apply a CSS snippet (a pushed author sheet, last in order) or set custom properties, then capture |
| `force` | Force pseudo-class states on nodes (`:hover`, `:focus-visible`, `:active`, `:checked`, `:disabled`) |
| `matrix` | Render a cross product of variants (sizes × scheme × color depth × states × overrides) to one contact sheet + captures |
| `diff` | Compare two captures: changed cells (count and a heatmap), changed nodes and boxes, changed computed properties with their provenance |
| `film` | Capture at a fixed virtual-clock step over a window; write the series, a filmstrip and timelines |
| `audit` | Run the checks below and print findings (JSON + text) |
| `attach` | The same verbs against a running app (see Live attach) |

Scenarios are files (TOML): viewport, color depth, scheme, then steps (`key`, `type`, `click`, `scroll`,
`resize`, `advance`, `force`, `override`, `capture "<name>"`).

## Audits (first set)

Each finding: `{check, severity, nodes, cells, inputs, threshold, message, not_checked}`.

- **contrast:** fg/bg contrast per text run (WCAG 2 ratio and APCA Lc), against the resolved colors *after* color-depth
  quantization.
- **truncation and clipping:** text cut by `overflow: hidden`/`clip` with no ellipsis; content outside every
  scrollport; zero-size elements that have text.
- **occlusion:** text painted and then fully covered by a later layer.
- **alignment:** box edges that miss a shared column or row by one cell (likely-unintended near-misses).
- **rhythm:** the set of gaps, paddings and margins in use; flags values off the dominant scale.
- **palette sprawl:** distinct colors in use, clustered; flags near-duplicates (ΔE OKLab under a threshold).
- **focus visibility:** for each focusable element, does `:focus-visible` change at least one painted cell, and by
  enough contrast?
- **color-depth loss:** colors that become indistinguishable at 256 or 16 colors (borders that disappear, states
  that look the same).
- **motion** (on a film): flicker (a cell that changes and reverts within one step), spinner period and width jitter,
  easing fit for color and geometry tweens, cells changed and bytes emitted per frame.

Audits make a design review concrete; they don't replace taste. The report shows the findings next to the images
so a human makes the call.

## Live attach

An app opts in (`App::with_inspector(...)` or `RDOM_INSPECT=<path>`). The `App` then serves a local channel (a Unix
socket or named pipe inside a path the app chooses; no TCP) and handles requests between frames: `capture`,
`inspect`, and, if enabled, `override`/`force`, which are listed in `meta` and removed by `reset`. It never writes to the
terminal outside its normal frame, and a client that disconnects mid-request leaves no state behind.

This is what makes "any page at any moment" work for the real app in a real state that's hard to reach from a
script.

## Where it lives

- **`rdom-tui`** (substrate work, public API, no test feature gate):
  - a capture paint: the same paint pass, writing an owner layer (`NodeId` + part) beside the buffer;
  - box geometry per node (border, padding, content, scroll, clip, layer);
  - per-property cascade provenance (winner + overridden), the public successor of the test-only
    `App::matched_rules`;
  - rule source locations (CSS parser: file + line; Rust builders: `#[track_caller]`);
  - a forced-state hook for user-action pseudo-classes.
- **`rdom-capture`** (new published crate, depends on `rdom-tui`): `Capture` types and schema, `Session`, the views,
  audits, diff, film, CLI helpers, the attach client and server glue, `inspect.html`.
- `rdom-core` gains nothing. `rdom-tui` doesn't depend on `rdom-capture`.

## Phases

Each phase is TDD, lands green, and is pinned against the acid pages (a known, spec-derived ground truth).

1. **Contract.** `Capture` types + JSON schema + round-trip tests; this document reviewed.
2. **Grid + owners.** Capture paint with an owner layer; `Session` (`App::with_backend` + `advance`); `text` and `json`
   views. Test: every acid tile's cells are owned by the tile's nodes; normal paint cost unchanged.
3. **Boxes + provenance + sources.** Per-property winners with rule source locations. Test: acid tiles whose
   reference names the winning rule.
4. **Images.** `svg` and `png` (bundled font) + `inspect.html`. Test: golden PNG/SVG for acid page 1.
5. **Iteration.** `override`, `force`, `matrix`, `diff`, the scenario format, the CLI.
6. **Audits.** The first set, each with a positive and a negative fixture.
7. **Motion.** `film`, timelines, motion audits, `cast`.
8. **Live attach.**

Phases 1–4 make everything visible and explainable. Phase 5 is what makes an agent productive.

## Open questions

- **PNG rasterization:** `resvg`/`tiny-skia` with an embedded font (license and size; must cover box drawing,
  blocks, Braille, and a fallback for wide glyphs) versus a minimal hand-rolled glyph rasterizer. Feature-gated
  either way.
- **Provenance cost:** recording winners for every property on every capture versus on request per node. Measure
  before deciding.
- **Attach transport on Windows:** named pipe parity, or capture-to-file only at first.
- **Schema stability:** `rdom-capture/1` is versioned from the start. Which fields are guaranteed versus
  informational (e.g. `stack`) needs deciding before the crate's first release.
- **Classification:** every new public type is listed in `DESIGN.md` §"Which public types are `#[non_exhaustive]`"
  when it lands; `Capture` and its parts are likely open (`#[non_exhaustive]`), since the schema will grow.

## Non-goals

- New CSS features (gradients and the like land on their own, and then show up here for free).
- Emulating a specific terminal pixel-for-pixel.
- A visual editor. The tools make an agent or designer precise; changes still land in the app's source.
