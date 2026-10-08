//! The packer's style switch at the first line's end (CSS
//! Pseudo-Elements 4 §2.2, `first_line`): while the first line is packed,
//! each run's text-shaping values are the first line's mapping of them
//! ([`FirstLineRun`]); when the line is settled the mapping ends, and the
//! word waiting in the buffer for the next line — taken in under the
//! first line's style — is reshaped in its own. The line keeps the hosts
//! whose `::first-line` paint reads (`LineBox::first_line`).

use std::borrow::Cow;

use rdom_core::NodeId;
use unicode_width::UnicodeWidthStr;

use super::super::first_line::FirstLineRun;
use super::super::run_style::RunStyle;
use super::super::transform::{self, CaseContext};
use super::{GraphemeKind, LinePacker, PendingGrapheme};

/// What the packer keeps of the first formatted line it packs.
#[derive(Debug, Clone)]
pub(in crate::render::inline) struct FirstLinePacking {
    /// The block containers whose first formatted line it is, innermost
    /// first: what the settled line keeps for paint.
    hosts: Box<[NodeId]>,
    /// The mapping of each run's values on the line; `None` when no
    /// `::first-line` there shapes text.
    run: Option<FirstLineRun>,
    /// The first line was settled: the mapping is over.
    done: bool,
}

impl<'a> LinePacker<'a> {
    /// The first line packed is the first formatted line of `hosts`
    /// (innermost first, `first_line::hosts`), its runs mapped by `run`.
    pub(in crate::render::inline) fn first_line(
        mut self,
        hosts: Vec<NodeId>,
        run: Option<FirstLineRun>,
    ) -> Self {
        if !hosts.is_empty() {
            self.first = Some(Box::new(FirstLinePacking {
                hosts: hosts.into_boxed_slice(),
                run,
                done: false,
            }));
        }
        self
    }

    /// The first line's packing, for a replica of this packer: as it was
    /// before any line was packed.
    pub(super) fn first_packing(&self) -> Option<Box<FirstLinePacking>> {
        self.first.clone().map(|mut f| {
            f.done = false;
            f
        })
    }

    /// The first line's mapping, while it is packed.
    fn first_run(&self) -> Option<&FirstLineRun> {
        self.first
            .as_deref()
            .filter(|f| !f.done)
            .and_then(|f| f.run.as_ref())
    }

    /// Take text in under `run` — mapped while the first line's style
    /// shapes it.
    pub(super) fn set_run(&mut self, run: RunStyle) {
        self.run_source = run;
        self.run = match self.first_run() {
            Some(map) => map.apply(run),
            None => run,
        };
    }

    /// Whether the first line's style maps the runs taken in now.
    pub(super) fn first_maps(&self) -> bool {
        self.first_run().is_some()
    }

    /// The first line was settled: it keeps its hosts, the runs are their
    /// own again, and the buffered word — the next line's start — and the
    /// pending separator are reshaped in their own style.
    pub(super) fn end_first_line(&mut self) {
        let Some(first) = self.first.as_deref_mut().filter(|f| !f.done) else {
            return;
        };
        first.done = true;
        let (hosts, shapes) = (first.hosts.clone(), first.run.is_some());
        if let Some(line) = self.lines.first_mut() {
            line.first_line = Some(hosts);
        }
        if !shapes {
            return;
        }
        self.run = self.run_source;
        let buffer = std::mem::take(&mut self.word_buffer);
        let mut ctx = CaseContext::default();
        self.word_width = 0;
        for g in buffer {
            let g = match g.unmapped {
                Some(run) => self.reshape(g, run, &mut ctx),
                None => g,
            };
            self.word_width = self.word_width.saturating_add(g.width);
            self.word_buffer.push(g);
        }
        if self.pending_space {
            self.pending_space_spacing = self.spacing_after(' ', true);
        }
    }

    /// `g`, taken in under the first line's mapping of `run`, as `run`
    /// itself makes it: its text transformed and spaced again (CSS Text 3
    /// §2.1, §9); a tab and a soft hyphen keep theirs.
    fn reshape(
        &mut self,
        g: PendingGrapheme<'a>,
        run: RunStyle,
        ctx: &mut CaseContext,
    ) -> PendingGrapheme<'a> {
        let mapped = self.run;
        self.run = run;
        let source = g.source;
        let first = source.chars().next().unwrap_or(' ');
        let reshaped = match g.kind {
            GraphemeKind::Text => {
                let rendered = transform::apply(source, run.transform, *ctx, None);
                *ctx = ctx.after(source);
                let text = rendered.unwrap_or(Cow::Borrowed(source));
                let width = UnicodeWidthStr::width(text.as_ref()) as u16;
                let spacing = self.spacing_after(
                    text.chars().next().unwrap_or(first),
                    super::spacing::is_word_separator(first),
                );
                Some((text, width, spacing))
            }
            GraphemeKind::Preserved { tab: None, .. } => {
                ctx.break_word();
                let (text, width): (&'a str, u16) = if run.transform.full_width {
                    ("\u{3000}", 2)
                } else if source == " " {
                    (source, 1)
                } else {
                    (" ", 1)
                };
                let spacing = self.spacing_after(' ', text == " ");
                Some((Cow::Borrowed(text), width, spacing))
            }
            GraphemeKind::Collapsible { .. } => {
                ctx.break_word();
                Some((Cow::Borrowed(" "), 1, self.spacing_after(' ', true)))
            }
            GraphemeKind::Preserved { .. } | GraphemeKind::SoftHyphen { .. } => None,
        };
        let out = match reshaped {
            Some((text, width, spacing)) => {
                let mut piece = self
                    .piece(g.origin, g.source_offset, source, text, width, g.kind)
                    .spaced(spacing);
                piece.split = g.split;
                piece.unmapped = None;
                piece
            }
            None => PendingGrapheme {
                unmapped: None,
                ..g
            },
        };
        self.run = mapped;
        out
    }
}
