//! What a parse returns: the stylesheet or style, its warnings, and the
//! errors a strict parse turns them into.

use rdom_style::{Stylesheet, TuiStyle};

pub(crate) fn warning_to_error(w: &Warning) -> ParseError {
    let kind = match &w.kind {
        WarningKind::UnterminatedComment => ParseErrorKind::UnterminatedComment,
        WarningKind::UnterminatedString => ParseErrorKind::UnterminatedString,
        WarningKind::InvalidSelector(s) => ParseErrorKind::InvalidSelector(s.clone()),
        WarningKind::UnknownProperty(_)
        | WarningKind::InvalidValue { .. }
        | WarningKind::MalformedDeclaration(_) => {
            ParseErrorKind::ExpectedToken("valid declaration")
        }
        WarningKind::UnsupportedAtRule(_) | WarningKind::ImportIgnored(_) => {
            ParseErrorKind::ExpectedToken("rule")
        }
        WarningKind::ImportCycle(_)
        | WarningKind::ImportFailed { .. }
        | WarningKind::ImportTooDeep(_) => ParseErrorKind::ExpectedToken("importable sheet"),
        WarningKind::BlockTooDeep => ParseErrorKind::ExpectedToken("block within the nesting cap"),
        WarningKind::InvalidAtRulePrelude { .. } | WarningKind::InvalidPropertyRule { .. } => {
            ParseErrorKind::ExpectedToken("at-rule prelude")
        }
        WarningKind::UnmeasuredQueryValue(_) => {
            ParseErrorKind::ExpectedToken("query value with a cell measure")
        }
        WarningKind::InvalidCounterStyleRule { .. } => {
            ParseErrorKind::ExpectedToken("valid counter style")
        }
        WarningKind::CounterStyleDescriptorDropped { .. } => {
            ParseErrorKind::ExpectedToken("valid counter style descriptor")
        }
        WarningKind::InvalidKeyframeSelector(_) => {
            ParseErrorKind::ExpectedToken("keyframe selector")
        }
        WarningKind::ImportantInKeyframe(_) => {
            ParseErrorKind::ExpectedToken("keyframe declaration without !important")
        }
        WarningKind::PositionTryDescriptorDropped { .. } => {
            ParseErrorKind::ExpectedToken("@position-try descriptor")
        }
    };
    ParseError {
        kind,
        line: w.line,
        column: w.column,
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ParseResult {
    pub stylesheet: Stylesheet,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct InlineParseResult {
    pub style: TuiStyle,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    UnexpectedEof,
    UnterminatedComment,
    UnterminatedString,
    InvalidSelector(String),
    ExpectedToken(&'static str),
}

impl std::fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEof => f.write_str("unexpected end of input"),
            Self::UnterminatedComment => f.write_str("unterminated comment"),
            Self::UnterminatedString => f.write_str("unterminated string"),
            Self::InvalidSelector(s) => write!(f, "invalid selector `{s}`"),
            Self::ExpectedToken(what) => write!(f, "expected {what}"),
        }
    }
}

/// `line:column: problem`, both 1-based.
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.kind)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Warning {
    pub kind: WarningKind,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WarningKind {
    UnknownProperty(String),
    InvalidValue {
        property: String,
        value: String,
    },
    /// A declaration segment that is not `name : value` (missing colon,
    /// missing name, stray tokens). Dropped per CSS Syntax 3 §5.4.4;
    /// the payload is the segment's rendered text.
    MalformedDeclaration(String),
    UnsupportedAtRule(String),
    /// An at-rule rdom evaluates whose prelude is invalid — `@layer a
    /// b;`, `@layer a, b { … }`, a reserved layer name. The whole rule
    /// (block included) is dropped (CSS Syntax 3 §5.4.2).
    InvalidAtRulePrelude {
        name: String,
        prelude: String,
    },
    InvalidSelector(String),
    /// A value in an `@media` or `@container` feature (as written) whose
    /// unit has no cell measure — `ex`, `cap`, `lh`, a viewport unit, a
    /// resolution: the feature is unknown, so neither it nor its `not`
    /// matches. `px`, `em`, `rem` and the absolute units map onto cells
    /// (DIVERGENCES §2, C14G-PX-BREAKPOINTS). The rule is kept.
    UnmeasuredQueryValue(String),
    /// An `@import` (its URL) after a rule other than `@charset` and
    /// `@layer` statements, or inside a block: ignored (CSS Cascade 5 §3).
    ImportIgnored(String),
    /// An `@import` (its URL as written) whose sheet — by the resolved
    /// URL the loader returned — is already being parsed: skipped.
    ImportCycle(String),
    /// An `@import` (its URL) nested deeper than [`MAX_IMPORT_DEPTH`](crate::MAX_IMPORT_DEPTH):
    /// nothing is imported.
    ImportTooDeep(String),
    /// A block nested deeper than [`MAX_BLOCK_DEPTH`](crate::MAX_BLOCK_DEPTH) — a style rule's or
    /// an at-rule's: skipped whole, and the parse goes on after its `}`.
    BlockTooDeep,
    /// An `@import` whose sheet did not load: no loader was given, or
    /// the loader refused (`reason`). Nothing is imported.
    ImportFailed {
        url: String,
        reason: String,
    },
    /// An `@property` rule (its prelude, `name`) that registers nothing:
    /// a missing or invalid descriptor, an invalid name, or an initial
    /// value that does not match the syntax (CSS Properties and Values
    /// API 1 §3).
    InvalidPropertyRule {
        name: String,
        reason: String,
    },
    /// An `@counter-style` rule (its prelude, `name`) that defines
    /// nothing, and why (CSS Counter Styles 3 §3).
    InvalidCounterStyleRule {
        name: String,
        reason: CounterStyleRuleReason,
    },
    /// A declaration of the `@counter-style` rule `name` dropped while the
    /// rule is defined (CSS Counter Styles 3 §3, CSS Syntax 3 §5.4.6):
    /// `descriptor` is its name — the whole declaration's text when it is
    /// not `descriptor: value` — and `reason` why.
    CounterStyleDescriptorDropped {
        name: String,
        descriptor: String,
        reason: CounterStyleDescriptorReason,
    },
    /// A keyframe block of an `@keyframes` rule whose selector list (as
    /// written) holds an invalid `<keyframe-selector>` — not `from`, `to`
    /// or a percentage in [0%, 100%]: the block is ignored (CSS Animations
    /// 1 §3).
    InvalidKeyframeSelector(String),
    /// A declaration (its property name) marked `!important` in a
    /// keyframe block: ignored (CSS Animations 1 §3).
    ImportantInKeyframe(String),
    /// A declaration of the `@position-try` rule `name` dropped (CSS
    /// Anchor Positioning 1 §4.1): `descriptor` is its property name
    /// (lowercased) and `reason` why — no descriptor of the rule, or
    /// `!important`.
    PositionTryDescriptorDropped {
        name: String,
        descriptor: String,
        reason: PositionTryDescriptorReason,
    },
    UnterminatedComment,
    UnterminatedString,
}

/// Why an `@counter-style` rule defines nothing
/// ([`WarningKind::InvalidCounterStyleRule`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CounterStyleRuleReason {
    /// The rule has no `{ … }` block.
    MissingBlock,
    /// Its block holds an unterminated string or comment.
    Unterminated,
    /// Its prelude is not one identifier.
    NotOneIdentifier,
    /// [`check_rule`](rdom_style::counters::check_rule) refused it.
    Rule(rdom_style::counters::CounterStyleRuleError),
}

/// Why a declaration of an `@counter-style` rule was dropped
/// ([`WarningKind::CounterStyleDescriptorDropped`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CounterStyleDescriptorReason {
    /// The declaration is not `descriptor: value`.
    Malformed,
    /// [`apply_descriptor`](rdom_style::counters::apply_descriptor)
    /// refused it.
    Descriptor(rdom_style::counters::DescriptorError),
}

/// Why a declaration of a `@position-try` rule was dropped
/// ([`WarningKind::PositionTryDescriptorDropped`], CSS Anchor Positioning
/// 1 §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PositionTryDescriptorReason {
    /// The property is none of the rule's descriptors (the inset, margin,
    /// sizing and self-alignment properties, `position-anchor`,
    /// `position-area`).
    NotADescriptor,
    /// The declaration is `!important`, which the descriptors do not take.
    Important,
}
