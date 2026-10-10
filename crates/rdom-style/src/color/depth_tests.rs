//! C16G-COLOR-DEPTH: detection, the media numbers and quantization.

use super::*;

fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let vars: Vec<(String, String)> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |name| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
}

/// `COLORTERM=truecolor|24bit` → 24-bit; `TERM=*-256color` → 256; else
/// 16; `NO_COLOR` non-empty → none, whatever the rest says.
#[test]
fn detection_follows_the_environment() {
    use ColorDepth::*;
    let cases: [(&[(&str, &str)], ColorDepth); 9] = [
        (
            &[("COLORTERM", "truecolor"), ("TERM", "xterm-256color")],
            TrueColor,
        ),
        (&[("COLORTERM", "24bit")], TrueColor),
        (&[("TERM", "xterm-256color")], Ansi256),
        (&[("TERM", "tmux-256color"), ("COLORTERM", "yes")], Ansi256),
        (&[("TERM", "xterm")], Ansi16),
        (&[], Ansi16),
        (&[("NO_COLOR", "1"), ("COLORTERM", "truecolor")], NoColor),
        (&[("NO_COLOR", ""), ("TERM", "xterm-256color")], Ansi256),
        (&[("WT_SESSION", "f00")], TrueColor),
    ];
    for (vars, want) in cases {
        assert_eq!(ColorDepth::detect(env(vars)), want, "{vars:?}");
    }
}

/// Media Queries 4 §6.1–§6.2.
#[test]
fn the_media_numbers() {
    use ColorDepth::*;
    let got: Vec<(u8, u32)> = [TrueColor, Ansi256, Ansi16, NoColor]
        .map(|d| (d.bits(), d.index_entries()))
        .to_vec();
    assert_eq!(got, [(8, 0), (2, 256), (1, 16), (0, 0)]);
}

/// A palette color quantizes to itself; between entries, the nearest.
#[test]
fn quantizing_to_256_takes_the_cube_and_the_ramp() {
    let q = |r, g, b| ColorDepth::Ansi256.quantize(Color::Rgb(r, g, b));
    assert_eq!(q(255, 135, 0), Color::Indexed(208));
    assert_eq!(q(0, 0, 95), Color::Indexed(17));
    assert_eq!(q(128, 128, 128), Color::Indexed(244), "the gray ramp");
    assert_eq!(q(250, 130, 5), Color::Indexed(208));
    assert_eq!(
        q(0, 0, 0),
        Color::Indexed(16),
        "black is the cube's, not theme 0"
    );
    for n in 16..=255u8 {
        let (r, g, b) = xterm_rgb(n);
        let back = ColorDepth::Ansi256.quantize(Color::Rgb(r, g, b));
        assert_eq!(
            super::super::palette::xterm_rgb(match back {
                Color::Indexed(i) => i,
                other => panic!("{other:?}"),
            }),
            (r, g, b),
            "index {n}"
        );
    }
}

/// 16 colors: the nearest of xterm's sixteen; a palette index past 15
/// too.
#[test]
fn quantizing_to_16() {
    let q = |c| ColorDepth::Ansi16.quantize(c);
    assert_eq!(q(Color::Rgb(250, 10, 10)), Color::Indexed(9));
    assert_eq!(q(Color::Rgb(0, 0, 230)), Color::Indexed(4));
    assert_eq!(q(Color::Indexed(196)), Color::Indexed(9));
    assert_eq!(q(Color::Indexed(3)), Color::Indexed(3));
    assert_eq!(q(Color::Reset), Color::Reset);
}

/// No color: everything is the default; 24-bit: unchanged, opaque.
#[test]
fn no_color_and_truecolor() {
    assert_eq!(
        ColorDepth::NoColor.quantize(Color::Rgb(1, 2, 3)),
        Color::Reset
    );
    assert_eq!(
        ColorDepth::NoColor.quantize(Color::Indexed(9)),
        Color::Reset
    );
    assert_eq!(
        ColorDepth::TrueColor.quantize(Color::Rgba(1, 2, 3, 4)),
        Color::Rgb(1, 2, 3)
    );
}
