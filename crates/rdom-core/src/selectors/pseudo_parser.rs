//! Pseudo-class parsing: the names, and the arguments of the
//! functional ones (`:not()`, `:is()`, `:where()`, the `:nth-*()`
//! family).

use super::anb::parse_anb;
use super::parser::Parser;
use super::{
    Combinator, NthKind, NthSelector, ParseError, PseudoClass, RelativeSelector, SimpleSelector,
};
use crate::Directionality;
use crate::css_syntax;

impl Parser<'_> {
    pub(super) fn parse_pseudo(&mut self) -> Result<SimpleSelector, ParseError> {
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
            "is" => {
                self.expect(b'(', ":is")?;
                let inner = self.parse_forgiving_list()?;
                self.expect(b')', ":is")?;
                Ok(SimpleSelector::Is(Box::new(inner)))
            }
            "where" => {
                self.expect(b'(', ":where")?;
                self.skip_ws();
                let inner = self.parse_selector_list()?;
                self.skip_ws();
                self.expect(b')', ":where")?;
                Ok(SimpleSelector::Where(Box::new(inner)))
            }
            "nth-child" => self.parse_nth(NthKind::Child),
            "nth-last-child" => self.parse_nth(NthKind::LastChild),
            "nth-of-type" => self.parse_nth(NthKind::OfType),
            "nth-last-of-type" => self.parse_nth(NthKind::LastOfType),
            "first-of-type" => Ok(SimpleSelector::Pseudo(PseudoClass::FirstOfType)),
            "last-of-type" => Ok(SimpleSelector::Pseudo(PseudoClass::LastOfType)),
            "only-of-type" => Ok(SimpleSelector::Pseudo(PseudoClass::OnlyOfType)),
            "any-link" => Ok(SimpleSelector::Pseudo(PseudoClass::AnyLink)),
            "link" => Ok(SimpleSelector::Pseudo(PseudoClass::Link)),
            "visited" => Ok(SimpleSelector::Pseudo(PseudoClass::Visited)),
            "has" => self.parse_has(),
            "lang" => self.parse_lang(),
            "dir" => self.parse_dir(),
            "first-child" => Ok(SimpleSelector::Pseudo(PseudoClass::FirstChild)),
            "last-child" => Ok(SimpleSelector::Pseudo(PseudoClass::LastChild)),
            "only-child" => Ok(SimpleSelector::Pseudo(PseudoClass::OnlyChild)),
            "empty" => Ok(SimpleSelector::Pseudo(PseudoClass::Empty)),
            "root" => Ok(SimpleSelector::Pseudo(PseudoClass::Root)),
            "scope" => {
                self.scope_seen = true;
                Ok(SimpleSelector::Pseudo(PseudoClass::Scope))
            }
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
            "read-write" => Ok(SimpleSelector::Pseudo(PseudoClass::ReadWrite)),
            "read-only" => Ok(SimpleSelector::Pseudo(PseudoClass::ReadOnly)),
            "default" => Ok(SimpleSelector::Pseudo(PseudoClass::Default)),
            "in-range" => Ok(SimpleSelector::Pseudo(PseudoClass::InRange)),
            "out-of-range" => Ok(SimpleSelector::Pseudo(PseudoClass::OutOfRange)),
            other => Err(self.err(format!("unsupported pseudo-class `:{other}`"))),
        }
    }

    /// Selectors 4 §13.3.1: `( <an+b> [of <complex-real-selector-list>]? )`,
    /// the `of` clause for the `-child` forms only. `of` is ASCII
    /// case-insensitive and needs white space before its list.
    fn parse_nth(&mut self, kind: NthKind) -> Result<SimpleSelector, ParseError> {
        let name = match kind {
            NthKind::Child => ":nth-child",
            NthKind::LastChild => ":nth-last-child",
            NthKind::OfType => ":nth-of-type",
            NthKind::LastOfType => ":nth-last-of-type",
        };
        self.expect(b'(', name)?;
        let (anb, used) = parse_anb(&self.src[self.pos..]).map_err(|msg| self.err(msg))?;
        self.pos += used;
        self.skip_ws();
        let of = if self.at_of_keyword() {
            if !matches!(kind, NthKind::Child | NthKind::LastChild) {
                return Err(self.err(format!("{name}() takes no `of` selector list")));
            }
            self.pos += 2;
            self.skip_ws();
            let list = self.parse_selector_list()?;
            self.skip_ws();
            Some(list)
        } else {
            None
        };
        self.expect(b')', name)?;
        Ok(SimpleSelector::Nth(Box::new(NthSelector {
            kind,
            a: anb.a,
            b: anb.b,
            of,
        })))
    }

    /// `of` followed by white space at the cursor.
    fn at_of_keyword(&self) -> bool {
        let rest = &self.bytes[self.pos..];
        rest.len() > 2 && rest[..2].eq_ignore_ascii_case(b"of") && rest[2].is_ascii_whitespace()
    }

    /// Selectors 4 §7.2: `:lang( [<ident> | <string>]# )`.
    fn parse_lang(&mut self) -> Result<SimpleSelector, ParseError> {
        self.expect(b'(', ":lang")?;
        let mut ranges = Vec::new();
        loop {
            self.skip_ws();
            let range = match self.peek() {
                Some(q @ (b'"' | b'\'')) => {
                    self.pos += 1;
                    let Some((value, used)) =
                        css_syntax::consume_string(&self.src[self.pos..], q as char)
                    else {
                        return Err(self.err("unterminated :lang() string".to_string()));
                    };
                    self.pos += used;
                    value
                }
                _ if css_syntax::would_start_ident(&self.src[self.pos..]) => self.parse_ident(),
                _ => return Err(self.err(":lang() takes identifiers or strings".to_string())),
            };
            ranges.push(range);
            self.skip_ws();
            if self.peek() == Some(b',') {
                self.pos += 1;
                continue;
            }
            self.expect(b')', ":lang")?;
            return Ok(SimpleSelector::Lang(ranges));
        }
    }

    /// Selectors 4 §7.1: `:dir(<ident>)` — `ltr` / `rtl` (ASCII
    /// case-insensitive); another identifier is valid and matches
    /// nothing.
    fn parse_dir(&mut self) -> Result<SimpleSelector, ParseError> {
        self.expect(b'(', ":dir")?;
        self.skip_ws();
        if !css_syntax::would_start_ident(&self.src[self.pos..]) {
            return Err(self.err(":dir() takes an identifier".to_string()));
        }
        let ident = self.parse_ident().to_ascii_lowercase();
        self.skip_ws();
        self.expect(b')', ":dir")?;
        let dir = match ident.as_str() {
            "ltr" => Some(Directionality::Ltr),
            "rtl" => Some(Directionality::Rtl),
            _ => None,
        };
        Ok(SimpleSelector::Pseudo(PseudoClass::Dir(dir)))
    }

    /// Selectors 4 §4.5: `:has( <relative-selector-list> )` — unforgiving,
    /// and not valid inside another `:has()`, however deep.
    fn parse_has(&mut self) -> Result<SimpleSelector, ParseError> {
        if self.in_has {
            return Err(self.err(":has() is not valid inside :has()".to_string()));
        }
        self.expect(b'(', ":has")?;
        self.in_has = true;
        let relative = self.parse_relative_list();
        self.in_has = false;
        let relative = relative?;
        self.skip_ws();
        self.expect(b')', ":has")?;
        Ok(SimpleSelector::Has(relative))
    }

    /// Selectors 4 §3.4 `<relative-selector-list>`: comma-separated
    /// complex selectors, each after an optional `>`, `+` or `~`.
    fn parse_relative_list(&mut self) -> Result<Vec<RelativeSelector>, ParseError> {
        let mut list = Vec::new();
        loop {
            self.skip_ws();
            let combinator = match self.peek() {
                Some(b'>') => Combinator::Child,
                Some(b'+') => Combinator::AdjacentSibling,
                Some(b'~') => Combinator::GeneralSibling,
                _ => Combinator::Descendant,
            };
            if combinator != Combinator::Descendant {
                self.pos += 1;
                self.skip_ws();
            }
            let selector = self.parse_complex_selector()?;
            list.push(RelativeSelector {
                combinator,
                selector,
            });
            self.skip_ws();
            if self.peek() != Some(b',') {
                return Ok(list);
            }
            self.pos += 1;
        }
    }
}
