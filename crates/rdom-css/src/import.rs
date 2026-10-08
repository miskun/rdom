//! `@import` (CSS Cascade 5 §3).
//!
//! `@import [ <url> | <string> ] [ layer | layer(<layer-name>) ]?
//! [ supports( … ) ]? <media-query-list>? ;`
//!
//! rdom has no network or filesystem policy of its own, so the sheet is
//! fetched through the host's [`ImportLoader`]. Its rules are parsed
//! into the importing sheet at the import's position — before the
//! importing sheet's own rules, as an import must lead the sheet —
//! inside the `layer` / `layer(name)` layer if one is given, under the
//! media list as an `@media` rule would put them. The `supports()` and
//! media conditions are also recorded on the
//! [`Import`](rdom_style::Import) record; until `@supports` lands
//! (C14-SUPPORTS) the `supports()` one counts as true.
//!
//! The loader resolves each URL against the importing sheet's URL
//! ([`ImportLoader::load_from`]) and returns the sheet's resolved URL,
//! which is its identity: an `@import` of a sheet already open — the
//! root included, when its URL is known (`parse_with_loader_at`) — closes
//! a cycle and is skipped (`ImportCycle`). Imports nest at most
//! [`MAX_IMPORT_DEPTH`](crate::MAX_IMPORT_DEPTH) deep (`ImportTooDeep`).
//!
//! An `@import` after any rule other than `@charset` and `@layer`
//! statements (or inside a block) is ignored (`ImportIgnored`); no
//! loader, or a loader error, imports nothing (`ImportFailed`). Warnings
//! inside an imported sheet carry positions in that sheet's text.

use rdom_style::parse::SourceCursor;
use rdom_style::{Import, LayerId, Stylesheet};

use crate::layer::{layer_names, read_prelude};
use crate::top_level::{parse_rule_list, skip_balanced_block};
use crate::{ImportLoader, Warning, WarningKind};

/// The import state of one parse: the loader, and the resolved URLs of
/// the sheets being parsed, outermost first — the root's own when the
/// host gave it — for cycle detection and as each import's base URL.
pub(crate) struct Imports<'l> {
    pub loader: Option<&'l dyn ImportLoader>,
    pub stack: Vec<String>,
    /// How many imported sheets are open (the root not counted).
    pub depth: usize,
}

impl<'l> Imports<'l> {
    /// The state for parsing a root sheet at `url`.
    pub(crate) fn new(loader: Option<&'l dyn ImportLoader>, url: Option<&str>) -> Self {
        Imports {
            loader,
            stack: url.map(str::to_string).into_iter().collect(),
            depth: 0,
        }
    }
}

/// What an at-rule at the head of `rest` (which starts with `@`) means
/// for a later `@import`.
pub(crate) enum Leading {
    /// It is an `@import`.
    Import,
    /// `@charset` or an `@layer` statement: imports may still follow.
    KeepsImports,
    /// Anything else ends the imports.
    Other,
}

pub(crate) fn leading_at_rule(rest: &str) -> Leading {
    let (name, used) = rdom_core::css_syntax::consume_ident(&rest[1..]);
    if name.eq_ignore_ascii_case("import") {
        return Leading::Import;
    }
    if name.eq_ignore_ascii_case("charset") {
        return Leading::KeepsImports;
    }
    if name.eq_ignore_ascii_case("layer") {
        // A statement ends at `;` before any `{`.
        let after = &rest[1 + used..];
        let statement = match (after.find(';'), after.find('{')) {
            (Some(semi), Some(brace)) => semi < brace,
            (Some(_), None) => true,
            _ => false,
        };
        if statement {
            return Leading::KeepsImports;
        }
    }
    Leading::Other
}

