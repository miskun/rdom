//! [`RunStyle`]: the CSS Text properties of one run of text — what the
//! packer reads per text node (its parent element's computed style) or
//! per generated run (its pseudo-element's). They apply to text, so
//! each inline element's text is processed and wrapped by its own
//! values (CSS Text 3 §3: `white-space` applies to text).

use super::breaking::BreakRules;
use crate::layout::{OverflowWrap, WhiteSpaceCollapse, WordBreak};
use crate::style::ComputedStyle;

/// The CSS Text values the packer applies to a run of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RunStyle {
    /// `white-space-collapse` (CSS Text 4 §4.1).
    pub(crate) collapse: WhiteSpaceCollapse,
    /// `text-wrap-mode: wrap` (CSS Text 4 §6.1).
    pub(crate) wraps: bool,
    /// `word-break`, `line-break`, `hyphens` (CSS Text 3 §5.2, §5.3,
    /// §6.1).
    pub(crate) breaks: BreakRules,
    /// `overflow-wrap` — `anywhere` under `word-break: break-word` (§5.2).
    pub(crate) overflow_wrap: OverflowWrap,
    /// `tab-size` in cells (§4.2).
    pub(crate) tab_size: u16,
}

impl Default for RunStyle {
    fn default() -> Self {
        RunStyle {
            collapse: WhiteSpaceCollapse::Collapse,
            wraps: true,
            breaks: BreakRules::default(),
            overflow_wrap: OverflowWrap::Normal,
            tab_size: 8,
        }
    }
}

impl RunStyle {
    /// The run style of text styled `style`.
    pub(crate) fn of(style: &ComputedStyle) -> Self {
        let text = &style.text;
        // §5.2: `break-word` is `normal` with `overflow-wrap: anywhere`.
        let legacy = text.word_break == WordBreak::BreakWord;
        RunStyle {
            collapse: text.white_space_collapse,
            wraps: text.wraps(),
            breaks: BreakRules {
                word_break: if legacy {
                    WordBreak::Normal
                } else {
                    text.word_break
                },
                line_break: text.line_break,
                hyphens: text.hyphens,
            },
            overflow_wrap: if legacy {
                OverflowWrap::Anywhere
            } else {
                text.overflow_wrap
            },
            tab_size: text.tab_size.cells(),
        }
    }

    /// Whether preserved white space at the end of a line hangs (CSS Text
    /// 3 §4.1.2): under `pre-wrap` (and `preserve-spaces wrap`); under
    /// `break-spaces` it takes up space, and without wrapping there is no
    /// soft-wrapped line end.
    pub(crate) fn hangs_spaces(self) -> bool {
        self.wraps
            && matches!(
                self.collapse,
                WhiteSpaceCollapse::Preserve | WhiteSpaceCollapse::PreserveSpaces
            )
    }
}
