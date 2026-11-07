//! A film as an asciicast v2 recording (`specs/CAPTURE.md` §Views,
//! `cast`): a header line, then one `[seconds, "o", text]` event per
//! frame. Every event after the first is the bytes rdom sent the terminal
//! for that frame; the first is the whole first frame, encoded from its
//! grid (cursor positions and SGR per run), since what rdom sent before
//! it assumed a screen the recording does not have. No timestamp in the
//! header: the same film is the same file.

use std::fmt::Write as _;

use super::Film;
use crate::capture::{Attr, ColorValue, Grid};

/// `grid` as the bytes that draw it on a cleared terminal.
pub(crate) fn whole_frame(grid: &Grid) -> Vec<u8> {
    let mut out = String::from("\x1b[0m\x1b[2J");
    for (y, runs) in grid.rows.iter().enumerate() {
        let _ = write!(out, "\x1b[{};1H", y + 1);
        for r in runs {
            let (fg, bg) = r.shown_colors();
            out.push_str("\x1b[0");
            sgr_color(&mut out, fg, 38);
            sgr_color(&mut out, bg, 48);
            for (a, code) in [
                (Attr::Bold, "1"),
                (Attr::Italic, "3"),
                (Attr::Underline, "4"),
                (Attr::BlinkSlow, "5"),
                (Attr::Hidden, "8"),
                (Attr::Strikethrough, "9"),
                (Attr::Overline, "53"),
            ] {
                if r.mods.contains(&a) {
                    out.push(';');
                    out.push_str(code);
                }
            }
            out.push('m');
            out.push_str(&r.text);
        }
    }
    out.push_str("\x1b[0m");
    out.into_bytes()
}

fn sgr_color(out: &mut String, c: ColorValue, base: u8) {
    match c {
        ColorValue::Default => {}
        ColorValue::Rgb(r, g, b) | ColorValue::Rgba(r, g, b, _) => {
            let _ = write!(out, ";{base};2;{r};{g};{b}");
        }
        ColorValue::Ansi(n) => {
            let _ = write!(out, ";{base};5;{n}");
        }
    }
}

/// `film` of a `width` × `height` terminal as an asciicast v2 file.
pub fn asciicast(film: &Film, width: u16, height: u16) -> String {
    let mut out = serde_json::json!({"version": 2, "width": width, "height": height}).to_string();
    out.push('\n');
    for (frame, bytes) in film.frames.iter().zip(&film.sent) {
        if bytes.is_empty() && frame.t_ms > 0 {
            continue;
        }
        let event = serde_json::json!([
            frame.t_ms as f64 / 1000.0,
            "o",
            String::from_utf8_lossy(bytes)
        ]);
        out.push_str(&event.to_string());
        out.push('\n');
    }
    out
}
