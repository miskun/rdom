# rdom-core

The pure DOM — arena-backed nodes, CSS selectors, events,
`MutationObserver`. Zero rendering dependencies, zero
presentation concerns.

Use this crate directly for:

- Building + querying an in-memory HTML-ish tree.
- Dispatching events with full capture → target → bubble
  semantics (stop-propagation, prevent-default, `AbortSignal`).
- Observing tree mutations for reactive bindings, a11y mirrors,
  devtools, or cascade invalidation.
- Serializing a subtree back to markup (`outer_markup`,
  `inner_markup`).

If you want to *paint* a tree to a terminal, pair it with
[`rdom-tui`](../rdom-tui/). If you want to *parse* HTML-ish
templates into a tree, pair it with
[`rdom-parser`](../rdom-parser/).

## Quick start

```rust
use rdom_core::Dom;

let mut dom: Dom = Dom::new();
let root = dom.root();

let app = dom.create_element("div");
dom.set_attribute(app, "id", "app").unwrap();

let h1 = dom.create_element("h1");
let t = dom.create_text_node("Hello, rdom!");
dom.append_child(h1, t).unwrap();
dom.append_child(app, h1).unwrap();
dom.append_child(root, app).unwrap();

// CSS queries.
assert_eq!(dom.query_selector_all_in(root, "h1").unwrap().len(), 1);

// Walk the tree with the accessor API.
assert_eq!(dom.node(h1).text_content(), "Hello, rdom!");
assert_eq!(dom.node(app).get_attribute("id"), Some("app"));

// Serialize.
println!("{}", dom.outer_markup(app));
// <div id="app"><h1>Hello, rdom!</h1></div>
```

See [`examples/tree_builder.rs`](examples/tree_builder.rs) for a
longer tour.

## Generic over the "presentation ext"

`Dom<Ext>` is generic over a per-element extension struct. Pure
rdom-core uses `Ext = ()` (the `Dom` alias). Presentation layers
parameterize with their own struct to hang layout / style /
rendering state off each element without touching core code.

```rust
// In rdom-tui:
pub type TuiDom = Dom<TuiExt>;
```

Core operates entirely through generic `NodeId` + attributes +
classes + children. It never reads or mutates the `Ext` data.

## Selectors

Full spec-subset matching via `query_selector`, `query_selector_all`,
`matches`, `closest`:

| | Syntax |
|---|---|
| type | `div`, `h1` |
| universal | `*` |
| id | `#app` |
| class | `.hero` |
| attribute | `[lang]`, `[lang="en"]`, `[class~="a"]`, `[class|="a"]`, `[href^="http"]`, `[src$=".png"]`, `[href*="://"]` |
| compound | `a.active[href="#"]` |
| descendant | `ul li` |
| child | `ul > li` |
| next-sibling | `h1 + p` |
| subsequent | `h1 ~ p` |
| pseudo-classes | `:not(...)`, `:first-child`, `:last-child`, `:only-child`, `:empty`, `:root`, `:hover`, `:focus` |

Pseudo-elements (`::before`, `::after`) are recognized as selector
suffixes; they're extracted before parsing and delivered to
rdom-tui's cascade separately. Core doesn't know what a
pseudo-element *means*, only that the suffix exists.

## Events

Capture → target → bubble dispatch with spec semantics:

```rust
use rdom_core::{Dom, Event, ListenerOptions};

let mut dom: Dom = Dom::new();
let root = dom.root();
let btn = dom.create_element("button");
dom.append_child(root, btn).unwrap();

dom.add_event_listener(btn, "click", ListenerOptions::default(), |ctx| {
    println!("clicked!");
    ctx.event.stop_propagation();   // don't bubble further
    ctx.event.prevent_default();    // cancel the default action
})
.unwrap();

let mut ev = Event::new("click");
dom.dispatch_event(btn, &mut ev).unwrap();
```

Options include `capture`, `once`, and `AbortSignal` for scope-bound
cleanup (`signal.abort()` removes every listener that shares it).

## Mutations + observers

Every mutation entry point (`set_attribute`, `add_class`,
`append_child`, `set_node_value`, `set_hovered`, `set_focused`,
`set_selection`, ...) fires a `Mutation` record to every
registered observer:

```rust
struct Logger;
impl MutationObserver<()> for Logger {
    fn observe(&mut self, _dom: &mut Dom<()>, record: &Mutation) {
        println!("mutation: {record:?}");
    }
}

let mut dom: Dom = Dom::new();
dom.add_mutation_observer(Box::new(Logger));

let d = dom.create_element("div");
dom.append_child(dom.root(), d).unwrap();
dom.add_class(d, "primary").unwrap();
// Logger sees: ChildListChanged, ClassChanged.
```

Observers are the single invalidation mechanism — rdom-tui's
`DirtyTracker` is one registered observer. A reactive framework or
devtools mirror would register its own without touching core.

`MutationObserver::observe` panics if it attempts to mutate the
Dom — enforced by an `is_observing` re-entrancy guard.

## Mutation record types

- `AttributeChanged { id, name, old, new }`
- `ClassChanged { id, added, removed }`
- `ChildListChanged { parent, added, removed }`
- `CharacterDataChanged { id, old, new }` — text / comment node data
- `InteractionChanged { prev, next, kind }` — hover / focus
- `SelectionChanged { prev, next }` — document selection

See [`examples/mutation_observer.rs`](examples/mutation_observer.rs)
for a logger that prints every record type.

## AbortSignal

Modern-HTML cancellation pattern. Create a controller, share its
signal across listener registrations, abort once to remove them all:

```rust
use rdom_core::AbortController;

let controller = AbortController::new();
let opts = ListenerOptions::default().signal(controller.signal());

dom.add_event_listener(a, "click", opts.clone(), handler_a)?;
dom.add_event_listener(b, "click", opts.clone(), handler_b)?;

// ... some time later ...
controller.abort();  // removes both listeners
```

Supersedes the older `remove_event_listener` workflow for
scope-bound cleanup (widgets unmounting, components tearing down).

## Serialization

`outer_markup(id)` emits a subtree as HTML-ish text:

```rust
let out = dom.outer_markup(root);
// <div id="app"><h1>Hello</h1></div>
```

`inner_markup(id)` omits the element's own tag, emitting only its
children. For most inputs in the defined subset, round-tripping
via [`rdom-parser`](../rdom-parser/) yields the original source —
see `rdom-parser`'s `examples/round_trip.rs`.

## Benchmarks

```text
cargo bench -p rdom-core --bench arena
```

Currently covers arena-construction and tree-walk micros. Full
dispatch + cascade benches live in
[`rdom-tui`](../rdom-tui/benches/runtime.rs).

## Further reading

- [`DESIGN.md`](../../specs/DESIGN.md) — architectural overview: crate map, non-negotiable invariants, roadmap.
- [`DIVERGENCES.md`](../../specs/DIVERGENCES.md) — every deliberate departure from the web platform.
- Module docs under `src/` — each file has a top-level doc comment covering its specific role.
