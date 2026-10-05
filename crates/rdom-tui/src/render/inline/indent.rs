//! `text-indent` (CSS Text 3 §8.1) as the packer applies it: which lines
//! of an inline formatting context are indented, and by how many cells —
//! "a margin applied to the start edge of the line box".

use crate::layout::TextIndent;

/// The indent of a flow's lines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LineIndent {
    /// The indent in cells, either sign.
    cells: i32,
    hanging: bool,
    each_line: bool,
    /// The flow's first line is the first formatted line of its element
    /// (§8.1: "the first line of an anonymous block box is only affected
    /// if it is the first child of its parent element").
    first_formatted: bool,
}

impl LineIndent {
    /// The indent `indent` gives a flow `width` cells wide — a percentage
    /// of it; `first_formatted` when the flow's first line is its
    /// element's first formatted line.
    pub(crate) fn of(indent: &TextIndent, width: u16, first_formatted: bool) -> Self {
        LineIndent {
            cells: indent.resolve(width),
            hanging: indent.hanging,
            each_line: indent.each_line,
            first_formatted,
        }
    }

    /// The indent of a line: the flow's first (`first`), or one after a
    /// forced line break (`after_forced`) or a soft wrap. §8.1: the first
    /// formatted line, and with `each-line` each line after a forced
    /// break; `hanging` inverts which.
    pub(crate) fn of_line(self, first: bool, after_forced: bool) -> i32 {
        let affected = (first && self.first_formatted) || (self.each_line && after_forced);
        if affected != self.hanging {
            self.cells
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §8.1's line selection, with and without the keywords.
    #[test]
    fn which_lines_are_indented() {
        let mut t = TextIndent::cells(2);
        let plain = LineIndent::of(&t, 10, true);
        assert_eq!(
            (plain.of_line(true, false), plain.of_line(false, true)),
            (2, 0)
        );
        assert_eq!(LineIndent::of(&t, 10, false).of_line(true, false), 0);
        t.each_line = true;
        let each = LineIndent::of(&t, 10, true);
        assert_eq!(
            (each.of_line(false, true), each.of_line(false, false)),
            (2, 0)
        );
        t.each_line = false;
        t.hanging = true;
        let hanging = LineIndent::of(&t, 10, true);
        assert_eq!(
            (hanging.of_line(true, false), hanging.of_line(false, false)),
            (0, 2)
        );
    }
}
