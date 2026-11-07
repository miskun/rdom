//! Writing an edit back to the CSS text it came from
//! (`specs/CAPTURE.md` §Edits, write-back): find the rule where its
//! location says — and check its selector is still there — then the
//! declarations at the top level of its block that set what the property
//! sets (`property_dispatch::overlaps`). A new value replaces the last of
//! them when that one is the property's own, else it is appended, after
//! whatever shorthand decides the property now. A removal drops every
//! declaration of the property, and is refused, with why, when the block
//! sets it only through a shorthand or not at all: the text cannot say
//! what the edit did.

/// One declaration change to write into a rule's block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclEdit {
    pub property: String,
    /// The new value; `None` removes the declaration.
    pub value: Option<String>,
    pub important: bool,
}

/// `text` with `edit` applied to the rule whose selector starts at
/// `line:column` (1-based, columns in characters). `Err` when no block
/// follows the location.
pub(crate) fn apply(
    text: &str,
    line: u32,
    column: u32,
    selector: &str,
    edit: &DeclEdit,
) -> Result<String, String> {
    let start =
        offset(text, line, column).ok_or_else(|| format!("{line}:{column} is past the text"))?;
    let open = text[start..]
        .find('{')
        .map(|i| start + i)
        .ok_or_else(|| format!("no rule block after {line}:{column}"))?;
    // The rule must still be where it was parsed: its selector (an item of
    // the list there) between the location and the block.
    if !squash(&text[start..open]).contains(&squash(selector)) {
        return Err(format!(
            "the rule at {line}:{column} is no longer `{selector}` (the file changed since it was parsed)"
        ));
    }
    let close = block_end(text, open)
        .ok_or_else(|| format!("the block at {line}:{column} is not closed"))?;
    let at = open + 1;
    let body = &text[at..close];
    let decls = top_level_declarations(body);
    let name_of = |d: &Decl| body[d.name.clone()].trim().to_string();
    // The declarations that set what the property sets: the last of them
    // decides it.
    let touching: Vec<&Decl> = decls
        .iter()
        .filter(|d| rdom_style::property_dispatch::overlaps(&name_of(d), &edit.property))
        .collect();
    let own = |d: &Decl| name_of(d).eq_ignore_ascii_case(&edit.property);
    match &edit.value {
        Some(v) => match touching.last() {
            // The last declaration of it is its own: its value in place.
            Some(d) if own(d) => {
                let mut out = String::with_capacity(text.len() + v.len());
                out.push_str(&text[..at + d.value.start]);
                out.push(' ');
                out.push_str(v);
                if edit.important {
                    out.push_str(" !important");
                }
                out.push_str(&text[at + d.value.end..]);
                Ok(out)
            }
            // Not declared, or a later shorthand decides it: appended last.
            _ => Ok(append(text, start, open, close, &decls, edit, v)),
        },
        None => {
            if touching.is_empty() {
                return Err(format!("the rule declares no {}", edit.property));
            }
            if let Some(d) = touching.iter().find(|d| !own(d)) {
                return Err(format!(
                    "the rule sets {} through `{}`: rewrite that declaration by hand",
                    edit.property,
                    name_of(d)
                ));
            }
            // Last first, so the earlier ranges hold.
            let mut out = text.to_string();
            for d in touching.iter().rev() {
                out = remove(&out, at + d.whole.start, at + d.whole.end);
            }
            Ok(out)
        }
    }
}

/// `text` with the declaration at `from..to` removed, with its `;` and the
/// line it sat on when it was alone there.
fn remove(text: &str, mut from: usize, mut to: usize) -> String {
    if text[to..].starts_with(';') {
        to += 1;
    }
    let line_start = text[..from].rfind('\n').map_or(0, |i| i + 1);
    if text[line_start..from].trim().is_empty() && text[to..].starts_with('\n') {
        from = line_start;
        to += 1;
    }
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..from]);
    out.push_str(&text[to..]);
    out
}

