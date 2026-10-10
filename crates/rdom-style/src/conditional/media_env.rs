//! What media queries are evaluated against: the viewport, the
//! preferred color scheme and the user's preferences — the environment
//! a backend describes (Media Queries 4 / 5 §4–§12).
//!
//! The values are a terminal's (rdom-tui's mapping, DIVERGENCES §2
//! "Media features in a terminal"): the viewport in cells, a grid device
//! whose colors are the terminal's color depth, that redraws fast, scripting enabled (the
//! event handlers are the scripts), a mouse — `hover` and a `fine`
//! pointer — unless the backend says otherwise; the color features read
//! the backend's [`ColorDepth`].

use crate::calc::Viewport;
use crate::color::{ColorDepth, ColorScheme};

/// The environment `@media` and `matchMedia` evaluate against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct MediaEnvironment {
    /// The viewport, in cells: `width` / `height` (Media Queries 4 §4.1–§4.2).
    pub viewport: Viewport,
    /// The document's preferred color scheme: `prefers-color-scheme`
    /// (Media Queries 5 §12.5).
    pub color_scheme: ColorScheme,
    /// The user's and the device's preferences.
    pub preferences: MediaPreferences,
    /// How many colors the terminal shows: `color`, `color-index` and
    /// `monochrome` (Media Queries 4 §6.1–§6.3). Default 24-bit.
    pub color_depth: ColorDepth,
    /// Whether the viewport is known: a backend that has none yet (a
    /// cascade before any layout or viewport) says so, and the size
    /// features — `width`, `height`, `aspect-ratio`, `orientation` and
    /// their `device-*` forms — are then unknown rather than measured
    /// against 0 × 0. `new` makes it known; `Default` does not.
    pub viewport_known: bool,
}

impl MediaEnvironment {
    /// `viewport` with the `color_scheme` and `preferences` given.
    pub fn new(
        viewport: Viewport,
        color_scheme: ColorScheme,
        preferences: MediaPreferences,
    ) -> Self {
        MediaEnvironment {
            viewport,
            color_scheme,
            preferences,
            color_depth: ColorDepth::TrueColor,
            viewport_known: true,
        }
    }

    /// The same environment on a terminal of `depth`.
    pub fn with_color_depth(mut self, depth: ColorDepth) -> Self {
        self.color_depth = depth;
        self
    }

    /// The same environment with no known viewport (`viewport_known`).
    pub fn without_viewport(mut self) -> Self {
        self.viewport_known = false;
        self
    }
}

/// The preferences and device facts a backend reports to media queries
/// beyond the viewport and the color scheme. The defaults are a
/// terminal's with a mouse and no stated preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MediaPreferences {
    /// `prefers-reduced-motion: reduce` (Media Queries 5 §12.1);
    /// default `false` (`no-preference`).
    pub reduced_motion: bool,
    /// `prefers-reduced-transparency: reduce` (§12.2); default `false`.
    pub reduced_transparency: bool,
    /// `prefers-contrast` (§12.3); default `no-preference`.
    pub contrast: Contrast,
    /// `prefers-reduced-data: reduce` (§12.7); default `false`.
    pub reduced_data: bool,
    /// `forced-colors: active` (§12.4); default `false` (`none`).
    pub forced_colors: bool,
    /// `inverted-colors: inverted` (§12.6); default `false` (`none`).
    pub inverted_colors: bool,
    /// `hover` / `any-hover` (Media Queries 4 §7.2): the pointer can
    /// hover. Default `true` — a terminal app reports mouse motion.
    pub hover: bool,
    /// `pointer` / `any-pointer` (§7.1); default `fine`.
    pub pointer: PointerAccuracy,
}

impl Default for MediaPreferences {
    fn default() -> Self {
        MediaPreferences {
            reduced_motion: false,
            reduced_transparency: false,
            contrast: Contrast::NoPreference,
            reduced_data: false,
            forced_colors: false,
            inverted_colors: false,
            hover: true,
            pointer: PointerAccuracy::Fine,
        }
    }
}

impl MediaPreferences {
    /// The defaults: a terminal with a mouse, no stated preference.
    pub fn new() -> Self {
        Self::default()
    }

    /// `prefers-reduced-motion: reduce` when `reduce`.
    pub fn with_reduced_motion(mut self, reduce: bool) -> Self {
        self.reduced_motion = reduce;
        self
    }

    /// `prefers-reduced-transparency: reduce` when `reduce`.
    pub fn with_reduced_transparency(mut self, reduce: bool) -> Self {
        self.reduced_transparency = reduce;
        self
    }

    /// `prefers-contrast`.
    pub fn with_contrast(mut self, contrast: Contrast) -> Self {
        self.contrast = contrast;
        self
    }

    /// `prefers-reduced-data: reduce` when `reduce`.
    pub fn with_reduced_data(mut self, reduce: bool) -> Self {
        self.reduced_data = reduce;
        self
    }

    /// `forced-colors: active` when `active`.
    pub fn with_forced_colors(mut self, active: bool) -> Self {
        self.forced_colors = active;
        self
    }

    /// `inverted-colors: inverted` when `inverted`.
    pub fn with_inverted_colors(mut self, inverted: bool) -> Self {
        self.inverted_colors = inverted;
        self
    }

    /// The pointing device: `hover` and its `pointer` accuracy — an app
    /// that runs without mouse reporting passes `(false,
    /// PointerAccuracy::None)`.
    pub fn with_pointer(mut self, hover: bool, pointer: PointerAccuracy) -> Self {
        self.hover = hover;
        self.pointer = pointer;
        self
    }
}

/// `prefers-contrast` (Media Queries 5 §12.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Contrast {
    #[default]
    NoPreference,
    More,
    Less,
    Custom,
}

/// `pointer` (Media Queries 4 §7.1): the primary pointing device's
/// accuracy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PointerAccuracy {
    /// No pointing device.
    None,
    /// Limited accuracy (a touchscreen).
    Coarse,
    /// An accurate pointing device (a mouse).
    #[default]
    Fine,
}
