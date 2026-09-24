# rdom-parser

HTML-ish template parser for [`rdom-core`](../rdom-core/). The
`parseFromString` equivalent for the rdom family.

Hand-rolled recursive descent. Zero runtime dependencies beyond
rdom-core.

## Quick start

```rust
use rdom_parser::parse;
use rdom_core::Dom;

let (dom, ids): (Dom<()>, _) = parse(r#"
    <div class="hero" id="main">
        <h1>Welcome</h1>
        <p>Hello &amp; <strong>world</strong></p>
    </div>
"#).unwrap();
```

For building into an existing tree:

```rust
use rdom_parser::parse_into;

let mut dom: Dom<()> = Dom::new();
let body = dom.create_element("body");
dom.append_child(dom.root(), body).unwrap();

let children = parse_into(&mut dom, "<h1>Title</h1><p>Body</p>", body)?;
```

## Supported

- Start, end, self-closing tags; case-insensitive tag names (normalized to lowercase)
- Void elements (`<br>`, `<hr>`, `<img>`, `<input>`, …) auto-close
- Attributes: `name="value"`, `name='value'`, `name=value`, `name` (boolean)
- `class="a b c"` populates the classList
- Text with character references: the full WHATWG named table (2 231 entries, legacy no-`;` names like `&amp` and `&copy` included, with HTML's attribute-value rule) plus `&#NNN;` / `&#xHH;`; invalid numeric references decode to U+FFFD
- A `<` not followed by a letter, `/`, `!`, or `?` is text — `a < b` needs no escaping
- `<style>` / `<script>` bodies are raw text; `<textarea>` / `<title>` bodies are RCDATA
- Comments: `<!-- … -->` preserved as Comment nodes; `<?…>` and non-DOCTYPE `<!…>` become Comment nodes too
- `<!DOCTYPE …>` is skipped
- Full UTF-8: CJK, emoji, ZWJ sequences, combining marks all preserved correctly

## Not supported

- Tree-construction recovery — a mismatched, missing, or stray end tag is an error, not auto-repaired
- CDATA sections as CDATA (they become bogus comments), namespace prefixes

## Error reporting

`ParseError` carries line + column + byte offset + optional hint:

```rust
use rdom_parser::parse;
use rdom_core::Dom;

let err = parse::<()>("<div><span></p></div>").unwrap_err();
assert!(err.msg.contains("mismatched"));
assert!(err.hint.is_some());
println!("{err}"); // "parse error at line 1, col 14: …"
```

## Round-tripping

For a well-behaved subset, `parse(x).outer_markup() == x`:

```rust
let src = "<div><p>Hello &amp; world</p></div>";
let (dom, ids) = parse::<()>(src).unwrap();
assert_eq!(dom.outer_markup(ids[0]), src);
```

Caveats: attributes serialize in alphabetic order, classes serialize
in alphabetic order, text entities only escape `& < >` (not `"` or `'`
outside attributes).

## Examples

```bash
cargo run -p rdom-parser --example parse_html
cargo run -p rdom-parser --example round_trip
```

`parse_html` walks a parsed tree and runs selector queries;
`round_trip` verifies parse ↔ serialize equivalence on a corpus
of well-behaved snippets.

For an end-to-end "parse → cascade → paint to terminal" demo, see
rdom-tui's [`parse_and_render` example](../rdom-tui/examples/parse_and_render.rs).

## Testing

```bash
cargo test -p rdom-parser
```

Unit, integration and doc tests covering parsing, entity decoding, nesting, errors, round-tripping, realistic template snippets, and Unicode content.
