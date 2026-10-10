//! The acid reference format (`specs/ACID.md` ground rules 1–2).
//!
//! A [`Reference`] is what the spec says one tile shows, cell by cell,
//! written by hand from the cited sections — never recorded from rdom.
//!
//! ## Grid
//!
//! [`Reference::grid`] is the tile's rows, top to bottom, each as **two
//! lines**: the glyphs, then one legend letter per column naming the
//! cell's style. Both lines sit between `|` bars so trailing blanks are
//! visible; anything outside the bars (indentation, a note after the
//! closing bar) is ignored, and so are lines with no bar.
//!
//! ```text
//! |ua late-sheet          |
//! |gg.gggggggggg..........|
//! ```
//!
//! A two-cell glyph (CJK, most emoji) takes two columns of the style line
//! and one character of the glyph line.
//!
//! ## Legend
//!
//! [`Reference::legend`] maps a letter to a glyph-agnostic style:
//! whitespace-separated words, every one optional —
//!
//! - `fg <color>` / `bg <color>` — the cell's colours (`#rrggbb`, or
//!   `default`, the terminal's own — SGR 39 / 49); `default` if absent;
//! - `ul <color>` — the underline's own colour (SGR 58); the glyph's if
//!   absent;
//! - modifiers: `bold`, `italic`, `underline` (single), `double`,
//!   `curly`, `dotted`, `dashed` (an underline in that style), `overline`,
//!   `strike`, `blink`.
//!
//! `.` is predefined as the all-default style and may not be redefined.
//!
//! ## What a blank cell compares
//!
//! A space shows no foreground and no weight or slant, so on a cell whose
//! expected glyph is a space the comparison skips `fg`, `bold`, `italic`
//! and `blink` — unless the cell carries a line decoration drawn in the
//! foreground: an overline, a strike, or an underline with no colour of
//! its own (an underline in an `ul` colour is drawn in that colour, so
//! the blank's foreground stays unseen). Background, decorations and the
//! underline colour are always compared.

use rdom_tui::{Color, Modifier};
use unicode_width::UnicodeWidthStr;

/// One tile's hand-derived reference.
pub struct Reference {
    /// The tile's id (`Tile::id`).
    pub tile: &'static str,
    /// The spec sections the derivation follows, printed with a failure.
    pub spec: &'static [&'static str],
    /// Letter → style ([module docs](self)).
    pub legend: &'static [(char, &'static str)],
    /// Glyph line, style line, per row ([module docs](self)).
    pub grid: &'static str,
}

/// A legend entry, parsed.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub underline_color: Color,
    pub modifier: Modifier,
}

impl Style {
    pub const DEFAULT: Style = Style {
        fg: Color::Reset,
        bg: Color::Reset,
        underline_color: Color::Reset,
        modifier: Modifier::empty(),
    };
}

/// One expected cell.
#[derive(Clone, Debug)]
pub struct Expected {
    /// The glyph; `""` for the second column of a two-cell glyph.
    pub glyph: String,
    pub style: Style,
}

/// A parsed reference: `rows[y][x]`.
pub struct Parsed {
    pub rows: Vec<Vec<Expected>>,
}

const LINES: Modifier = Modifier::from_bits_truncate(
    Modifier::UNDERLINED.bits()
        | Modifier::UNDERLINE_DOUBLE.bits()
        | Modifier::UNDERLINE_CURLY.bits()
        | Modifier::UNDERLINE_DOTTED.bits()
        | Modifier::UNDERLINE_DASHED.bits()
        | Modifier::OVERLINED.bits()
        | Modifier::CROSSED_OUT.bits(),
);

/// Whether a cell whose glyph is `glyph` shows `style`'s foreground: a
/// non-blank glyph does, a blank one only through a line decoration drawn
/// in it — an overline or a strike, or an underline without a colour of
/// its own (`underline_color` `Reset`; SGR 58 draws it in that colour).
pub fn shows_fg(glyph: &str, modifier: Modifier, underline_color: Color) -> bool {
    let own_color = underline_color != Color::Reset;
    let in_fg = if own_color {
        Modifier::OVERLINED | Modifier::CROSSED_OUT
    } else {
        LINES
    };
    glyph != " " || !(modifier & in_fg).is_empty()
}

