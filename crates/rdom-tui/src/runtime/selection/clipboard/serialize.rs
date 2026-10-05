//! Selection → plain-text serializer for copy / cut.
//!
//! The clipboard gets the selection's **rendered text** — what the user
//! sees — following HTML §3.2.7's rendered text collection steps (the
//! `innerText` algorithm, which Chromium's selection serializer tracks):
//!
//! - Collapsible whitespace collapses per `white-space` (CSS Text 3 §4.1):
//!   under `normal` / `nowrap` a run of spaces, tabs and newlines is one
//!   space, and a space at the start or end of a line — next to a block
//!   edge, a table-cell edge or a `<br>` — is removed. The collapse runs
//!   over the whole line, not just the selected part, so a selection that
//!   starts inside a run of spaces mid-line keeps one space.
//!   `pre` / `pre-wrap` text is copied verbatim.
//! - A block-level box (`display: block`, which in rdom includes flex
//!   containers and table sections) is set off by one line break; runs of
//!   required breaks between two pieces of text collapse to the largest,
//!   and breaks at the start or end of the selection are dropped.
//!   `<p>` asks for two — a blank line — when it has a vertical margin
//!   (innerText's two breaks model the paragraph margin; rdom's UA `<p>`
//!   has none, so by default it is one break like any block).
//! - `<br>` is a line break.
//! - Table cells (`<td>` / `<th>`) are separated by a tab and rows by a
//!   line break.
//! - `display: none` subtrees contribute nothing. Generated content
//!   (`::before` / `::after`, list markers) is not in the DOM and is never
//!   copied, as in browsers.
//! - Text whose used `user-select` is `none` is not copied, though it
//!   still takes part in whitespace collapsing (it is rendered). The walk
//!   carries the used value down (CSS UI 4 §6.1), so an explicit `text`
//!   descendant of a `none` element is still copied.
//!
//! The walk streams: every rendered character goes through one
//! collapse-state machine and only the selected ones are kept, so memory
//! is proportional to the selection. It stops at the first rendered
//! character or line break after the selection's end (the lookahead that
//! decides whether a trailing space ended a line).

use rdom_core::{Dom, NodeId, NodeType, Range};

use crate::ext::TuiExt;
use crate::layout::{Display, MarginValue, UserSelect, WhiteSpace};
use crate::runtime::selection::user_select;

/// The rendered text of `range`. Empty when the range is collapsed or
/// selects no rendered, selectable text.
pub fn serialize_selection(dom: &Dom<TuiExt>, range: &Range) -> String {
    let mut walk = Walk {
        dom,
        range,
        state: WalkState::Before,
        out: Rendered::default(),
    };
    let root = dom.root();
    let used = user_select::used_value(dom, root);
    walk.visit(root, used);
    walk.out.finish()
}

/// Where the walk is relative to the range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WalkState {
    Before,
    Inside,
    Done,
}

struct Walk<'a> {
    dom: &'a Dom<TuiExt>,
    range: &'a Range,
    state: WalkState,
    out: Rendered,
}

