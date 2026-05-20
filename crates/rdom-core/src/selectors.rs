//! CSS selector grammar — tokenizer + parser + AST.
//!
//! Supports a practical subset of CSS Level 3:
//!
//! - Simple selectors: `*`, `tag`, `#id`, `.class`, `[attr]`, `[attr=v]`,
//!   `[attr="v"]`, `[attr^=v]`, `[attr$=v]`, `[attr*=v]`, `[attr~=v]`, `[attr|=v]`
//! - Compound: `tag.foo#bar[attr=x]`
//! - Combinators: descendant (space), child `>`, adjacent `+`, general `~`
//! - Pseudo-classes: `:not(selector)`, `:first-child`, `:last-child`,
//!   `:only-child`, `:empty`, `:root`
//! - Selector list: `a, b, c`
//!
//! Not supported yet (reserved for later phases):
//! - `:nth-child(an+b)`, `:has(...)`, `:is(...)`, namespaces, attribute
//!   case flags (`[attr="v" i]`), pseudo-elements (`::before`, `::after`).

use std::fmt;

// ─── AST ─────────────────────────────────────────────────────────────

/// A full selector expression — one or more comma-separated compound
/// selector chains connected by combinators.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectorList(pub Vec<ComplexSelector>);

/// A chain of compound selectors joined by combinators, read left→right.
/// `a > b c` parses as `[Compound(a), Combinator(Child), Compound(b),
/// Combinator(Descendant), Compound(c)]` — stored as a root compound plus
/// a list of `(combinator, compound)` pairs for easier right-to-left
/// matching.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexSelector {
    /// The right-most compound (what we match against the candidate node).
    pub subject: CompoundSelector,
    /// Parents of the subject, each prefaced by the combinator connecting
    /// it to the *next* compound on the right. Ordered right-to-left
    /// for efficient matching (e.g. `a > b c` → [(Descendant, b), (Child, a)]).
    pub ancestors: Vec<(Combinator, CompoundSelector)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// `a b` — b descends from a.
    Descendant,
    /// `a > b` — b is a direct child of a.
    Child,
    /// `a + b` — b is the immediately-following sibling of a.
    AdjacentSibling,
    /// `a ~ b` — b is a following sibling of a.
    GeneralSibling,
}

/// A compound selector: one or more simple selectors that must *all* match.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompoundSelector {
    pub simples: Vec<SimpleSelector>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SimpleSelector {
    /// `*`.
    Universal,
    /// `tag`.
    Type(String),
    /// `#id` → equivalent to `[id=...]` but stored separately for index support.
    Id(String),
    /// `.class`.
    Class(String),
    /// `[attr]` / `[attr op value]`.
    Attribute {
        name: String,
        op: Option<AttrOp>,
        value: Option<String>,
    },
    /// `:not(...)` — the negated selector list.
    Not(Box<SelectorList>),
    /// Structural pseudo-classes.
    Pseudo(PseudoClass),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrOp {
    /// `[attr=value]` — exact match.
    Exact,
    /// `[attr~=value]` — one of a whitespace-separated list.
    Includes,
    /// `[attr|=value]` — exact or starts with `value-`.
    DashMatch,
    /// `[attr^=value]` — prefix.
    Prefix,
    /// `[attr$=value]` — suffix.
    Suffix,
    /// `[attr*=value]` — substring.
    Substring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoClass {
    FirstChild,
    LastChild,
    OnlyChild,
    Empty,
    Root,
    /// `:hover` — matches when the node is the Dom's currently-hovered
    /// node (tracked via `Dom::set_hovered`).
    Hover,
    /// `:focus` — matches when the node is the Dom's currently-focused
    /// node (tracked via `Dom::set_focused`).
    Focus,
    /// `:checked` — matches when the element has a `checked`
    /// attribute (any value, presence-only). The user-toggle
    /// builtins flip this attribute on click / Space, so this
    /// selector reflects current state without needing a separate
    /// IDL property.
    Checked,
    /// `:placeholder-shown` — matches form controls that have a
    /// non-empty `placeholder` attribute AND whose current text
    /// content is empty. Used by UA rules to render the
    /// placeholder via `::before { content: attr(placeholder) }`.
    PlaceholderShown,
    /// `:indeterminate` — matches elements in an indeterminate
    /// state. In this v1 that means `<progress>` without a `value`
    /// attribute; browsers also match indeterminate checkboxes
    /// and orphan radio buttons, which we defer to polish (neither
    /// has a concrete state model in rdom yet).
    Indeterminate,
    /// `:open` — matches elements with the `open` attribute
    /// present. Covers `<details open>` and `<dialog open>` /
    /// `<dialog data-rdom-open>`. Authors use it to style the
    /// expanded state of disclosure widgets via CSS without having
    /// to write attribute selectors themselves.
    Open,
}

// ─── Error ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub msg: String,
    pub pos: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "selector parse error at {}: {}", self.pos, self.msg)
    }
}

