//! `@property` (CSS Properties and Values API 1 §3).
//!
//! `@property --name { syntax: '<color>'; inherits: false;
//! initial-value: red }` registers a custom property in the sheet
//! (`Stylesheet::register_property`). `syntax` and `inherits` are
//! required, `initial-value` too unless the syntax is `*`; unknown
//! descriptors are ignored. An invalid rule registers nothing and is
//! reported with `WarningKind::InvalidPropertyRule`.

use rdom_style::parse::Cursor;
use rdom_style::parse::token::{Token, tokenize};
use rdom_style::parse::values::render_value;
use rdom_style::{PropertyRegistration, Stylesheet};

use crate::layer::read_prelude;
use crate::top_level::{read_string_into, skip_comment_into};
use crate::{Warning, WarningKind};

/// Consume an `@property` rule; the cursor is just past the
/// at-keyword, `at` is the position of `@`.
pub(crate) fn consume_property_rule(
    cursor: &mut Cursor,
    sheet: &mut Stylesheet,
    warnings: &mut Vec<Warning>,
    at: (u32, u32),
) {
    let Some(prelude) = read_prelude(cursor, warnings) else {
        return;
    };
    let name = prelude.trim().to_string();
    let body = match cursor.peek() {
        Some('{') => {
            cursor.bump();
            read_body(cursor)
        }
        _ => {
            if cursor.peek() == Some(';') {
                cursor.bump();
            }
            None
        }
    };
    let registration = body
        .ok_or_else(|| "an `@property` rule needs a block".to_string())
        .and_then(|body| registration(&name, &body));
    match registration {
        Ok(registration) => sheet.register_property(registration),
        Err(reason) => warnings.push(Warning {
            kind: WarningKind::InvalidPropertyRule { name, reason },
            line: at.0,
            column: at.1,
        }),
    }
}

/// The registration a descriptor block describes.
fn registration(name: &str, body: &str) -> Result<PropertyRegistration, String> {
    let tokens = tokenize(body).map_err(|_| "unterminated string or comment".to_string())?;
    let mut syntax = None;
    let mut inherits = None;
    let mut initial = None;
    for decl in tokens.split(|t| *t == Token::Semicolon) {
        let [Token::Ident(descriptor), Token::Colon, value @ ..] = decl else {
            continue;
        };
        match descriptor.to_ascii_lowercase().as_str() {
            "syntax" => match value {
                [Token::String(s)] => syntax = Some(s.clone()),
                _ => return Err("`syntax` must be a string".to_string()),
            },
            "inherits" => match value {
                [Token::Ident(v)] if v.eq_ignore_ascii_case("true") => inherits = Some(true),
                [Token::Ident(v)] if v.eq_ignore_ascii_case("false") => inherits = Some(false),
                _ => return Err("`inherits` must be `true` or `false`".to_string()),
            },
            "initial-value" => initial = Some(render_value(value)),
            _ => {}
        }
    }
    let syntax = syntax.ok_or("the `syntax` descriptor is required")?;
    let inherits = inherits.ok_or("the `inherits` descriptor is required")?;
    PropertyRegistration::new(name, &syntax, inherits, initial.as_deref())
        .map_err(|e| e.to_string())
}

/// From just inside `{`, the block's text through its matching `}`
/// (consumed; EOF closes it).
fn read_body(cursor: &mut Cursor) -> Option<String> {
    let mut out = String::new();
    let mut depth = 0usize;
    loop {
        match cursor.peek() {
            None => return Some(out),
            Some('}') if depth == 0 => {
                cursor.bump();
                return Some(out);
            }
            Some(q @ ('"' | '\'')) => {
                out.push(q);
                cursor.bump();
                if !read_string_into(cursor, q, &mut out) {
                    return Some(out);
                }
            }
            Some('/') if matches!(cursor.peek_two(), (_, Some('*'))) => {
                out.push_str("/*");
                cursor.bump();
                cursor.bump();
                if !skip_comment_into(cursor, &mut out) {
                    return None;
                }
            }
            Some(c) => {
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                out.push(c);
                cursor.bump();
            }
        }
    }
}