impl Walk<'_> {
    /// `used` is the used `user-select` of `id` (for a non-element, its
    /// parent element's).
    fn visit(&mut self, id: NodeId, used: UserSelect) {
        if self.out.closed {
            return;
        }
        let dom = self.dom;
        let node = dom.node(id);
        match node.node_type() {
            NodeType::Text => self.text(id, used),
            NodeType::Element => {
                let computed = node.ext().and_then(|e| e.computed.as_ref());
                if computed.is_some_and(|c| c.display == Display::None) {
                    // Not rendered: no text, but a range boundary inside
                    // still opens or closes the selection.
                    self.skip_subtree(id);
                    return;
                }
                match node.tag_name() {
                    Some("br") => {
                        let selected = self.state == WalkState::Inside;
                        self.out.preserved('\n', selected);
                        return;
                    }
                    Some("td" | "th") => {
                        self.out.edge();
                        self.children(id, used);
                        self.out.edge();
                        if self.next_cell(id) {
                            let selected = self.state == WalkState::Inside;
                            self.out.preserved('\t', selected);
                        }
                        return;
                    }
                    _ => {}
                }
                let breaks = match computed {
                    Some(c) if c.display == Display::Block => {
                        if node.tag_name() == Some("p") && has_vertical_margin(c) {
                            2
                        } else {
                            1
                        }
                    }
                    _ => 0,
                };
                if breaks > 0 {
                    self.out.line_breaks(breaks);
                }
                self.children(id, used);
                if breaks > 0 {
                    self.out.line_breaks(breaks);
                }
            }
            // Document / fragment: recurse. Comments render nothing.
            NodeType::Comment => {}
            _ => self.children(id, used),
        }
    }

    fn children(&mut self, id: NodeId, used: UserSelect) {
        let dom = self.dom;
        let mut child = dom.node(id).first_child().map(|c| c.id());
        while let Some(c) = child {
            let child_used = if dom.node(c).node_type() == NodeType::Element {
                user_select::resolve_child(dom, c, used)
            } else {
                used
            };
            self.visit(c, child_used);
            if self.out.closed {
                return;
            }
            child = dom.node(c).next_sibling().map(|n| n.id());
        }
    }

    /// Feed a text node's characters, the selected ones marked.
    fn text(&mut self, id: NodeId, used: UserSelect) {
        let dom = self.dom;
        let Some(data) = dom.node(id).node_value() else {
            return;
        };
        let (from, to, ends_here) = self.selected_bytes(id, data.len());
        // HTML §3.2.7 rendered text: a node whose `visibility` is not
        // `visible` contributes no text (CSS Display 3 §4) — the used
        // value paint draws its owner's text by, a running transition's
        // included (`render::visibility`).
        let shown = dom.node(id).parent_element().is_none_or(|p| {
            crate::render::visibility::shows(dom, p.id(), crate::ext::StyleSlot::Host)
        });
        let copyable = used != UserSelect::None && shown;
        let collapsible = dom
            .node(id)
            .parent_element()
            .and_then(|p| p.ext().and_then(|e| e.computed.as_ref()))
            .is_none_or(|c| matches!(c.white_space, WhiteSpace::Normal | WhiteSpace::NoWrap));
        for (i, ch) in data.char_indices() {
            if ends_here && i >= to {
                // Past the end: the rest is lookahead.
                self.out.selection_ended();
            }
            let selected = copyable && i >= from && i < to;
            if collapsible {
                self.out.collapsible(ch, selected);
            } else {
                self.out.preserved(ch, selected);
            }
            if self.out.closed {
                return;
            }
        }
        if ends_here {
            self.out.selection_ended();
        }
    }

    /// The byte span of text node `id` inside the range (empty when none)
    /// and whether the range ends in it, advancing the walk state past its
    /// boundaries.
    fn selected_bytes(&mut self, id: NodeId, len: usize) -> (usize, usize, bool) {
        let is_start = self.range.start.node == id;
        let is_end = self.range.end.node == id;
        let start = self.range.start.offset.min(len);
        let end = self.range.end.offset.min(len);
        match (is_start, is_end, self.state) {
            (true, true, _) => {
                self.state = WalkState::Done;
                (start, end.max(start), true)
            }
            (true, false, _) => {
                self.state = WalkState::Inside;
                (start, len, false)
            }
            (false, true, WalkState::Inside) => {
                self.state = WalkState::Done;
                (0, end, true)
            }
            (false, false, WalkState::Inside) => (0, len, false),
            _ => (0, 0, false),
        }
    }

    /// Advance the walk state over an unrendered subtree's text nodes.
    fn skip_subtree(&mut self, id: NodeId) {
        let dom = self.dom;
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            let node = dom.node(n);
            if node.node_type() == NodeType::Text {
                let len = node.node_value().map_or(0, str::len);
                if self.selected_bytes(n, len).2 {
                    self.out.selection_ended();
                }
            }
            let kids: Vec<NodeId> = node.child_nodes().map(|c| c.id()).collect();
            stack.extend(kids.into_iter().rev());
        }
    }

    /// Whether a rendered table cell follows `cell` in its row.
    fn next_cell(&self, cell: NodeId) -> bool {
        let mut sib = self.dom.node(cell).next_sibling().map(|n| n.id());
        while let Some(s) = sib {
            let node = self.dom.node(s);
            if matches!(node.tag_name(), Some("td" | "th"))
                && !node
                    .ext()
                    .and_then(|e| e.computed.as_ref())
                    .is_some_and(|c| c.display == Display::None)
            {
                return true;
            }
            sib = node.next_sibling().map(|n| n.id());
        }
        false
    }
}

fn has_vertical_margin(c: &crate::style::ComputedStyle) -> bool {
    let nonzero = |m: &MarginValue| !matches!(m, MarginValue::Cells(0) | MarginValue::Auto);
    nonzero(&c.margin.top) || nonzero(&c.margin.bottom)
}

/// The collapse-state machine over the rendered characters, in order.
#[derive(Default)]
struct Rendered {
    out: String,
    /// A collapsible space waits to see whether the line goes on; the
    /// flag says whether it is selected.
    pending_space: Option<bool>,
    /// At the start of a line (collapsible spaces are removed).
    mid_line: bool,
    /// Required line breaks between the last rendered character and the
    /// next one (the largest request wins).
    pending_breaks: u8,
    /// Something selected has been emitted (breaks before it are dropped).
    emitted: bool,
    /// The selection ended; the next rendered character or line break
    /// settles the trailing space and closes the walk.
    ended: bool,
    closed: bool,
}

impl Rendered {
    fn collapsible(&mut self, ch: char, selected: bool) {
        if matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0c') {
            if !self.mid_line {
                return;
            }
            self.pending_space = Some(self.pending_space.unwrap_or(false) || selected);
        } else {
            self.render(ch, selected);
        }
    }

    fn preserved(&mut self, ch: char, selected: bool) {
        if ch == '\n' {
            // A forced break ends the line: a collapsible space before
            // it is removed.
            self.pending_space = None;
            self.emit(ch, selected);
            self.mid_line = false;
        } else {
            self.render(ch, selected);
        }
    }

    /// A block or table-cell edge: ends the line for collapsing without
    /// adding a break.
    fn edge(&mut self) {
        self.pending_space = None;
        self.mid_line = false;
        if self.ended {
            self.closed = true;
        }
    }

    fn line_breaks(&mut self, n: u8) {
        self.edge();
        self.pending_breaks = self.pending_breaks.max(n);
    }

    fn selection_ended(&mut self) {
        self.ended = true;
    }

    fn render(&mut self, ch: char, selected: bool) {
        if let Some(space_selected) = self.pending_space.take() {
            self.emit(' ', space_selected);
        }
        self.emit(ch, selected);
        self.mid_line = true;
    }

    fn emit(&mut self, ch: char, selected: bool) {
        if self.ended && !selected {
            self.closed = true;
            return;
        }
        if selected {
            if self.emitted {
                for _ in 0..self.pending_breaks {
                    self.out.push('\n');
                }
            }
            self.out.push(ch);
            self.emitted = true;
        }
        self.pending_breaks = 0;
    }

    fn finish(self) -> String {
        self.out
    }
}