/// The modifier bits a cell with `glyph` shows.
pub fn visible_modifiers(glyph: &str, modifier: Modifier) -> Modifier {
    if glyph == " " {
        modifier & LINES
    } else {
        modifier
    }
}

fn color(word: &str) -> Result<Color, String> {
    if word == "default" {
        return Ok(Color::Reset);
    }
    let hex = word
        .strip_prefix('#')
        .filter(|h| h.len() == 6)
        .ok_or_else(|| format!("colour {word:?}: want #rrggbb or default"))?;
    let byte = |i: usize| {
        u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| format!("colour {word:?}: {e}"))
    };
    Ok(Color::Rgb(byte(0)?, byte(2)?, byte(4)?))
}

/// Parse a legend entry ([module docs](self)).
pub fn parse_style(text: &str) -> Result<Style, String> {
    let mut style = Style::DEFAULT;
    let mut words = text.split_whitespace();
    while let Some(word) = words.next() {
        let mut next = || {
            words
                .next()
                .ok_or_else(|| format!("{word:?} wants a colour"))
        };
        match word {
            "fg" => style.fg = color(next()?)?,
            "bg" => style.bg = color(next()?)?,
            "ul" => style.underline_color = color(next()?)?,
            "bold" => style.modifier |= Modifier::BOLD,
            "italic" => style.modifier |= Modifier::ITALIC,
            "underline" => style.modifier |= Modifier::UNDERLINED,
            "double" => style.modifier |= Modifier::UNDERLINED | Modifier::UNDERLINE_DOUBLE,
            "curly" => style.modifier |= Modifier::UNDERLINED | Modifier::UNDERLINE_CURLY,
            "dotted" => style.modifier |= Modifier::UNDERLINED | Modifier::UNDERLINE_DOTTED,
            "dashed" => style.modifier |= Modifier::UNDERLINED | Modifier::UNDERLINE_DASHED,
            "overline" => style.modifier |= Modifier::OVERLINED,
            "strike" => style.modifier |= Modifier::CROSSED_OUT,
            "blink" => style.modifier |= Modifier::SLOW_BLINK,
            other => return Err(format!("unknown legend word {other:?}")),
        }
    }
    Ok(style)
}

/// The text between a line's first and last `|`, or `None` for a line
/// with no bars.
fn barred(line: &str) -> Option<&str> {
    let start = line.find('|')?;
    let end = line.rfind('|')?;
    (end > start).then(|| &line[start + 1..end])
}

impl Reference {
    /// Parse the grid against a `w` × `h` tile.
    pub fn parse(&self, w: u16, h: u16) -> Result<Parsed, String> {
        let mut legend: Vec<(char, Style)> = vec![('.', Style::DEFAULT)];
        for &(letter, text) in self.legend {
            if legend.iter().any(|(l, _)| *l == letter) {
                return Err(format!("legend letter {letter:?} defined twice"));
            }
            let style = parse_style(text).map_err(|e| format!("legend {letter:?}: {e}"))?;
            legend.push((letter, style));
        }
        let lines: Vec<&str> = self.grid.lines().filter_map(barred).collect();
        if lines.len() != 2 * usize::from(h) {
            return Err(format!(
                "grid has {} barred lines, want {} (glyph + style for {h} rows)",
                lines.len(),
                2 * h
            ));
        }
        let mut rows = Vec::new();
        for (y, pair) in lines.chunks(2).enumerate() {
            let (glyphs, styles) = (pair[0], pair[1]);
            let mut cells = Vec::new();
            for g in glyphs.chars() {
                let s = g.to_string();
                let width = UnicodeWidthStr::width(s.as_str()).max(1);
                cells.push(s);
                if width == 2 {
                    cells.push(String::new());
                }
            }
            let letters: Vec<char> = styles.chars().collect();
            if cells.len() != usize::from(w) || letters.len() != usize::from(w) {
                return Err(format!(
                    "row {y}: {} glyph columns and {} style letters, want {w}",
                    cells.len(),
                    letters.len()
                ));
            }
            let mut row = Vec::new();
            for (glyph, letter) in cells.into_iter().zip(letters) {
                let style = legend
                    .iter()
                    .find(|(l, _)| *l == letter)
                    .map(|(_, s)| *s)
                    .ok_or_else(|| format!("row {y}: style letter {letter:?} not in the legend"))?;
                row.push(Expected { glyph, style });
            }
            rows.push(row);
        }
        Ok(Parsed { rows })
    }
}