/// `text` with `property: value` appended to the block `open..close` of
/// the rule at `start`, indented as the block's last declaration (or two
/// spaces in from the rule).
fn append(
    text: &str,
    start: usize,
    open: usize,
    close: usize,
    decls: &[Decl],
    edit: &DeclEdit,
    value: &str,
) -> String {
    let body = &text[open + 1..close];
    let indent = decls
        .last()
        .map(|d| {
            let at = open + 1 + d.whole.start;
            let ls = text[..at].rfind('\n').map_or(0, |i| i + 1);
            text[ls..at].to_string()
        })
        .filter(|i| i.trim().is_empty())
        .unwrap_or_else(|| "  ".to_string());
    let before = text[..close].trim_end();
    let mut out = String::with_capacity(text.len() + value.len() + 32);
    out.push_str(before);
    if !before.ends_with(';') && !before.ends_with('{') {
        out.push(';');
    }
    let decl = format!(
        "{}: {}{};",
        edit.property,
        value,
        if edit.important { " !important" } else { "" }
    );
    if body.contains('\n') {
        out.push('\n');
        out.push_str(&indent);
        out.push_str(&decl);
        out.push('\n');
        let ls = text[..start].rfind('\n').map_or(0, |i| i + 1);
        out.push_str(&text[ls..start]);
    } else {
        out.push(' ');
        out.push_str(&decl);
        out.push(' ');
    }
    out.push_str(&text[close..]);
    out
}

/// `s` with runs of whitespace as one space, trimmed.
fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The byte offset of 1-based `line`, `column` (characters).
fn offset(text: &str, line: u32, column: u32) -> Option<usize> {
    let mut at = 0;
    for _ in 1..line {
        at += text[at..].find('\n')? + 1;
    }
    let col = text[at..]
        .char_indices()
        .nth(column.saturating_sub(1) as usize)?
        .0;
    Some(at + col)
}

/// The byte offset of the `}` closing the block opened at `open`.
fn block_end(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut chars = text[open..].char_indices().peekable();
    let mut quote: Option<char> = None;
    while let Some((i, c)) = chars.next() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), '\\') => {
                chars.next();
            }
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '/') if chars.peek().map(|p| p.1) == Some('*') => {
                let rest = &text[open + i..];
                let end = rest.find("*/")?;
                while chars.peek().is_some_and(|p| p.0 < i + end + 2) {
                    chars.next();
                }
            }
            (None, '{') => depth += 1,
            (None, '}') => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// One top-level declaration of a block body: byte ranges of its name,
/// its value (after the `:`, before `;` / the end) and the whole.
#[derive(Debug, Clone)]
struct Decl {
    name: std::ops::Range<usize>,
    value: std::ops::Range<usize>,
    whole: std::ops::Range<usize>,
}

