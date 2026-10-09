//! Reading a style rule's block at the source level (CSS Syntax 3 §5.4,
//! CSS Nesting 1 §2): whether an item is a declaration or a nested rule,
//! a declaration's text up to its end, a nested rule's prelude, and the
//! rest of a block skipped — strings, escapes, comments and nested blocks
//! kept whole.

use rdom_style::parse::SourceCursor;

use crate::Warning;
use crate::top_level::{copy_escape_into, read_string_into, skip_comment, skip_comment_into};

/// Does the block item at the start of `rest` parse as a declaration?
/// `<ident> <ws>* :`, then — unless the name is a custom property's —
/// no top-level `{` before the `;` or `}` that ends it.
pub(crate) fn is_declaration(rest: &str) -> bool {
    if !rdom_core::css_syntax::would_start_ident(rest) {
        return false;
    }
    let (name, used) = rdom_core::css_syntax::consume_ident(rest);
    let after = rest[used..].trim_start();
    if !after.starts_with(':') {
        return false;
    }
    if name.starts_with("--") {
        return true;
    }
    let mut chars = after[1..].chars().peekable();
    let mut depth = 0usize;
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            q @ ('"' | '\'') => {
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        chars.next();
                    } else if c == q {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut star = false;
                for c in chars.by_ref() {
                    if star && c == '/' {
                        break;
                    }
                    star = c == '*';
                }
            }
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            '{' if depth == 0 => return false,
            ';' | '}' if depth == 0 => return true,
            _ => {}
        }
    }
    true
}

/// Consume one declaration's text through its `;` (consumed) or up to
/// the block's `}` (not consumed) or EOF. Strings, comments, escapes
/// and bracketed groups are passed through whole.
pub(crate) fn read_declaration(cursor: &mut SourceCursor) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    loop {
        match cursor.peek() {
            None => return out,
            Some(';') if depth == 0 => {
                cursor.bump();
                return out;
            }
            Some('}') if depth == 0 => return out,
            Some(q @ ('"' | '\'')) => {
                out.push(q);
                cursor.bump();
                if !read_string_into(cursor, q, &mut out) {
                    return out;
                }
            }
            Some('\\') => copy_escape_into(cursor, &mut out),
            Some('/') if matches!(cursor.peek_two(), (_, Some('*'))) => {
                out.push_str("/*");
                cursor.bump();
                cursor.bump();
                if !skip_comment_into(cursor, &mut out) {
                    return out;
                }
            }
            Some(c) => {
                match c {
                    '(' | '[' | '{' => depth += 1,
                    ')' | ']' | '}' => depth = depth.saturating_sub(1),
                    _ => {}
                }
                out.push(c);
                cursor.bump();
            }
        }
    }
}

/// A style rule's prelude up to (not consuming) `{` — or, nested, up
/// to a `;` / `}` that ends the item first. Comments become a space;
/// strings and escapes are copied through so a `{` in them does not
/// end the prelude. `None` at EOF or on an unterminated comment.
pub(crate) fn read_prelude(
    cursor: &mut SourceCursor,
    warnings: &mut Vec<Warning>,
    nested: bool,
) -> Option<String> {
    let mut out = String::new();
    loop {
        match cursor.peek() {
            None => return None,
            Some('{') => return Some(out),
            Some(';' | '}') if nested => return Some(out),
            Some(q @ ('"' | '\'')) => {
                out.push(q);
                cursor.bump();
                if !read_string_into(cursor, q, &mut out) {
                    return None;
                }
            }
            // An escape (CSS Syntax 3 §4.3.7) is copied through
            // undecoded — the selector parser decodes it — so `.x\{`
            // does not end the prelude.
            Some('\\') => copy_escape_into(cursor, &mut out),
            Some('/') if matches!(cursor.peek_two(), (_, Some('*'))) => {
                if !skip_comment(cursor, warnings) {
                    return None;
                }
                // `a/* */b` is `a b`.
                if !out.ends_with(char::is_whitespace) && !out.is_empty() {
                    out.push(' ');
                }
            }
            Some(c) => {
                out.push(c);
                cursor.bump();
            }
        }
    }
}

/// From just inside a block's `{`, skip through its matching `}`.
pub(crate) fn skip_rest_of_block(cursor: &mut SourceCursor) {
    let mut depth = 1usize;
    loop {
        match cursor.peek() {
            None => return,
            Some('{') => {
                depth += 1;
                cursor.bump();
            }
            Some('}') => {
                cursor.bump();
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
            Some(q @ ('"' | '\'')) => {
                cursor.bump();
                let mut sink = String::new();
                if !read_string_into(cursor, q, &mut sink) {
                    return;
                }
            }
            Some('\\') => {
                let mut sink = String::new();
                copy_escape_into(cursor, &mut sink);
            }
            Some('/') if matches!(cursor.peek_two(), (_, Some('*'))) => {
                let mut sink = String::new();
                cursor.bump();
                cursor.bump();
                if !skip_comment_into(cursor, &mut sink) {
                    return;
                }
            }
            Some(_) => {
                cursor.bump();
            }
        }
    }
}
