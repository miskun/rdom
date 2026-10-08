//! The property subsets of the pseudo-elements that take only some
//! properties: `::first-line` (and `::placeholder`), `::first-letter`,
//! the highlight pseudo-elements (`::selection`, `::highlight()`) and
//! `::marker` (CSS Pseudo-Elements 4 §2.2.1, §2.3.1, §3.2, §4.3; CSS
//! Lists 3 §3.2). A rule for one is cut to its subset when it is built.

use super::{ImportantMask, TextDeclarations, TuiStyle};

impl TuiStyle {
    /// The background properties' bits (CSS Backgrounds 3 §3).
    const BACKGROUND: ImportantMask = ImportantMask::BG
        .union(ImportantMask::BACKGROUND_IMAGE)
        .union(ImportantMask::BACKGROUND_POSITION)
        .union(ImportantMask::BACKGROUND_SIZE)
        .union(ImportantMask::BACKGROUND_REPEAT)
        .union(ImportantMask::BACKGROUND_ATTACHMENT)
        .union(ImportantMask::BACKGROUND_ORIGIN)
        .union(ImportantMask::BACKGROUND_CLIP);

    /// The properties that apply to `::first-line` (CSS Pseudo-Elements 4
    /// §2.2.1) among those rdom has: the font properties, `color`,
    /// `opacity`, the background properties, the text decoration ones,
    /// and the typesetting properties that apply to inline boxes —
    /// `text-transform`, `letter-spacing`, `word-spacing`.
    const FIRST_LINE: ImportantMask = ImportantMask::FG
        .union(Self::BACKGROUND)
        .union(ImportantMask::FONT)
        .union(ImportantMask::TEXT_DECORATION)
        .union(ImportantMask::OPACITY)
        .union(ImportantMask::TEXT_TRANSFORM)
        .union(ImportantMask::LETTER_SPACING)
        .union(ImportantMask::WORD_SPACING);

    /// This block restricted to the properties that apply to
    /// `::first-line` — and so to `::placeholder` (CSS Pseudo-Elements 4
    /// §2.2.1, §4.3) — among those rdom has: `color`, the background
    /// properties, the font properties, the text decoration properties,
    /// `opacity`, `text-transform`, `letter-spacing`, `word-spacing`, and
    /// custom properties. Everything else is dropped with its
    /// `!important` bit. A declaration kept for the cascade (a `var()`
    /// value, CSS Variables 1 §3) stays when it sets one of them,
    /// restricted to them; none is flow-relative, so the subset keeps a
    /// declaration only while one waits for substitution.
    pub fn first_line_subset(&self) -> Self {
        let mut text = TextDeclarations::default();
        text.text_transform = self.text.text_transform;
        text.letter_spacing = self.text.letter_spacing.clone();
        text.word_spacing = self.text.word_spacing.clone();
        Self {
            fg: self.fg.clone(),
            bg: self.bg.clone(),
            background_image: self.background_image.clone(),
            background_position: self.background_position.clone(),
            background_size: self.background_size.clone(),
            background_repeat: self.background_repeat.clone(),
            background_attachment: self.background_attachment.clone(),
            background_origin: self.background_origin.clone(),
            background_clip: self.background_clip.clone(),
            font: self.font.clone(),
            text_decoration: self.text_decoration.clone(),
            opacity: self.opacity,
            text,
            ..self.restricted_to(Self::FIRST_LINE, false)
        }
    }

    /// This block restricted to the properties that apply to
    /// `::first-letter` (CSS Pseudo-Elements 4 §2.3.1) among those rdom
    /// has: the `::first-line` ones ([`first_line_subset`](Self::first_line_subset)),
    /// `line-height`, `vertical-align`, `float`, and the margins, padding
    /// and borders (their flow-relative forms kept, mapped by direction as
    /// any rule's are). `display`, sizes, `position` and every other
    /// property are dropped with their `!important` bits.
    pub fn first_letter_subset(&self) -> Self {
        let keep = Self::FIRST_LINE
            .union(ImportantMask::LINE_HEIGHT)
            .union(ImportantMask::VERTICAL_ALIGN)
            .union(ImportantMask::FLOAT)
            .union(Self::box_edges());
        let restricted = self.restricted_to(keep, true);
        let mut out = self.first_line_subset();
        out.text.line_height = self.text.line_height.clone();
        out.vertical_align = self.vertical_align.clone();
        out.float = self.float;
        out.margin = self.margin.clone();
        out.padding = self.padding.clone();
        out.border_style = self.border_style;
        out.border_width = self.border_width.clone();
        out.border_color = self.border_color.clone();
        out.border_radius = self.border_radius.clone();
        out.pending = restricted.pending;
        out.custom_properties = restricted.custom_properties;
        out.important = restricted.important;
        out
    }

