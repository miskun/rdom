//! The compositing values (Compositing and Blending 1 §3, §5, §10;
//! Compositing 2): `mix-blend-mode`, `isolation`, `background-blend-mode`,
//! and the blend functions on opaque sRGB colors.

/// A `<blend-mode>` (Compositing 1 §10), or Compositing 2's
/// `plus-darker` / `plus-lighter` (which `mix-blend-mode` takes).
///
/// Open (DESIGN, `#[non_exhaustive]`): Compositing 2 grew it, and a
/// reader that meets a mode it does not know can composite normally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum BlendMode {
    /// The initial value: the source color.
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    PlusDarker,
    PlusLighter,
}

impl BlendMode {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, BlendMode)] = &[
        ("normal", BlendMode::Normal),
        ("multiply", BlendMode::Multiply),
        ("screen", BlendMode::Screen),
        ("overlay", BlendMode::Overlay),
        ("darken", BlendMode::Darken),
        ("lighten", BlendMode::Lighten),
        ("color-dodge", BlendMode::ColorDodge),
        ("color-burn", BlendMode::ColorBurn),
        ("hard-light", BlendMode::HardLight),
        ("soft-light", BlendMode::SoftLight),
        ("difference", BlendMode::Difference),
        ("exclusion", BlendMode::Exclusion),
        ("hue", BlendMode::Hue),
        ("saturation", BlendMode::Saturation),
        ("color", BlendMode::Color),
        ("luminosity", BlendMode::Luminosity),
        ("plus-darker", BlendMode::PlusDarker),
        ("plus-lighter", BlendMode::PlusLighter),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, m)| *m == self)
            .map_or("normal", |(k, _)| k)
    }

    /// Whether it is one of §10's `<blend-mode>`s (not `plus-darker` /
    /// `plus-lighter`, which `background-blend-mode` does not take).
    pub fn is_blend_mode(self) -> bool {
        !matches!(self, BlendMode::PlusDarker | BlendMode::PlusLighter)
    }

    /// `B(Cb, Cs)` (§10) of the opaque sRGB backdrop `cb` and source `cs`,
    /// rounded once to 8-bit channels.
    pub fn blend(self, cb: (u8, u8, u8), cs: (u8, u8, u8)) -> (u8, u8, u8) {
        let unit = |(r, g, b): (u8, u8, u8)| [r, g, b].map(|v| f64::from(v) / 255.0);
        let (b, s) = (unit(cb), unit(cs));
        let out = match self {
            BlendMode::Hue => set_lum(set_sat(s, sat(b)), lum(b)),
            BlendMode::Saturation => set_lum(set_sat(b, sat(s)), lum(b)),
            BlendMode::Color => set_lum(s, lum(b)),
            BlendMode::Luminosity => set_lum(b, lum(s)),
            separable => [0, 1, 2].map(|i| separable.channel(b[i], s[i])),
        };
        let [r, g, b] = out.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        (r, g, b)
    }

    /// A separable mode on one channel (§10.1).
    fn channel(self, b: f64, s: f64) -> f64 {
        let multiply = |b: f64, s: f64| b * s;
        let screen = |b: f64, s: f64| b + s - b * s;
        let hard_light = |b: f64, s: f64| {
            if s <= 0.5 {
                multiply(b, 2.0 * s)
            } else {
                screen(b, 2.0 * s - 1.0)
            }
        };
        match self {
            BlendMode::Multiply => multiply(b, s),
            BlendMode::Screen => screen(b, s),
            BlendMode::Overlay => hard_light(s, b),
            BlendMode::Darken => b.min(s),
            BlendMode::Lighten => b.max(s),
            BlendMode::ColorDodge if b == 0.0 => 0.0,
            BlendMode::ColorDodge if s >= 1.0 => 1.0,
            BlendMode::ColorDodge => (b / (1.0 - s)).min(1.0),
            BlendMode::ColorBurn if b >= 1.0 => 1.0,
            BlendMode::ColorBurn if s == 0.0 => 0.0,
            BlendMode::ColorBurn => 1.0 - ((1.0 - b) / s).min(1.0),
            BlendMode::HardLight => hard_light(b, s),
            BlendMode::SoftLight if s <= 0.5 => b - (1.0 - 2.0 * s) * b * (1.0 - b),
            BlendMode::SoftLight => {
                let d = if b <= 0.25 {
                    ((16.0 * b - 12.0) * b + 4.0) * b
                } else {
                    b.sqrt()
                };
                b + (2.0 * s - 1.0) * (d - b)
            }
            BlendMode::Difference => (b - s).abs(),
            BlendMode::Exclusion => b + s - 2.0 * b * s,
            // Compositing 2 §9: the sum, clamped; and its dual.
            BlendMode::PlusLighter => (b + s).min(1.0),
            BlendMode::PlusDarker => (b + s - 1.0).max(0.0),
            _ => s,
        }
    }
}

/// §10.2's luminosity.
fn lum(c: [f64; 3]) -> f64 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

/// §10.2 `ClipColor`.
fn clip_color(c: [f64; 3]) -> [f64; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut c = c;
    if n < 0.0 {
        c = c.map(|v| l + (v - l) * l / (l - n));
    }
    if x > 1.0 {
        c = c.map(|v| l + (v - l) * (1.0 - l) / (x - l));
    }
    c
}

/// §10.2 `SetLum`.
fn set_lum(c: [f64; 3], l: f64) -> [f64; 3] {
    let d = l - lum(c);
    clip_color(c.map(|v| v + d))
}

/// §10.2 `Sat`.
fn sat(c: [f64; 3]) -> f64 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

/// §10.2 `SetSat`.
fn set_sat(c: [f64; 3], s: f64) -> [f64; 3] {
    let mut idx = [0usize, 1, 2];
    idx.sort_by(|&a, &b| c[a].total_cmp(&c[b]));
    let [min, mid, max] = idx;
    let mut out = [0.0; 3];
    if c[max] > c[min] {
        out[mid] = (c[mid] - c[min]) * s / (c[max] - c[min]);
        out[max] = s;
    }
    out
}

/// `isolation` (Compositing 1 §5.2).
///
/// Closed (DESIGN): the property's two values, each a group the painter
/// must make or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Isolation {
    #[default]
    Auto,
    Isolate,
}

impl Isolation {
    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        match self {
            Isolation::Auto => "auto",
            Isolation::Isolate => "isolate",
        }
    }
}
