//! Computed values made absolute: the viewport-percentage lengths of a
//! computed style resolved against the terminal (CSS Values 4 §6.1.2),
//! and the line-height units (`lh`, `rlh`, §6.1.1) against the line
//! heights — they are absolute lengths at computed-value time.
//! Percentages stay for layout, which alone knows their basis; an
//! expression left with none folds to whole cells.

mod expr;

use expr::{
    Reading, absolutize, absolutize_list, absolutize_template, absolutize_transform,
    absolutize_translate, needs, transform_has_calc,
};
pub(crate) use expr::{cells_i32, cells_u16};

use crate::ComputedStyle;
use crate::calc::{CalcExpr, UnitContext, UnitReads, Viewport};
use crate::layout::{
    BorderWidth, FlexBasis, GapValue, IntrinsicSize, Length, MarginValue, MaxSize, MinSize,
    PaddingValue, PaintLength, Size,
};

impl ComputedStyle {
    /// Resolve every viewport-percentage length in the style's
    /// length-bearing values against `viewport`, and the line-height
    /// units as one row each ([`Self::resolve_context_units`] with
    /// [`UnitContext::new`]); returns what that read.
    pub fn resolve_viewport_units(&mut self, viewport: Viewport) -> UnitReads {
        self.resolve_context_units(&UnitContext::new(viewport))
    }

