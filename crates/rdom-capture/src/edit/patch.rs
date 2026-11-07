//! Writing an edit back to the CSS text it came from
//! (`specs/CAPTURE.md` §Edits, write-back): find the rule where its
//! location says, find the property's declaration at the top level of its
//! block, and replace its value — or append the declaration to the block
//! when the rule did not declare it (or declared it through a shorthand,
//! which a later longhand overrides), or drop it for a removal.

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
pub(crate) fn apply(text: &str, line: u32, column: u32, edit: &DeclEdit) -> Result<String, String> {
    let start =
        offset(text, line, column).ok_or_else(|| format!("{line}:{column} is past the text"))?;
    let open = text[start..]
        .find('{')
        .map(|i| start + i)
        .ok_or_else(|| format!("no rule block after {line}:{column}"))?;
    let close = block_end(text, open)
        .ok_or_else(|| format!("the block at {line}:{column} is not closed"))?;
    let body = &text[open + 1..close];
    let decls = top_level_declarations(body);
    let wanted = edit.property.to_ascii_lowercase();
    let found = decls
        .iter()
        .rev()
        .find(|d| body[d.name.clone()].trim().eq_ignore_ascii_case(&wanted));
    let mut out = String::with_capacity(text.len() + 32);
    match (found, &edit.value) {
        (Some(d), Some(v)) => {
            let at = open + 1;
            out.push_str(&text[..at + d.value.start]);
            out.push(' ');
            out.push_str(v);
            if edit.important {
                out.push_str(" !important");
            }
            out.push_str(&text[at + d.value.end..]);
        }
        (Some(d), None) => {
            let at = open + 1;
            // The whole declaration, with its `;` and the line it sat on
            // when it was alone there.
            let (mut from, mut to) = (at + d.whole.start, at + d.whole.end);
            if text[to..].starts_with(';') {
                to += 1;
            }
            let line_start = text[..from].rfind('\n').map_or(0, |i| i + 1);
            if text[line_start..from].trim().is_empty() && text[to..].starts_with('\n') {
                from = line_start;
                to += 1;
            }
            out.push_str(&text[..from]);
            out.push_str(&text[to..]);
        }
        (None, Some(v)) => {
            // Appended before the block's `}`, indented as the block's
            // last declaration (or two spaces in from the rule).
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
            let multiline = body.contains('\n');
            out.push_str(before);
            if !before.ends_with(';') && !before.ends_with('{') {
                out.push(';');
            }
            let decl = format!(
                "{}: {}{};",
                edit.property,
                v,
                if edit.important { " !important" } else { "" }
            );
            if multiline {
                out.push('\n');
                out.push_str(&indent);
                out.push_str(&decl);
                out.push('\n');
                let rule_indent = {
                    let ls = text[..start].rfind('\n').map_or(0, |i| i + 1);
                    text[ls..start].to_string()
                };
                out.push_str(&rule_indent);
            } else {
                out.push(' ');
                out.push_str(&decl);
                out.push(' ');
            }
            out.push_str(&text[close..]);
        }
        (None, None) => return Ok(text.to_string()),
    }
    Ok(out)
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
                    if depth == 0 && seg_start == i {
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
        let out = apply(css, 3, 1, &set("color", "#123456")).unwrap();
        assert_eq!(
            out,
            "a { color: red; }\n\n.panel {\n  color: #123456;\n  padding: 1;\n}\n"
        );
    }

    #[test]
    fn a_missing_declaration_is_appended_with_the_blocks_indent() {
        let css = ".panel {\n  color: blue;\n}\n";
        let out = apply(css, 1, 1, &set("padding-top", "2")).unwrap();
        assert_eq!(out, ".panel {\n  color: blue;\n  padding-top: 2;\n}\n");
        let one = apply("a { color: red }", 1, 1, &set("margin-top", "1")).unwrap();
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
        assert_eq!(apply(css, 1, 1, &edit).unwrap(), ".p {\n  padding: 1;\n}\n");
    }

    #[test]
    fn nested_rules_strings_and_comments_are_not_declarations_of_the_block() {
        let css = ".c {\n  /* color: no; */\n  content: \"a;b\";\n  & .t { color: red; }\n  color: blue;\n}\n";
        let out = apply(css, 1, 1, &set("color", "green")).unwrap();
        assert!(out.contains("& .t { color: red; }"), "{out}");
        assert!(out.contains("  color: green;\n}"), "{out}");
        assert!(out.contains("content: \"a;b\""), "{out}");
    }

    #[test]
    fn important_is_written() {
        let out = apply(
            "a { color: red; }",
            1,
            1,
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
