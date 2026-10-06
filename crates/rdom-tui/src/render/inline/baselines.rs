//! The first and last baselines of an inline layout (CSS 2.1 §10.8.1, CSS
//! Box Alignment 3 §9.1): its first and last line boxes' glyph rows, which
//! leading (`line-height`) and `vertical-align` move within each line. A
//! box's baselines are its packed lines' (`layout_pass::baselines`).

use super::InlineLayout;

impl InlineLayout {
    /// The glyph rows of its first and last lines, from its top; `None`
    /// with no line.
    pub(crate) fn baselines(&self) -> Option<(u16, u16)> {
        let first = self.lines.first()?.text_row();
        let last = self.lines.last()?.text_row();
        Some((first, last))
    }
}
