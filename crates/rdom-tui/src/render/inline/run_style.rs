//! [`RunStyle`]: the CSS Text properties of one run of text — what the
//! packer reads per text node (its parent element's computed style) or
//! per generated run (its pseudo-element's). They apply to text, so
//! each inline element's text is processed and wrapped by its own
//! values (CSS Text 3 §3: `white-space` applies to text).

use crate::layout::WhiteSpaceCollapse;
use crate::style::ComputedStyle;

/// The CSS Text values the packer applies to a run of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RunStyle {
    /// `white-space-collapse` (CSS Text 4 §4.1).
    pub(crate) collapse: WhiteSpaceCollapse,
    /// `text-wrap-mode: wrap` (CSS Text 4 §6.1).
    pub(crate) wraps: bool,
}

impl Default for RunStyle {
    fn default() -> Self {
        RunStyle {
            collapse: WhiteSpaceCollapse::Collapse,
            wraps: true,
        }
    }
}

impl RunStyle {
    /// The run style of text styled `style`.
    pub(crate) fn of(style: &ComputedStyle) -> Self {
        RunStyle {
            collapse: style.text.white_space_collapse,
            wraps: style.text.wraps(),
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