/// Consume an `@import` rule, the cursor on its `@`. `imports` is
/// `None` where an import is not allowed. `layer` is the layer the
/// importing sheet's rules sit in.
pub(crate) fn consume_import(
    cursor: &mut SourceCursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    layer: Option<LayerId>,
    imports: Option<&mut Imports<'_>>,
) {
    let at = (cursor.line(), cursor.col());
    let warn = |warnings: &mut Vec<Warning>, kind| {
        warnings.push(Warning {
            kind,
            line: at.0,
            column: at.1,
        })
    };
    cursor.bump(); // '@'
    let (_, used) = rdom_core::css_syntax::consume_ident(cursor.rest());
    cursor.advance(used);
    let Some(prelude) = read_prelude(cursor, warnings) else {
        return;
    };
    match cursor.peek() {
        Some('{') => {
            skip_balanced_block(cursor);
            warn(warnings, invalid(&prelude));
            return;
        }
        Some(';') => {
            cursor.bump();
        }
        _ => {}
    }
    let Some(parsed) = parse_prelude(&prelude) else {
        warn(warnings, invalid(&prelude));
        return;
    };
    let Some(imports) = imports else {
        warn(warnings, WarningKind::ImportIgnored(parsed.url));
        return;
    };
    if imports.depth >= crate::MAX_IMPORT_DEPTH {
        warn(warnings, WarningKind::ImportTooDeep(parsed.url));
        return;
    }
    let base = imports.stack.last().map(String::as_str);
    let loaded = match imports.loader {
        None => Err("no import loader".to_string()),
        Some(loader) => loader.load_from(&parsed.url, base),
    };
    let loaded = match loaded {
        Ok(loaded) => loaded,
        Err(reason) => {
            warn(
                warnings,
                WarningKind::ImportFailed {
                    url: parsed.url,
                    reason,
                },
            );
            return;
        }
    };
    // The same sheet is the same resolved URL, however it is spelled.
    if imports.stack.contains(&loaded.url) {
        warn(warnings, WarningKind::ImportCycle(parsed.url));
        return;
    }
    let into = match &parsed.layer {
        None => layer,
        Some(None) => Some(sheet.declare_anonymous_layer(layer)),
        Some(Some(name)) => {
            let segments: Vec<&str> = name.iter().map(String::as_str).collect();
            sheet.declare_layer(layer, &segments)
        }
    };
    // CSS Cascade 5 §3: the imported rules apply while the media list
    // matches.
    let mut ctx = rdom_style::RuleContext::default().in_layer(into);
    if let Some(media) = &parsed.media {
        let queries = rdom_style::conditional::MediaList::parse(media);
        ctx = crate::conditional::declare(sheet, ctx, rdom_style::ConditionKind::Media(queries));
    }
    sheet.record_import(Import::new(
        parsed.url.clone(),
        into,
        parsed.supports,
        parsed.media,
    ));
    imports.stack.push(loaded.url);
    imports.depth += 1;
    let mut inner = SourceCursor::new(&loaded.text);
    parse_rule_list(&mut inner, sheet, warnings, ctx, Some(imports));
    imports.depth -= 1;
    imports.stack.pop();
}

fn invalid(prelude: &str) -> WarningKind {
    WarningKind::InvalidAtRulePrelude {
        name: "import".to_string(),
        prelude: prelude.trim().to_string(),
    }
}

/// An `@import` prelude.
struct Prelude {
    url: String,
    /// `None`: no layer; `Some(None)`: `layer`; `Some(Some(name))`:
    /// `layer(name)`, as segments.
    layer: Option<Option<Vec<String>>>,
    supports: Option<String>,
    media: Option<String>,
}

fn parse_prelude(prelude: &str) -> Option<Prelude> {
    let rest = prelude.trim();
    let (url, rest) = url(rest)?;
    let mut rest = rest.trim_start();
    let mut layer = None;
    if let Some(after) = function(rest, "layer") {
        let (inner, after) = crate::scope::parenthesized(after)?;
        let names = layer_names(inner)?;
        let [name] = names.as_slice() else {
            return None;
        };
        layer = Some(Some(name.clone()));
        rest = after.trim_start();
    } else if let Some(after) = keyword(rest, "layer") {
        layer = Some(None);
        rest = after.trim_start();
    }
    let mut supports = None;
    if let Some(after) = function(rest, "supports") {
        let (inner, after) = crate::scope::parenthesized(after)?;
        supports = Some(inner.trim().to_string());
        rest = after.trim_start();
    }
    let media = (!rest.is_empty()).then(|| rest.to_string());
    Some(Prelude {
        url,
        layer,
        supports,
        media,
    })
}

/// `url(…)` (quoted or not) or a string, and the rest.
fn url(text: &str) -> Option<(String, &str)> {
    if let Some(q @ ('"' | '\'')) = text.chars().next() {
        let (value, used) = rdom_core::css_syntax::consume_string(&text[1..], q)?;
        return Some((value, &text[1 + used..]));
    }
    let after = function(text, "url")?;
    let close = after.find(')')?;
    let inner = after[1..close].trim();
    let value = match inner.chars().next() {
        Some(q @ ('"' | '\'')) => rdom_core::css_syntax::consume_string(&inner[1..], q)?.0,
        _ => inner.to_string(),
    };
    (!value.is_empty()).then_some((value, &after[close + 1..]))
}

/// `name(` at the head of `text` (ASCII case-insensitive): the text from
/// the `(` on.
fn function<'t>(text: &'t str, name: &str) -> Option<&'t str> {
    let head = text.get(..name.len())?;
    let after = &text[name.len()..];
    (head.eq_ignore_ascii_case(name) && after.starts_with('(')).then_some(after)
}

/// The keyword `name` alone at the head of `text`: the text after it.
fn keyword<'t>(text: &'t str, name: &str) -> Option<&'t str> {
    let head = text.get(..name.len())?;
    let after = &text[name.len()..];
    let ends = after.chars().next().is_none_or(|c| c.is_whitespace());
    (head.eq_ignore_ascii_case(name) && ends).then_some(after)
}