impl std::error::Error for ParseError {}

// ─── Parser ──────────────────────────────────────────────────────────

/// Parse `input` into a selector list. Returns `ParseError` on malformed
/// input. Whitespace-tolerant; comments are not supported.
pub fn parse(input: &str) -> Result<SelectorList, ParseError> {
    let mut p = Parser::new(input);
    let list = p.parse_selector_list()?;
    p.skip_ws();
    if !p.eof() {
        return Err(p.err(format!("unexpected trailing input: `{}`", &p.src[p.pos..])));
    }
    Ok(list)
}

struct Parser<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    fn eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn err(&self, msg: String) -> ParseError {
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

    fn parse_complex_selector(&mut self) -> Result<ComplexSelector, ParseError> {
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
            Some(b) if is_ident_start(b) => {
                let name = self.parse_ident();
                simples.push(SimpleSelector::Type(name));
            }
            _ => {}
        }
        loop {
            match self.peek() {
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
            Some(b'"') => {
                self.pos += 1;
                let start = self.pos;
                while let Some(b) = self.peek() {
                    if b == b'"' {
                        break;
                    }
                    self.pos += 1;
                }
                let s = self.src[start..self.pos].to_string();
                self.expect(b'"', "quoted attribute value")?;
                Ok(s)
            }
            Some(b'\'') => {
                self.pos += 1;
                let start = self.pos;
                while let Some(b) = self.peek() {
                    if b == b'\'' {
                        break;
                    }
                    self.pos += 1;
                }
                let s = self.src[start..self.pos].to_string();
                self.expect(b'\'', "quoted attribute value")?;
                Ok(s)
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
        match name.as_str() {
            "not" => {
                self.expect(b'(', ":not")?;
                self.skip_ws();
                let inner = self.parse_selector_list()?;
                self.skip_ws();
                self.expect(b')', ":not")?;
                Ok(SimpleSelector::Not(Box::new(inner)))
            }
            "first-child" => Ok(SimpleSelector::Pseudo(PseudoClass::FirstChild)),
            "last-child" => Ok(SimpleSelector::Pseudo(PseudoClass::LastChild)),
            "only-child" => Ok(SimpleSelector::Pseudo(PseudoClass::OnlyChild)),
            "empty" => Ok(SimpleSelector::Pseudo(PseudoClass::Empty)),
            "root" => Ok(SimpleSelector::Pseudo(PseudoClass::Root)),
            "hover" => Ok(SimpleSelector::Pseudo(PseudoClass::Hover)),
            "focus" => Ok(SimpleSelector::Pseudo(PseudoClass::Focus)),
            "checked" => Ok(SimpleSelector::Pseudo(PseudoClass::Checked)),
            "placeholder-shown" => Ok(SimpleSelector::Pseudo(PseudoClass::PlaceholderShown)),
            "indeterminate" => Ok(SimpleSelector::Pseudo(PseudoClass::Indeterminate)),
            "open" => Ok(SimpleSelector::Pseudo(PseudoClass::Open)),
            other => Err(self.err(format!("unsupported pseudo-class `:{other}`"))),
        }
    }

    fn parse_ident(&mut self) -> String {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if is_ident_continue(b) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.src[start..self.pos].to_string()
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'-'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> SimpleSelector {
        SimpleSelector::Type(v.to_string())
    }

    #[test]
    fn parse_type_selector() {
        let sl = parse("div").unwrap();
        assert_eq!(sl.0.len(), 1);
        assert_eq!(sl.0[0].subject.simples, vec![s("div")]);
        assert!(sl.0[0].ancestors.is_empty());
    }

    #[test]
    fn parse_universal() {
        let sl = parse("*").unwrap();
        assert_eq!(sl.0[0].subject.simples, vec![SimpleSelector::Universal]);
    }

    #[test]
    fn parse_id() {
        let sl = parse("#main").unwrap();
        assert_eq!(
            sl.0[0].subject.simples,
            vec![SimpleSelector::Id("main".into())]
        );
    }

    #[test]
    fn parse_class() {
        let sl = parse(".foo").unwrap();
        assert_eq!(
            sl.0[0].subject.simples,
            vec![SimpleSelector::Class("foo".into())]
        );
    }

    #[test]
    fn parse_compound() {
        let sl = parse("div.foo#bar.baz").unwrap();
        assert_eq!(
            sl.0[0].subject.simples,
            vec![
                s("div"),
                SimpleSelector::Class("foo".into()),
                SimpleSelector::Id("bar".into()),
                SimpleSelector::Class("baz".into()),
            ]
        );
    }

    #[test]
    fn parse_attribute_presence() {
        let sl = parse("[disabled]").unwrap();
        assert_eq!(
            sl.0[0].subject.simples,
            vec![SimpleSelector::Attribute {
                name: "disabled".into(),
                op: None,
                value: None,
            }]
        );
    }

    #[test]
    fn parse_attribute_exact_unquoted() {
        let sl = parse("[role=banner]").unwrap();
        match &sl.0[0].subject.simples[0] {
            SimpleSelector::Attribute { name, op, value } => {
                assert_eq!(name, "role");
                assert_eq!(*op, Some(AttrOp::Exact));
                assert_eq!(value.as_deref(), Some("banner"));
            }
            _ => panic!("expected attribute"),
        }
    }

    #[test]
    fn parse_attribute_exact_quoted() {
        let sl = parse(r#"[title="hello world"]"#).unwrap();
        match &sl.0[0].subject.simples[0] {
            SimpleSelector::Attribute { value, .. } => {
                assert_eq!(value.as_deref(), Some("hello world"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_attribute_operators() {
        for (src, op) in [
            ("[a~=b]", AttrOp::Includes),
            ("[a|=b]", AttrOp::DashMatch),
            ("[a^=b]", AttrOp::Prefix),
            ("[a$=b]", AttrOp::Suffix),
            ("[a*=b]", AttrOp::Substring),
        ] {
            let sl = parse(src).unwrap();
            match &sl.0[0].subject.simples[0] {
                SimpleSelector::Attribute { op: got, .. } => assert_eq!(*got, Some(op)),
                _ => panic!("{src}"),
            }
        }
    }

    #[test]
    fn parse_descendant() {
        let sl = parse("a b").unwrap();
        let complex = &sl.0[0];
        assert_eq!(complex.subject.simples, vec![s("b")]);
        assert_eq!(complex.ancestors.len(), 1);
        assert_eq!(complex.ancestors[0].0, Combinator::Descendant);
        assert_eq!(complex.ancestors[0].1.simples, vec![s("a")]);
    }

    #[test]
    fn parse_child_combinator() {
        let sl = parse("a > b").unwrap();
        let c = &sl.0[0];
        assert_eq!(c.ancestors[0].0, Combinator::Child);
    }

    #[test]
    fn parse_adjacent_and_general_sibling() {
        let sl = parse("a + b").unwrap();
        assert_eq!(sl.0[0].ancestors[0].0, Combinator::AdjacentSibling);
        let sl = parse("a ~ b").unwrap();
        assert_eq!(sl.0[0].ancestors[0].0, Combinator::GeneralSibling);
    }

    #[test]
    fn parse_chain_right_to_left() {
        let sl = parse("a > b c").unwrap();
        let c = &sl.0[0];
        // Subject = c, ancestors (closest→outer) = [(Descendant, b), (Child, a)]
        assert_eq!(c.subject.simples, vec![s("c")]);
        assert_eq!(c.ancestors.len(), 2);
        assert_eq!(c.ancestors[0].0, Combinator::Descendant);
        assert_eq!(c.ancestors[0].1.simples, vec![s("b")]);
        assert_eq!(c.ancestors[1].0, Combinator::Child);
        assert_eq!(c.ancestors[1].1.simples, vec![s("a")]);
    }

    #[test]
    fn parse_selector_list() {
        let sl = parse("a, b, c.foo").unwrap();
        assert_eq!(sl.0.len(), 3);
    }

    #[test]
    fn parse_not_pseudo() {
        let sl = parse("div:not(.foo)").unwrap();
        match &sl.0[0].subject.simples[1] {
            SimpleSelector::Not(inner) => {
                assert_eq!(inner.0.len(), 1);
                assert_eq!(
                    inner.0[0].subject.simples,
                    vec![SimpleSelector::Class("foo".into())]
                );
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_structural_pseudos() {
        for (src, p) in [
            (":first-child", PseudoClass::FirstChild),
            (":last-child", PseudoClass::LastChild),
            (":only-child", PseudoClass::OnlyChild),
            (":empty", PseudoClass::Empty),
            (":root", PseudoClass::Root),
        ] {
            let sl = parse(src).unwrap();
            assert_eq!(sl.0[0].subject.simples, vec![SimpleSelector::Pseudo(p)]);
        }
    }

    #[test]
    fn parse_whitespace_tolerance() {
        parse("  a  ,  b  ").unwrap();
        parse("a   >   b").unwrap();
        parse("[ role = \"x\" ]").unwrap();
    }

    #[test]
    fn parse_errors_on_empty_input() {
        assert!(parse("").is_err());
        assert!(parse("   ").is_err());
    }

    #[test]
    fn parse_errors_on_trailing_garbage() {
        assert!(parse("div ))").is_err());
    }

    #[test]
    fn parse_errors_on_unknown_pseudo() {
        assert!(parse(":banana").is_err());
    }

    #[test]
    fn parse_errors_on_unterminated_attr() {
        assert!(parse("[foo=").is_err());
        assert!(parse("[foo=bar").is_err());
    }

    #[test]
    fn parse_errors_on_pseudo_element() {
        assert!(parse("::before").is_err());
    }
}
