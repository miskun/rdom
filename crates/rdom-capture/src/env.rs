//! The deterministic environment a session runs an app in
//! (`specs/CAPTURE.md` §Deterministic environment).
//!
//! A capture is a function of the app build, the fixture, this
//! environment, the input log, the edits and the virtual time. Everything
//! an `App` would otherwise read from the terminal or the process — size,
//! color depth, the SGR extensions, the color scheme, the media
//! preferences — is set here and never probed. The clipboard and the URL
//! opener are in-memory, so a session never touches the user's system.
//!
//! [`Environment::app`] builds an `App` with every setting applied; the
//! session applies them again after
//! [`CaptureApp::build`](crate::CaptureApp::build) returns ([`Environment::enforce`]), so an
//! app that sets one of them itself is overridden, not trusted.

use std::io;
use std::rc::Rc;

use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::selection::clipboard::MemoryClipboard;
use rdom_tui::runtime::url_opener::MemoryUrlOpener;
use rdom_tui::{
    App, ColorDepth, ColorScheme, Contrast, MediaPreferences, SgrCapabilities, Stylesheet, TuiDom,
};
use serde::{Deserialize, Serialize};

/// The environment a session runs an app in. Every field has a default,
/// so a request names only what it changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct Environment {
    /// The terminal's size in cells.
    pub viewport: Viewport,
    /// How many colors the terminal shows.
    pub color_depth: Depth,
    /// The color scheme the terminal reports.
    pub color_scheme: Scheme,
    /// The SGR extensions the terminal reads.
    pub sgr: Sgr,
    /// `prefers-reduced-motion: reduce`.
    pub reduced_motion: bool,
    /// `prefers-reduced-transparency: reduce`.
    pub reduced_transparency: bool,
    /// `prefers-contrast`.
    pub contrast: ContrastPreference,
    /// `forced-colors: active`.
    pub forced_colors: bool,
    /// Frames per second while anything animates.
    pub animation_fps: u16,
}

impl Default for Environment {
    fn default() -> Self {
        Environment {
            viewport: Viewport {
                width: 120,
                height: 40,
            },
            color_depth: Depth::TrueColor,
            color_scheme: Scheme::Dark,
            sgr: Sgr::Extended,
            reduced_motion: false,
            reduced_transparency: false,
            contrast: ContrastPreference::NoPreference,
            forced_colors: false,
            animation_fps: 60,
        }
    }
}

/// A terminal size in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Viewport {
    pub width: u16,
    pub height: u16,
}

/// How many colors the terminal shows ([`ColorDepth`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Depth {
    #[serde(rename = "truecolor")]
    TrueColor,
    #[serde(rename = "256")]
    Ansi256,
    #[serde(rename = "16")]
    Ansi16,
    #[serde(rename = "none")]
    NoColor,
}

impl Depth {
    /// The runtime's depth.
    pub fn color_depth(self) -> ColorDepth {
        match self {
            Depth::TrueColor => ColorDepth::TrueColor,
            Depth::Ansi256 => ColorDepth::Ansi256,
            Depth::Ansi16 => ColorDepth::Ansi16,
            Depth::NoColor => ColorDepth::NoColor,
        }
    }
}

/// The color scheme the terminal reports ([`ColorScheme`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    Dark,
    Light,
}

impl Scheme {
    /// The runtime's scheme.
    pub fn color_scheme(self) -> ColorScheme {
        match self {
            Scheme::Dark => ColorScheme::Dark,
            Scheme::Light => ColorScheme::Light,
        }
    }
}

/// The SGR extensions the terminal reads ([`SgrCapabilities`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Sgr {
    /// ECMA-48's common subset.
    Basic,
    /// The underline styles and color, and the overline.
    Extended,
}

impl Sgr {
    /// The runtime's capabilities.
    pub fn capabilities(self) -> SgrCapabilities {
        match self {
            Sgr::Basic => SgrCapabilities::BASIC,
            Sgr::Extended => SgrCapabilities::EXTENDED,
        }
    }
}

/// `prefers-contrast` ([`Contrast`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ContrastPreference {
    NoPreference,
    More,
    Less,
    Custom,
}

impl ContrastPreference {
    /// The runtime's preference.
    pub fn contrast(self) -> Contrast {
        match self {
            ContrastPreference::NoPreference => Contrast::NoPreference,
            ContrastPreference::More => Contrast::More,
            ContrastPreference::Less => Contrast::Less,
            ContrastPreference::Custom => Contrast::Custom,
        }
    }
}

impl Environment {
    /// Whether every value is one a session can run: a viewport of at
    /// least one cell each way and a frame rate above zero.
    pub fn validate(&self) -> Result<(), String> {
        if self.viewport.width == 0 || self.viewport.height == 0 {
            return Err(format!(
                "viewport must be at least 1×1, got {}×{}",
                self.viewport.width, self.viewport.height
            ));
        }
        if self.animation_fps == 0 {
            return Err("animation_fps must be above 0".to_string());
        }
        Ok(())
    }

    /// The media preferences this environment reports.
    pub fn media_preferences(&self) -> MediaPreferences {
        MediaPreferences::new()
            .with_reduced_motion(self.reduced_motion)
            .with_reduced_transparency(self.reduced_transparency)
            .with_contrast(self.contrast.contrast())
            .with_forced_colors(self.forced_colors)
    }

    /// A headless `App` over `dom` and `stylesheet`, sized to the
    /// viewport, with every setting of this environment applied.
    pub fn app(&self, dom: TuiDom, stylesheet: Stylesheet) -> io::Result<App<TestBackend>> {
        let backend = TestBackend::new(self.viewport.width, self.viewport.height);
        let app = App::with_backend(dom, stylesheet, Terminal::new(backend)?)?;
        Ok(self.enforce(app))
    }

    /// `app` with every setting of this environment applied, whatever the
    /// app set itself: the color depth, the SGR extensions, the color
    /// scheme, the media preferences, the frame rate, a steady caret, an
    /// in-memory clipboard and URL opener. The viewport is checked, not
    /// set: see [`Environment::check_viewport`].
    pub fn enforce(&self, app: App<TestBackend>) -> App<TestBackend> {
        let mut app = app
            .with_sgr_capabilities(self.sgr.capabilities())
            .with_color_depth(self.color_depth.color_depth())
            .with_color_scheme(self.color_scheme.color_scheme())
            .with_media_preferences(self.media_preferences())
            .with_animation_frame_rate(self.animation_fps)
            .with_caret_blink(None)
            .with_clipboard(Box::new(MemoryClipboard::new()))
            .with_url_opener(Rc::new(MemoryUrlOpener::new()));
        app.terminal_mut().backend_mut().clear_bytes();
        app
    }

    /// An error when `app`'s terminal is not the viewport's size: an app
    /// built its own backend at another size.
    pub fn check_viewport(&self, app: &App<TestBackend>) -> Result<(), String> {
        let size = app.terminal().size();
        if (size.width, size.height) != (self.viewport.width, self.viewport.height) {
            return Err(format!(
                "the app's terminal is {}×{}, the environment asks for {}×{}; build it with Environment::app",
                size.width, size.height, self.viewport.width, self.viewport.height
            ));
        }
        Ok(())
    }
}