/// The declarations at the top level of `body` — not inside a nested rule.
fn top_level_declarations(body: &str) -> Vec<Decl> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut seg_start = 0usize;
    let mut quote: Option<char> = None;
    let bytes: Vec<(usize, char)> = body.char_indices().collect();
    let mut k = 0;
    let push = |from: usize, to: usize, out: &mut Vec<Decl>| {
        let seg = &body[from..to];
        let Some(colon) = seg.find(':') else {
            return;
        };
        let name = seg[..colon].trim();
        if name.is_empty()
            || name.contains('{')
            || !name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return;
        }
        let lead = seg.len() - seg.trim_start().len();
        let value_end = seg.trim_end().len();
        out.push(Decl {
            name: from + lead..from + colon,
            value: from + colon + 1..from + value_end,
            whole: from + lead..from + value_end,
        });
    };
    while k < bytes.len() {
        let (i, c) = bytes[k];
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), '\\') => k += 1,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '/') if body[i..].starts_with("/*") => {
                if let Some(end) = body[i..].find("*/") {
                    while k < bytes.len() && bytes[k].0 < i + end + 2 {
                        k += 1;
                    }
                    // A comment before a declaration (only space between
                    // them) is not part of its name.
                    if depth == 0 && body[seg_start..i].trim().is_empty() {
                        seg_start = i + end + 2;
                    }
                    continue;
                }
            }
            (None, '{') => depth += 1,
            (None, '}') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    seg_start = i + 1;
                }
            }
            (None, ';') if depth == 0 => {
                push(seg_start, i, &mut out);
                seg_start = i + 1;
            }
            _ => {}
        }
        k += 1;
    }
    if depth == 0 && seg_start < body.len() {
        push(seg_start, body.len(), &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{DeclEdit, apply};

    fn set(p: &str, v: &str) -> DeclEdit {
        DeclEdit {
            property: p.into(),
            value: Some(v.into()),
            important: false,
        }
    }

    #[test]
    fn a_value_is_replaced_in_place() {
        let css = "a { color: red; }\n\n.panel {\n  color: blue;\n  padding: 1;\n}\n";
        let out = apply(css, 3, 1, ".panel", &set("color", "#123456")).unwrap();
        assert_eq!(
            out,
            "a { color: red; }\n\n.panel {\n  color: #123456;\n  padding: 1;\n}\n"
        );
    }

    #[test]
    fn a_missing_declaration_is_appended_with_the_blocks_indent() {
        let css = ".panel {\n  color: blue;\n}\n";
        let out = apply(css, 1, 1, ".panel", &set("padding-top", "2")).unwrap();
        assert_eq!(out, ".panel {\n  color: blue;\n  padding-top: 2;\n}\n");
        let one = apply("a { color: red }", 1, 1, "a", &set("margin-top", "1")).unwrap();
        assert_eq!(one, "a { color: red; margin-top: 1; }");
    }

    #[test]
    fn a_declaration_is_removed_with_its_line() {
        let css = ".p {\n  color: blue;\n  padding: 1;\n}\n";
        let edit = DeclEdit {
            property: "color".into(),
            value: None,
            important: false,
        };
        assert_eq!(
            apply(css, 1, 1, ".p", &edit).unwrap(),
            ".p {\n  padding: 1;\n}\n"
        );
    }

    #[test]
    fn nested_rules_strings_and_comments_are_not_declarations_of_the_block() {
        let css = ".c {\n  /* color: no; */\n  content: \"a;b\";\n  & .t { color: red; }\n  color: blue;\n}\n";
        let out = apply(css, 1, 1, ".c", &set("color", "green")).unwrap();
        assert!(out.contains("& .t { color: red; }"), "{out}");
        assert!(out.contains("  color: green;\n}"), "{out}");
        assert!(out.contains("content: \"a;b\""), "{out}");
        assert_eq!(
            out.matches("color: green").count(),
            1,
            "replaced, not appended: {out}"
        );
    }

    fn remove(p: &str) -> DeclEdit {
        DeclEdit {
            property: p.into(),
            value: None,
            important: false,
        }
    }

    #[test]
    fn a_comment_before_a_declaration_does_not_hide_it() {
        let css = ".p {\n  padding: 1;\n  /* the brand color */\n  color: blue;\n}\n";
        let out = apply(css, 1, 1, ".p", &set("color", "red")).unwrap();
        assert_eq!(
            out,
            ".p {\n  padding: 1;\n  /* the brand color */\n  color: red;\n}\n"
        );
        let out = apply(css, 1, 1, ".p", &remove("color")).unwrap();
        assert!(!out.contains("color: blue"), "{out}");
    }

    #[test]
    fn a_longhand_a_later_shorthand_overrides_is_appended() {
        // `margin` after `margin-top` decides it: changing `margin-top` in
        // place would not show; appended after, it does.
        let css = ".p {\n  margin-top: 1;\n  margin: 2;\n}\n";
        let out = apply(css, 1, 1, ".p", &set("margin-top", "3")).unwrap();
        assert_eq!(
            out,
            ".p {\n  margin-top: 1;\n  margin: 2;\n  margin-top: 3;\n}\n"
        );
    }

    #[test]
    fn a_removal_the_text_cannot_express_is_refused_with_why() {
        let css = ".p {\n  margin: 1;\n}\n";
        let err = apply(css, 1, 1, ".p", &remove("margin-top")).unwrap_err();
        assert!(err.contains("margin"), "{err}");
        let err = apply(css, 1, 1, ".p", &remove("color")).unwrap_err();
        assert!(err.contains("declares no color"), "{err}");
    }

    #[test]
    fn every_declaration_of_a_removed_property_goes() {
        let css = ".p {\n  color: red;\n  padding: 1;\n  color: blue;\n}\n";
        assert_eq!(
            apply(css, 1, 1, ".p", &remove("color")).unwrap(),
            ".p {\n  padding: 1;\n}\n"
        );
    }

    #[test]
    fn a_rule_no_longer_at_its_place_is_refused() {
        // The file changed since it was parsed: the rule at 1:1 is not the
        // edited one, so nothing is written there.
        let css = ".other {\n  color: red;\n}\n.p {\n  color: blue;\n}\n";
        let err = apply(css, 1, 1, ".p", &set("color", "green")).unwrap_err();
        assert!(err.contains(".p"), "{err}");
        // An item of a selector list is found in the list.
        let list = "a,\n.p { color: blue; }";
        assert!(apply(list, 1, 1, ".p", &set("color", "green")).is_ok());
    }

    #[test]
    fn important_is_written() {
        let out = apply(
            "a { color: red; }",
            1,
            1,
            "a",
            &DeclEdit {
                property: "color".into(),
                value: Some("blue".into()),
                important: true,
            },
        )
        .unwrap();
        assert_eq!(out, "a { color: blue !important; }");
    }
}