    /// Resolve every unit that needs a context — the viewport-percentage
    /// lengths, `lh` and `rlh` — in the style's length-bearing values
    /// against `cx`. The cascade calls this once per computed style, once
    /// `line-height` is computed (`lh` reads it); values without such a
    /// unit are untouched. Returns the context sizes the values read —
    /// what the style depends on beside its declarations.
    pub fn resolve_context_units(&mut self, cx: &UnitContext) -> UnitReads {
        let vp = &Reading::new(cx);
        absolutize(&mut self.width, vp, Size::Calc, |v| {
            Size::Fixed(cells_u16(v))
        });
        absolutize(&mut self.height, vp, Size::Calc, |v| {
            Size::Fixed(cells_u16(v))
        });
        // A `calc-size()` offset's viewport and line-height units.
        let absolute = |c: &std::sync::Arc<crate::layout::CalcSize>| {
            c.offset.needs_context().then(|| {
                let mut absolute = (**c).clone();
                absolute.offset = vp.absolutize(&absolute.offset);
                std::sync::Arc::new(absolute)
            })
        };
        for size in [&mut self.width, &mut self.height] {
            if let Size::CalcSize(c) = size
                && let Some(a) = absolute(c)
            {
                *size = Size::CalcSize(a);
            }
        }
        for min in [&mut self.min_width, &mut self.min_height] {
            if let MinSize::CalcSize(c) = min
                && let Some(a) = absolute(c)
            {
                *min = MinSize::CalcSize(a);
            }
        }
        for max in [&mut self.max_width, &mut self.max_height] {
            if let MaxSize::CalcSize(c) = max
                && let Some(a) = absolute(c)
            {
                *max = MaxSize::CalcSize(a);
            }
        }
        if let FlexBasis::CalcSize(c) = &self.flex_basis
            && let Some(a) = absolute(c)
        {
            self.flex_basis = FlexBasis::CalcSize(a);
        }
        for min in [&mut self.min_width, &mut self.min_height] {
            absolutize(min, vp, MinSize::Calc, |v| MinSize::Cells(cells_u16(v)));
        }
        for max in [&mut self.max_width, &mut self.max_height] {
            absolutize(max, vp, MaxSize::Calc, |v| MaxSize::Cells(cells_u16(v)));
        }
        // A `fit-content()` limit stays an expression (layout resolves it
        // against the containing block), with its viewport units absolute.
        let limits = [
            match &mut self.width {
                Size::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.height {
                Size::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.min_width {
                MinSize::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.min_height {
                MinSize::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.max_width {
                MaxSize::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.max_height {
                MaxSize::Intrinsic(k) => Some(k),
                _ => None,
            },
            match &mut self.flex_basis {
                FlexBasis::Intrinsic(k) => Some(k),
                _ => None,
            },
        ];
        for size in [
            &mut self.contain_intrinsic_width,
            &mut self.contain_intrinsic_height,
        ] {
            if let Some(expr) = size.length.as_mut().filter(|e| e.needs_context()) {
                let absolute = vp.absolutize(expr);
                *expr = CalcExpr::Length(cells_i32(
                    absolute.resolve_f64(&crate::calc::ResolveCtx::new(0)),
                ));
            }
        }
        for limit in limits.into_iter().flatten() {
            if let IntrinsicSize::FitContentLimit(expr) = limit
                && expr.needs_context()
            {
                *expr = std::sync::Arc::new(vp.absolutize(expr));
            }
        }
        let p = &mut self.padding;
        for side in [&mut p.top, &mut p.right, &mut p.bottom, &mut p.left] {
            absolutize(side, vp, PaddingValue::Calc, |v| {
                PaddingValue::Cells(cells_u16(v))
            });
        }
        let sp = &mut self.scroll_padding;
        for side in [&mut sp.top, &mut sp.right, &mut sp.bottom, &mut sp.left] {
            if let crate::layout::ScrollPadding::Length(v) = side {
                absolutize(v, vp, PaddingValue::Calc, |v| {
                    PaddingValue::Cells(cells_u16(v))
                });
            }
        }
        let m = &mut self.margin;
        for side in [&mut m.top, &mut m.right, &mut m.bottom, &mut m.left] {
            absolutize(side, vp, MarginValue::Calc, |v| {
                MarginValue::Cells(
                    cells_i32(v).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
                )
            });
        }
        for gap in [
            &mut self.row_gap,
            &mut self.column_gap,
            &mut self.border_spacing.horizontal,
            &mut self.border_spacing.vertical,
        ] {
            absolutize(gap, vp, GapValue::Calc, |v| GapValue::Cells(cells_u16(v)));
        }
        absolutize(&mut self.flex_basis, vp, FlexBasis::Calc, |v| {
            FlexBasis::Cells(cells_u16(v))
        });
        for width in self.border_width.each_mut() {
            absolutize(
                width,
                vp,
                |e| BorderWidth::Length(PaintLength::Calc(e)),
                |v| BorderWidth::Length(PaintLength::Cells(v as f32)),
            );
        }
        // CSS Multi-column 1 §3.1, §4.3: the column width and the rule's
        // width — the shared group copied only when one needs it.
        if needs(&self.multicol.column_width) || needs(&self.multicol.column_rule_width) {
            let m = &mut *self.multicol;
            absolutize(
                &mut m.column_width,
                vp,
                crate::layout::ColumnWidth::Calc,
                |v| crate::layout::ColumnWidth::Cells(cells_u16(v)),
            );
            absolutize(
                &mut m.column_rule_width,
                vp,
                |e| BorderWidth::Length(PaintLength::Calc(e)),
                |v| BorderWidth::Length(PaintLength::Cells(v as f32)),
            );
        }
        // CSS UI 4 §5.3–§5.4: the outline's width and offset.
        if needs(&self.ui.outline_width) || needs(&self.ui.outline_offset) {
            let ui = &mut *self.ui;
            absolutize(
                &mut ui.outline_width,
                vp,
                |e| BorderWidth::Length(PaintLength::Calc(e)),
                |v| BorderWidth::Length(PaintLength::Cells(v as f32)),
            );
            absolutize(&mut ui.outline_offset, vp, PaintLength::Calc, |v| {
                PaintLength::Cells(v as f32)
            });
        }
        for shadow in &mut self.box_shadow {
            for length in [
                &mut shadow.offset_x,
                &mut shadow.offset_y,
                &mut shadow.blur,
                &mut shadow.spread,
            ] {
                absolutize(length, vp, PaintLength::Calc, |v| {
                    PaintLength::Cells(v as f32)
                });
            }
        }
        for radius in self.border_radius.each_mut() {
            for axis in [&mut radius.horizontal, &mut radius.vertical] {
                absolutize(axis, vp, PaintLength::Calc, |v| {
                    PaintLength::Cells(v as f32)
                });
            }
        }
        // The grid group is shared: a grid that declares track lists is
        // worked on as a copy, written back only when a value moved.
        let g = &*self.grid;
        let tracks =
            |t: &crate::layout::GridTemplate| matches!(t, crate::layout::GridTemplate::Tracks(_));
        let owned = |l: &std::borrow::Cow<'static, [crate::layout::TrackSize]>| {
            matches!(l, std::borrow::Cow::Owned(_))
        };
        if tracks(&g.grid_template_columns)
            || tracks(&g.grid_template_rows)
            || owned(&g.grid_auto_columns)
            || owned(&g.grid_auto_rows)
        {
            let mut grid = g.clone();
            for template in [
                &mut grid.grid_template_columns,
                &mut grid.grid_template_rows,
            ] {
                absolutize_template(template, vp);
            }
            for list in [&mut grid.grid_auto_columns, &mut grid.grid_auto_rows] {
                absolutize_list(list, vp);
            }
            if grid != *self.grid {
                self.grid = grid.into();
            }
        }
        // CSS Transforms 2 §6.1: the translations' offsets (a percentage of
        // the reference box stays for layout).
        if self
            .effects
            .translate
            .as_ref()
            .is_some_and(|t| needs(&t.x) || needs(&t.y))
            && let Some(t) = &mut self.effects.translate
        {
            absolutize_translate(t, vp);
        }
        if transform_has_calc(&self.effects.transform) {
            absolutize_transform(&mut self.effects.transform, vp);
        }
        // Transforms 1 §6: the origin's lengths (inert, kept absolute).
        let origin = &self.effects.transform_origin;
        if origin.needs_context() {
            let mut lengths = [origin.x(), origin.y(), origin.z()];
            for l in &mut lengths {
                absolutize(l, vp, PaintLength::Calc, |v| PaintLength::Cells(v as f32));
            }
            let [x, y, z] = lengths;
            self.effects.transform_origin = crate::layout::TransformOrigin::new(x, y, z);
        }
        for inset in [
            &mut self.top,
            &mut self.right,
            &mut self.bottom,
            &mut self.left,
        ] {
            absolutize(inset, vp, Length::Calc, |v| Length::Cells(cells_i32(v)));
        }
        // CSS Text Decoration 4 §2.5 / §4.2: the decoration lengths
        // (parsed and kept, not drawn).
        if let crate::layout::TextDecorationThickness::Length(length) =
            &mut self.text_decoration.thickness
        {
            absolutize(length, vp, PaintLength::Calc, |v| {
                PaintLength::Cells(v as f32)
            });
        }
        if let crate::layout::TextUnderlineOffset::Length(length) =
            &mut self.text.text_underline_offset
        {
            absolutize(length, vp, PaintLength::Calc, |v| {
                PaintLength::Cells(v as f32)
            });
        }
        // CSS Fonts 4 §2.5: `font-size` (parsed and kept, not drawn).
        if let crate::layout::FontSize::Length(length) = &mut self.font.size {
            absolutize(length, vp, PaintLength::Calc, |v| {
                PaintLength::Cells(v as f32)
            });
        }
        // CSS Text 3 §9.1 / §9.2: the spacing lengths.
        for spacing in [&mut self.text.letter_spacing, &mut self.text.word_spacing] {
            absolutize(spacing, vp, crate::layout::Spacing::Calc, |v| {
                crate::layout::Spacing::Cells(v as f32)
            });
        }
        // CSS Text 3 §8.1: `text-indent` is a length-percentage.
        absolutize(&mut self.text.text_indent.length, vp, Length::Calc, |v| {
            Length::Cells(cells_i32(v))
        });
        // Scroll-driven Animations 1 §3.2.3 / §4.3: the view timeline
        // insets and the range offsets are length-percentages.
        // The motion group is shared: one that holds insets or ranges is
        // worked on as a copy, written back only when a value moved.
        let m = &*self.motion;
        if !(m.view_timeline_inset.is_empty()
            && m.animation_range_start.is_empty()
            && m.animation_range_end.is_empty())
        {
            let mut motion = m.clone();
            for inset in &mut motion.view_timeline_inset {
                for side in [&mut inset.start, &mut inset.end] {
                    absolutize(side, vp, Length::Calc, |v| Length::Cells(cells_i32(v)));
                }
            }
            for boundary in motion
                .animation_range_start
                .iter_mut()
                .chain(&mut motion.animation_range_end)
            {
                if let crate::keyframes::RangeBoundary::Offset { offset, .. } = boundary {
                    absolutize(offset, vp, Length::Calc, |v| Length::Cells(cells_i32(v)));
                }
            }
            if motion != *self.motion {
                self.motion = motion.into();
            }
        }
        vp.reads.get()
    }
}
