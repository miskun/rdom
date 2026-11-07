//! Why an element's style is what it is (`specs/CAPTURE.md` §Derivation
//! chain): every declaration the cascade applied to it, in the order it
//! applied them — the last one for a property is the winner, the ones
//! before it are what it overrode — each with where it came from: its
//! origin, its rule's selector and specificity, and where the rule is
//! written.
//!
//! Declarations are listed per longhand (a `padding` shorthand is its four
//! sides, each a declaration) and per custom property (`--accent`), with
//! the value as the cascade applied it: a `var()` substituted. A
//! property's value is the winner's unless it has none, in which case it
//! inherits (an inherited property, [`inherits`]) or takes its initial
//! value.
//!
//! The trace covers the element's own box; its pseudo-elements' are not
//! traced yet.

use std::rc::Rc;

use rdom_core::NodeId;
use rdom_style::property_dispatch;

use crate::TuiDom;
use crate::style::cascade::PropertyRegistry;
use crate::style::cascade::explain::{Applied, Origin, explain_element};
use crate::style::{RuleOrigin, SourceLocation, Specificity, Stylesheet, TuiStyle};

/// Where a declaration came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeclarationOrigin {
    /// A user-agent rule (the built-in defaults).
    UserAgent,
    /// An author rule.
    Author,
    /// A presentational hint (an HTML attribute, `<ol start>`).
    Hint,
    /// The element's `style` attribute.
    Inline,
}

/// The rule a declaration is in.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RuleRef {
    /// The rule's sheet: its index in the sheets the cascade ran with.
    pub sheet: usize,
    /// The rule's index in its sheet.
    pub index: usize,
    /// The rule's selector, as written.
    pub selector: String,
    pub specificity: Specificity,
    /// Where the rule is written ([`SourceLocation`]).
    pub location: Option<SourceLocation>,
    /// The name of the CSS text a [`SourceFile::Css`](crate::SourceFile::Css)
    /// location is in: its file or URL.
    pub source_name: Option<String>,
    /// The `<style>` element the rule's sheet came from.
    pub owner_node: Option<NodeId>,
}

/// One declaration the cascade applied.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Declared {
    /// The property: a longhand's canonical name, or a custom property's
    /// (`--accent`).
    pub property: String,
    /// The value as applied (a `var()` substituted).
    pub value: String,
    pub important: bool,
    pub origin: DeclarationOrigin,
    /// The rule, for a declaration in one.
    pub rule: Option<RuleRef>,
}

/// Every declaration the cascade applied to one element, in order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleTrace {
    declarations: Vec<Declared>,
}

impl StyleTrace {
    /// Every declaration, in the order the cascade applied them.
    pub fn declarations(&self) -> &[Declared] {
        &self.declarations
    }

    /// The declaration whose value `property` takes: the last applied.
    pub fn winner(&self, property: &str) -> Option<&Declared> {
        self.declarations
            .iter()
            .rev()
            .find(|d| d.property == property)
    }

    /// The declarations of `property` the winner overrode, latest first.
    pub fn overridden(&self, property: &str) -> Vec<&Declared> {
        let mut all: Vec<&Declared> = self
            .declarations
            .iter()
            .filter(|d| d.property == property)
            .collect();
        all.pop();
        all.reverse();
        all
    }

    /// The properties with a declaration, each once, in the order their
    /// winners applied.
    pub fn properties(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for d in self.declarations.iter().rev() {
            if !out.contains(&d.property.as_str()) {
                out.push(&d.property);
            }
        }
        out.reverse();
        out
    }
}

/// Whether `property` inherits when no declaration sets it (CSS Cascade 4
/// §7.2); custom properties do.
pub fn inherits(property: &str) -> bool {
    property.starts_with("--") || property_dispatch::inherits(property)
}

/// The declarations the cascade applied to element `id`, cascaded under
/// `sheets` (in cascade order); `None` for a node the cascade did not
/// style. [`App::explain_style`](crate::App::explain_style) passes the
/// App's own sheets.
pub fn explain_style(dom: &TuiDom, sheets: &[&Stylesheet], id: NodeId) -> Option<StyleTrace> {
    let registry = Rc::new(PropertyRegistry::new(sheets));
    explain_with(dom, sheets, registry, id)
}

pub(crate) fn explain_with(
    dom: &TuiDom,
    sheets: &[&Stylesheet],
    registry: Rc<PropertyRegistry>,
    id: NodeId,
) -> Option<StyleTrace> {
    let applied = explain_element(dom, sheets, registry, id)?;
    let mut declarations = Vec::new();
    for block in &applied {
        declared_in(sheets, block, &mut declarations);
    }
    Some(StyleTrace { declarations })
}

/// The declarations `block` holds at its step's importance.
fn declared_in(sheets: &[&Stylesheet], block: &Applied<'_>, out: &mut Vec<Declared>) {
    let (origin, rule) = match block.origin {
        Origin::Hint => (DeclarationOrigin::Hint, None),
        Origin::Inline => (DeclarationOrigin::Inline, None),
        Origin::Rule { sheet, index, rule } => {
            let origin = match rule.origin {
                RuleOrigin::UserAgent => DeclarationOrigin::UserAgent,
                _ => DeclarationOrigin::Author,
            };
            let source_name = rule.location.and_then(|l| match l.file {
                crate::SourceFile::Css(i) => sheets[sheet].source_name(i).map(str::to_string),
                _ => None,
            });
            let r = RuleRef {
                sheet,
                index,
                selector: rule.source_text.clone(),
                specificity: rule.specificity,
                location: rule.location,
                source_name,
                owner_node: sheets[sheet].owner_node(),
            };
            (origin, Some(r))
        }
    };
    let style: &TuiStyle = &block.style;
    let mut push = |property: String, value: String| {
        // A rule's block and its substituted / direction-mapped
        // declarations may both hold a property: it is one declaration,
        // applied where it applied last.
        let same_rule = |d: &Declared| {
            d.property == property
                && d.important == block.important
                && d.origin == origin
                && match (&d.rule, &rule) {
                    (Some(a), Some(b)) => (a.sheet, a.index) == (b.sheet, b.index),
                    (None, None) => true,
                    _ => false,
                }
        };
        if let Some(i) = out.iter().rposition(same_rule) {
            out.remove(i);
        }
        out.push(Declared {
            property,
            value,
            important: block.important,
            origin,
            rule: rule.clone(),
        });
    };
    for c in &style.custom_properties {
        if c.important == block.important {
            push(format!("--{}", c.name), c.value.as_str().to_string());
        }
    }
    for &name in property_dispatch::longhand_names() {
        if property_dispatch::is_important(name, style) != block.important {
            continue;
        }
        if let Some(value) = property_dispatch::serialize(name, style) {
            push(name.to_string(), value);
        }
    }
}