/// `style` as legend words — how a failure prints a cell's style.
pub fn describe(style: &Style) -> String {
    let hex = |c: Color| match c {
        Color::Reset => "default".to_string(),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        other => format!("{other:?}"),
    };
    let mut out = format!("fg {} bg {}", hex(style.fg), hex(style.bg));
    if style.underline_color != Color::Reset {
        out.push_str(&format!(" ul {}", hex(style.underline_color)));
    }
    let m = style.modifier;
    for (bit, word) in [
        (Modifier::BOLD, "bold"),
        (Modifier::ITALIC, "italic"),
        (Modifier::UNDERLINE_DOUBLE, "double"),
        (Modifier::UNDERLINE_CURLY, "curly"),
        (Modifier::UNDERLINE_DOTTED, "dotted"),
        (Modifier::UNDERLINE_DASHED, "dashed"),
        (Modifier::OVERLINED, "overline"),
        (Modifier::CROSSED_OUT, "strike"),
        (Modifier::SLOW_BLINK, "blink"),
        (Modifier::HIDDEN, "hidden"),
    ] {
        if m.contains(bit) {
            out.push(' ');
            out.push_str(word);
        }
    }
    let styled = Modifier::UNDERLINE_DOUBLE
        | Modifier::UNDERLINE_CURLY
        | Modifier::UNDERLINE_DOTTED
        | Modifier::UNDERLINE_DASHED;
    if m.contains(Modifier::UNDERLINED) && (m & styled).is_empty() {
        out.push_str(" underline");
    }
    out
}

/// The format's own cases.
#[test]
fn reference_format_parses_its_documented_forms() {
    let style = parse_style("fg #ff0000 bg default bold curly ul #0000ff").unwrap();
    assert_eq!(style.fg, Color::Rgb(255, 0, 0));
    assert_eq!(style.bg, Color::Reset);
    assert_eq!(style.underline_color, Color::Rgb(0, 0, 255));
    let bits = Modifier::BOLD | Modifier::UNDERLINED | Modifier::UNDERLINE_CURLY;
    assert!(style.modifier.contains(bits));
    assert!(parse_style("fg red").is_err());
    let r = Reference {
        tile: "t",
        spec: &[],
        legend: &[('a', "fg #00a000")],
        grid: "  |中a |  note\n  |..a.|\n",
    };
    let parsed = r.parse(4, 1).unwrap();
    assert_eq!(parsed.rows[0][0].glyph, "中");
    assert_eq!(parsed.rows[0][1].glyph, "");
    assert_eq!(parsed.rows[0][2].style.fg, Color::Rgb(0, 160, 0));
    assert!(r.parse(5, 1).is_err());
    assert!(shows_fg("x", Modifier::empty(), Color::Reset));
    assert!(!shows_fg(" ", Modifier::BOLD, Color::Reset));
    assert!(shows_fg(" ", Modifier::UNDERLINED, Color::Reset));
    let red = Color::Rgb(255, 0, 0);
    assert!(
        !shows_fg(" ", Modifier::UNDERLINED, red),
        "drawn in its own colour"
    );
    assert!(shows_fg(
        " ",
        Modifier::UNDERLINED | Modifier::CROSSED_OUT,
        red
    ));
}