    /// The margin, padding and border properties' bits.
    fn box_edges() -> ImportantMask {
        use ImportantMask as M;
        M::MARGIN_TOP
            .union(M::MARGIN_RIGHT)
            .union(M::MARGIN_BOTTOM)
            .union(M::MARGIN_LEFT)
            .union(M::PADDING_TOP)
            .union(M::PADDING_RIGHT)
            .union(M::PADDING_BOTTOM)
            .union(M::PADDING_LEFT)
            .union(M::BORDER_TOP_STYLE)
            .union(M::BORDER_RIGHT_STYLE)
            .union(M::BORDER_BOTTOM_STYLE)
            .union(M::BORDER_LEFT_STYLE)
            .union(M::BORDER_TOP_WIDTH)
            .union(M::BORDER_RIGHT_WIDTH)
            .union(M::BORDER_BOTTOM_WIDTH)
            .union(M::BORDER_LEFT_WIDTH)
            .union(M::BORDER_TOP_COLOR)
            .union(M::BORDER_RIGHT_COLOR)
            .union(M::BORDER_BOTTOM_COLOR)
            .union(M::BORDER_LEFT_COLOR)
            .union(M::BORDER_TOP_LEFT_RADIUS)
            .union(M::BORDER_TOP_RIGHT_RADIUS)
            .union(M::BORDER_BOTTOM_RIGHT_RADIUS)
            .union(M::BORDER_BOTTOM_LEFT_RADIUS)
    }

    /// This block restricted to the properties that apply to `::marker`
    /// (CSS Lists 3 §3.2, CSS Pseudo-Elements 4 §3.1.1) among those rdom
    /// has: `color`, the font properties, `white-space` (its two
    /// longhands), `content`, `direction`, the transition properties,
    /// and custom properties. Everything else — sizes, margins, padding,
    /// `display`, `position`, `text-transform` — is dropped with its
    /// `!important` bit; a kept `var()` declaration is restricted the
    /// same way.
    pub fn marker_subset(&self) -> Self {
        let keep = ImportantMask::FG
            | ImportantMask::FONT
            | ImportantMask::WHITE_SPACE
            | ImportantMask::CONTENT
            | ImportantMask::TEXT_DIRECTION
            | ImportantMask::TRANSITION_PROPERTY
            | ImportantMask::TRANSITION_DURATION
            | ImportantMask::TRANSITION_TIMING_FUNCTION
            | ImportantMask::TRANSITION_DELAY;
        let mut text = TextDeclarations::default();
        text.white_space_collapse = self.text.white_space_collapse;
        text.text_wrap_mode = self.text.text_wrap_mode;
        Self {
            fg: self.fg.clone(),
            font: self.font.clone(),
            text,
            content: self.content.clone(),
            text_direction: self.text_direction,
            transition_property: self.transition_property.clone(),
            transition_duration: self.transition_duration.clone(),
            transition_timing_function: self.transition_timing_function.clone(),
            transition_delay: self.transition_delay.clone(),
            ..self.restricted_to(keep, false)
        }
    }

    /// An empty block with this one's custom properties, the `keep` bits
    /// of its `!important` mask, and its kept declarations (CSS Variables
    /// 1 §3) that set a `keep` property — restricted to `keep` — while
    /// one waits for substitution, or always with `flow_relative` (a
    /// subset with flow-relative properties keeps them to map by
    /// direction); the subsets copy their fields over it.
    fn restricted_to(&self, keep: ImportantMask, flow_relative: bool) -> Self {
        let mut pending: Vec<crate::var::PendingDeclaration> = self
            .pending
            .iter()
            .filter(|d| {
                crate::property_dispatch::property_mask(&d.name).is_some_and(|m| m.intersects(keep))
            })
            .cloned()
            .map(|mut d| {
                let own = crate::property_dispatch::property_mask(&d.name).unwrap_or_default();
                if !keep.contains(own) {
                    d.restriction = crate::var::Restriction::Within(keep);
                }
                d
            })
            .collect();
        if !flow_relative && !pending.iter().any(|d| d.has_substitution) {
            pending.clear();
        }
        Self {
            pending,
            custom_properties: self.custom_properties.clone(),
            important: self.important & keep,
            ..Self::default()
        }
    }

    /// This block restricted to the properties that apply to the
    /// highlight pseudo-elements — `::selection`, `::highlight()` (CSS
    /// Pseudo-Elements 4 §3.2) — among those rdom has: `color`,
    /// `background-color`, the text decoration properties, and custom
    /// properties. Everything else (fonts, box properties, `opacity`) is
    /// dropped with its `!important` bit; none of them is flow-relative.
    pub fn highlight_subset(&self) -> Self {
        let keep = ImportantMask::FG
            .union(ImportantMask::BG)
            .union(ImportantMask::TEXT_DECORATION);
        Self {
            fg: self.fg.clone(),
            bg: self.bg.clone(),
            text_decoration: self.text_decoration.clone(),
            ..self.restricted_to(keep, false)
        }
    }
}
