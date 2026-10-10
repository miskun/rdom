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

/// The pseudo-classes that are a keyword alone (Selectors 4), by their
/// ASCII-lowercase name: the table the parser reads them from, so it lists
/// every [`PseudoClass`] the parser makes but `:dir()`'s
/// ([`super::pseudo_class_names`]).
pub(super) const KEYWORD_PSEUDO_CLASSES: &[(&str, PseudoClass)] = &[
    ("first-of-type", PseudoClass::FirstOfType),
    ("last-of-type", PseudoClass::LastOfType),
    ("only-of-type", PseudoClass::OnlyOfType),
    ("any-link", PseudoClass::AnyLink),
    ("link", PseudoClass::Link),
    ("visited", PseudoClass::Visited),
    ("first-child", PseudoClass::FirstChild),
    ("last-child", PseudoClass::LastChild),
    ("only-child", PseudoClass::OnlyChild),
    ("empty", PseudoClass::Empty),
    ("root", PseudoClass::Root),
    ("hover", PseudoClass::Hover),
    ("active", PseudoClass::Active),
    ("focus", PseudoClass::Focus),
    ("focus-within", PseudoClass::FocusWithin),
    ("focus-visible", PseudoClass::FocusVisible),
    ("checked", PseudoClass::Checked),
    ("placeholder-shown", PseudoClass::PlaceholderShown),
    ("indeterminate", PseudoClass::Indeterminate),
    ("open", PseudoClass::Open),
    ("disabled", PseudoClass::Disabled),
    ("enabled", PseudoClass::Enabled),
    ("valid", PseudoClass::Valid),
    ("invalid", PseudoClass::Invalid),
    ("required", PseudoClass::Required),
    ("optional", PseudoClass::Optional),
    ("read-write", PseudoClass::ReadWrite),
    ("read-only", PseudoClass::ReadOnly),
    ("default", PseudoClass::Default),
    ("in-range", PseudoClass::InRange),
    ("out-of-range", PseudoClass::OutOfRange),
    ("user-valid", PseudoClass::UserValid),
    ("user-invalid", PseudoClass::UserInvalid),
    ("modal", PseudoClass::Modal),
    ("popover-open", PseudoClass::PopoverOpen),
    ("scope", PseudoClass::Scope),
];

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
                let inner = self.descend(Self::parse_selector_list)?;
                self.skip_ws();
                self.expect(b')', ":not")?;
                Ok(SimpleSelector::Not(Box::new(inner)))
            }
            "is" => {
                self.expect(b'(', ":is")?;
                let inner = self.descend(Self::parse_forgiving_list)?;
                self.expect(b')', ":is")?;
                Ok(SimpleSelector::Is(Box::new(inner)))
            }
            "where" => {
                self.expect(b'(', ":where")?;
                self.skip_ws();
                let inner = self.descend(Self::parse_selector_list)?;
                self.skip_ws();
                self.expect(b')', ":where")?;
                Ok(SimpleSelector::Where(Box::new(inner)))
            }
            "nth-child" => self.parse_nth(NthKind::Child),
            "nth-last-child" => self.parse_nth(NthKind::LastChild),
            "nth-of-type" => self.parse_nth(NthKind::OfType),
            "nth-last-of-type" => self.parse_nth(NthKind::LastOfType),
            "nth-col" => self.parse_nth_column(false),
            "nth-last-col" => self.parse_nth_column(true),
            "has" => self.parse_has(),
            "lang" => self.parse_lang(),
            "dir" => self.parse_dir(),
            other => match KEYWORD_PSEUDO_CLASSES.iter().find(|(n, _)| *n == other) {
                Some((_, class)) => {
                    // `:scope` makes a sheet's rule scope-relative.
                    self.scope_seen |= *class == PseudoClass::Scope;
                    Ok(SimpleSelector::Pseudo(*class))
                }
                None => Err(self.err(format!("unsupported pseudo-class `:{other}`"))),
            },
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
            let list = self.descend(Self::parse_selector_list)?;
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

    /// Selectors 4 §16.2 / §16.3: `( <an+b> )`.
    fn parse_nth_column(&mut self, last: bool) -> Result<SimpleSelector, ParseError> {
        let name = if last { ":nth-last-col" } else { ":nth-col" };
        self.expect(b'(', name)?;
        let (anb, used) = parse_anb(&self.src[self.pos..]).map_err(|msg| self.err(msg))?;
        self.pos += used;
        self.skip_ws();
        self.expect(b')', name)?;
        Ok(SimpleSelector::NthColumn(super::NthColumnSelector {
            last,
            a: anb.a,
            b: anb.b,
        }))
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
        let relative = self.descend(Self::parse_relative_list);
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
