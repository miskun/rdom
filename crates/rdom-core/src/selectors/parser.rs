//! The selector parser: text → [`SelectorList`].

use super::{
    AttrOp, Combinator, ComplexSelector, CompoundSelector, ParseError, PseudoClass, SelectorList,
    SimpleSelector,
};
use crate::css_syntax;

// ─── Parser ──────────────────────────────────────────────────────────

/// Parse `input` into a selector list. Returns `ParseError` on malformed
/// input. Whitespace-tolerant; comments are not supported.
pub fn parse(input: &str) -> Result<SelectorList, ParseError> {
    let mut p = Parser::new(input, None);
    let list = p.parse_selector_list()?;
    p.finish()?;
    Ok(list)
}

pub(super) struct Parser<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    /// The selector list `&` stands for (CSS Nesting 1 §2), when the
    /// text is a nested rule's selector; `&` is an error otherwise.
    nest: Option<&'a SelectorList>,
    /// Whether the complex selector being parsed used `&` (anywhere,
    /// pseudo-class arguments included).
    pub(super) nest_seen: bool,
}

impl<'a> Parser<'a> {
    pub(super) fn new(src: &'a str, nest: Option<&'a SelectorList>) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            nest,
            nest_seen: false,
        }
    }

    /// Only whitespace may follow the parsed list.
    pub(super) fn finish(&mut self) -> Result<(), ParseError> {
        self.skip_ws();
        if !self.eof() {
            return Err(self.err(format!(
                "unexpected trailing input: `{}`",
                &self.src[self.pos..]
            )));
        }
        Ok(())
    }

    pub(super) fn eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    pub(super) fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    pub(super) fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    pub(super) fn err(&self, msg: String) -> ParseError {
        ParseError { msg, pos: self.pos }
    }

    fn expect(&mut self, want: u8, ctx: &str) -> Result<(), ParseError> {
        match self.peek() {
            Some(b) if b == want => {
                self.pos += 1;
                Ok(())
            }
            Some(b) => Err(self.err(format!(
                "expected `{}` in {ctx}, got `{}`",
                want as char, b as char
            ))),
            None => Err(self.err(format!("expected `{}` in {ctx}, got EOF", want as char))),
        }
    }

    fn parse_selector_list(&mut self) -> Result<SelectorList, ParseError> {
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.eof() {
                break;
            }
            let sel = self.parse_complex_selector()?;
            items.push(sel);
            self.skip_ws();
            if self.peek() == Some(b',') {
                self.pos += 1;
                continue;
            }
            break;
        }
        if items.is_empty() {
            return Err(self.err("empty selector".to_string()));
        }
        Ok(SelectorList(items))
    }

    pub(super) fn bump(&mut self) {
        self.pos += 1;
    }

    pub(super) fn parse_complex_selector(&mut self) -> Result<ComplexSelector, ParseError> {
        // Parse left-to-right into (compound, combinator_to_next) pairs,
        // then rotate into subject + ancestors (subject = rightmost).
        let first = self.parse_compound()?;
        let mut chain: Vec<(CompoundSelector, Combinator)> = Vec::new();
        let mut pending = first;

        loop {
            let had_ws = self.skip_ws_noting();
            let Some(b) = self.peek() else {
                // End of input — finalize with pending compound.
                return Ok(self.finalize(pending, chain));
            };
            let combinator = match b {
                b',' | b')' => {
                    return Ok(self.finalize(pending, chain));
                }
                b'>' => {
                    self.pos += 1;
                    self.skip_ws();
                    Combinator::Child
                }
                b'+' => {
                    self.pos += 1;
                    self.skip_ws();
                    Combinator::AdjacentSibling
                }
                b'~' => {
                    self.pos += 1;
                    self.skip_ws();
                    Combinator::GeneralSibling
                }
                _ if had_ws => Combinator::Descendant,
                _ => {
                    return Ok(self.finalize(pending, chain));
                }
            };
            let next = self.parse_compound()?;
            chain.push((pending, combinator));
            pending = next;
        }
    }

    /// Assemble subject + right-to-left ancestor list.
    fn finalize(
        &self,
        subject: CompoundSelector,
        chain: Vec<(CompoundSelector, Combinator)>,
    ) -> ComplexSelector {
        // chain is [(c0, comb0_to_c1), (c1, comb1_to_c2), …, (c_{n-1}, comb_to_subject)]
        // We want ancestors ordered from closest-to-subject → outward,
        // each tagged with the combinator that connects it to the *next*
        // compound on the right.
        let mut ancestors: Vec<(Combinator, CompoundSelector)> =
            chain.into_iter().map(|(c, comb)| (comb, c)).collect();
        ancestors.reverse();
        ComplexSelector { subject, ancestors }
    }

    /// Skip whitespace and return true if any was consumed. Used to
    /// distinguish descendant combinators from compound boundaries.
    fn skip_ws_noting(&mut self) -> bool {
        let start = self.pos;
        self.skip_ws();
        self.pos > start
    }

    fn parse_compound(&mut self) -> Result<CompoundSelector, ParseError> {
        let mut simples = Vec::new();
        // Optional type / universal at the head.
        match self.peek() {
            Some(b'*') => {
                self.pos += 1;
                simples.push(SimpleSelector::Universal);
            }
            Some(b) if is_ident_start(b) || self.at_valid_escape() => {
                let name = self.parse_ident();
                simples.push(SimpleSelector::Type(name));
            }
            _ => {}
        }
        loop {
            match self.peek() {
                Some(b'&') => {
                    simples.push(self.parse_nesting_selector()?);
                }
                Some(b'#') => {
                    self.pos += 1;
                    let id = self.parse_ident();
                    if id.is_empty() {
                        return Err(self.err("empty id selector".to_string()));
                    }
                    simples.push(SimpleSelector::Id(id));
                }
                Some(b'.') => {
                    self.pos += 1;
                    let cls = self.parse_ident();
                    if cls.is_empty() {
                        return Err(self.err("empty class selector".to_string()));
                    }
                    simples.push(SimpleSelector::Class(cls));
                }
                Some(b'[') => {
                    simples.push(self.parse_attribute()?);
                }
                Some(b':') => {
                    simples.push(self.parse_pseudo()?);
                }
                _ => break,
            }
        }
        if simples.is_empty() {
            return Err(self.err("expected selector".to_string()));
        }
        Ok(CompoundSelector { simples })
    }

    /// `&` (CSS Nesting 1 §2): the parent rule's selector list, as
    /// `:is()` — matching any of its items, with the specificity of
    /// the most specific.
    fn parse_nesting_selector(&mut self) -> Result<SimpleSelector, ParseError> {
        let Some(parent) = self.nest else {
            return Err(self.err("`&` outside a nested style rule".to_string()));
        };
        self.pos += 1;
        self.nest_seen = true;
        Ok(SimpleSelector::Is(Box::new(parent.clone())))
    }

    fn parse_attribute(&mut self) -> Result<SimpleSelector, ParseError> {
        self.expect(b'[', "attribute selector")?;
        self.skip_ws();
        let name = self.parse_ident();
        if name.is_empty() {
            return Err(self.err("expected attribute name".to_string()));
        }
        self.skip_ws();

        let op = match self.peek() {
            Some(b']') => None,
            Some(b'=') => {
                self.pos += 1;
                Some(AttrOp::Exact)
            }
            Some(b'~') if self.bytes.get(self.pos + 1) == Some(&b'=') => {
                self.pos += 2;
                Some(AttrOp::Includes)
            }
            Some(b'|') if self.bytes.get(self.pos + 1) == Some(&b'=') => {
                self.pos += 2;
                Some(AttrOp::DashMatch)
            }
            Some(b'^') if self.bytes.get(self.pos + 1) == Some(&b'=') => {
                self.pos += 2;
                Some(AttrOp::Prefix)
            }
            Some(b'$') if self.bytes.get(self.pos + 1) == Some(&b'=') => {
                self.pos += 2;
                Some(AttrOp::Suffix)
            }
            Some(b'*') if self.bytes.get(self.pos + 1) == Some(&b'=') => {
                self.pos += 2;
                Some(AttrOp::Substring)
            }
            Some(b) => {
                return Err(self.err(format!("unexpected `{}` in attribute selector", b as char)));
            }
            None => return Err(self.err("unexpected EOF in attribute selector".to_string())),
        };

        let value = if op.is_some() {
            self.skip_ws();
            Some(self.parse_attr_value()?)
        } else {
            None
        };
        self.skip_ws();
        self.expect(b']', "attribute selector")?;
        Ok(SimpleSelector::Attribute { name, op, value })
    }

    fn parse_attr_value(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            Some(q @ (b'"' | b'\'')) => {
                self.pos += 1;
                let Some((value, used)) =
                    css_syntax::consume_string(&self.src[self.pos..], q as char)
                else {
                    self.pos = self.bytes.len();
                    return Err(self.err("unterminated quoted attribute value".to_string()));
                };
                self.pos += used;
                Ok(value)
            }
            _ => {
                let id = self.parse_ident();
                if id.is_empty() {
                    return Err(self.err("expected attribute value".to_string()));
                }
                Ok(id)
            }
        }
    }

    fn parse_pseudo(&mut self) -> Result<SimpleSelector, ParseError> {
        self.expect(b':', "pseudo-class")?;
        // Reject pseudo-elements (`::before` etc.) — reserved.
        if self.peek() == Some(b':') {
            return Err(self.err("pseudo-elements are not supported yet".to_string()));
        }
        let name = self.parse_ident();
        if name.is_empty() {
            return Err(self.err("expected pseudo-class name".to_string()));
        }
        // Pseudo-class names are ASCII case-insensitive (Selectors 4
        // §3.1, CSS Values 4 §2.1).
        match name.to_ascii_lowercase().as_str() {
            "not" => {
                self.expect(b'(', ":not")?;
                self.skip_ws();
                let inner = self.parse_selector_list()?;
                self.skip_ws();
                self.expect(b')', ":not")?;
                Ok(SimpleSelector::Not(Box::new(inner)))
            }
            "where" => {
                self.expect(b'(', ":where")?;
                self.skip_ws();
                let inner = self.parse_selector_list()?;
                self.skip_ws();
                self.expect(b')', ":where")?;
                Ok(SimpleSelector::Where(Box::new(inner)))
            }
            "first-child" => Ok(SimpleSelector::Pseudo(PseudoClass::FirstChild)),
            "last-child" => Ok(SimpleSelector::Pseudo(PseudoClass::LastChild)),
            "only-child" => Ok(SimpleSelector::Pseudo(PseudoClass::OnlyChild)),
            "empty" => Ok(SimpleSelector::Pseudo(PseudoClass::Empty)),
            "root" => Ok(SimpleSelector::Pseudo(PseudoClass::Root)),
            "hover" => Ok(SimpleSelector::Pseudo(PseudoClass::Hover)),
            "active" => Ok(SimpleSelector::Pseudo(PseudoClass::Active)),
            "focus" => Ok(SimpleSelector::Pseudo(PseudoClass::Focus)),
            "focus-within" => Ok(SimpleSelector::Pseudo(PseudoClass::FocusWithin)),
            "focus-visible" => Ok(SimpleSelector::Pseudo(PseudoClass::FocusVisible)),
            "checked" => Ok(SimpleSelector::Pseudo(PseudoClass::Checked)),
            "placeholder-shown" => Ok(SimpleSelector::Pseudo(PseudoClass::PlaceholderShown)),
            "indeterminate" => Ok(SimpleSelector::Pseudo(PseudoClass::Indeterminate)),
            "open" => Ok(SimpleSelector::Pseudo(PseudoClass::Open)),
            "disabled" => Ok(SimpleSelector::Pseudo(PseudoClass::Disabled)),
            "enabled" => Ok(SimpleSelector::Pseudo(PseudoClass::Enabled)),
            "valid" => Ok(SimpleSelector::Pseudo(PseudoClass::Valid)),
            "invalid" => Ok(SimpleSelector::Pseudo(PseudoClass::Invalid)),
            "required" => Ok(SimpleSelector::Pseudo(PseudoClass::Required)),
            "optional" => Ok(SimpleSelector::Pseudo(PseudoClass::Optional)),
            other => Err(self.err(format!("unsupported pseudo-class `:{other}`"))),
        }
    }

    /// §4.3.11 "consume an ident sequence", escapes decoded (§4.3.7).
    /// The selector grammar is lenient about the start (`.10` is class
    /// `10`), so this does not check §4.3.9.
    fn parse_ident(&mut self) -> String {
        let (name, used) = css_syntax::consume_ident(&self.src[self.pos..]);
        self.pos += used;
        name
    }

    /// `\` not followed by a newline (§4.3.8) — an escape that starts
    /// an identifier.
    fn at_valid_escape(&self) -> bool {
        css_syntax::is_valid_escape(&self.src[self.pos..])
    }
}

/// CSS Syntax 3 §4.2 ident-start code point, byte-wise: a letter, `_`,
/// or any non-ASCII code point (every byte of a multi-byte UTF-8
/// sequence is ≥ 0x80, so a byte scan stays on `char` boundaries). `-`
/// is accepted too, for `-foo` / custom idents.
fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'-' || !b.is_ascii()
}
