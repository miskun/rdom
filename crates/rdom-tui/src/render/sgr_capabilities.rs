//! [`SgrCapabilities`] — which SGR decorations beyond ECMA-48's common
//! subset a terminal understands, so the backend emits them only where
//! they are read right (CSS Text Decoration 4's underline styles, its
//! underline color, the overline).

/// The SGR extensions a terminal understands. A backend emits a
/// decoration only to a terminal that has it: an underline style degrades
/// to a plain underline, and an underline color or overline is left out.
/// The colon sub-parameters of `4:n` and `58:2::r:g:b` are the risk — a
/// terminal that does not know them may read the numbers as separate
/// codes (`4:3` as underline and italic) — so the default is
/// [`BASIC`](Self::BASIC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct SgrCapabilities {
    /// The underline styles `4:1`–`4:5` (single, double, curly, dotted,
    /// dashed).
    pub styled_underline: bool,
    /// The underline color, SGR 58 / 59.
    pub underline_color: bool,
    /// The overline, SGR 53 / 55.
    pub overline: bool,
}

impl SgrCapabilities {
    /// ECMA-48's common subset: none of the extensions.
    pub const BASIC: Self = SgrCapabilities {
        styled_underline: false,
        underline_color: false,
        overline: false,
    };

    /// All of them (kitty, WezTerm, foot, Ghostty, mintty, VTE ≥ 0.60).
    pub const EXTENDED: Self = SgrCapabilities {
        styled_underline: true,
        underline_color: true,
        overline: true,
    };

    /// These capabilities with the underline styles `4:1`–`4:5` on or
    /// off: with the other builders, any set —
    /// `SgrCapabilities::BASIC.with_styled_underline(true)` is a terminal
    /// with curly underlines and nothing else.
    pub const fn with_styled_underline(mut self, on: bool) -> Self {
        self.styled_underline = on;
        self
    }

    /// These capabilities with the underline color (SGR 58 / 59) on or off.
    pub const fn with_underline_color(mut self, on: bool) -> Self {
        self.underline_color = on;
        self
    }

    /// These capabilities with the overline (SGR 53 / 55) on or off.
    pub const fn with_overline(mut self, on: bool) -> Self {
        self.overline = on;
        self
    }

    /// The capabilities of the terminal this process writes to, guessed
    /// from its environment ([`detect`](Self::detect) has the table).
    /// [`App::new`](crate::App::new) uses it;
    /// [`App::with_sgr_capabilities`](crate::App::with_sgr_capabilities)
    /// overrides it.
    pub fn from_env() -> Self {
        Self::detect(|name| std::env::var(name).ok())
    }

    /// [`from_env`](Self::from_env) reading the variables through `var`.
    /// Conservative — a terminal is given an extension only where it is
    /// known to read it, and anything unknown gets [`BASIC`](Self::BASIC):
    ///
    /// 1. **A multiplexer wins** over the outer terminal's variables it
    ///    inherits (`KITTY_WINDOW_ID`, `TERM_PROGRAM`, …), since it is
    ///    what the program writes to. GNU screen (`STY` set) reads none
    ///    of the colon forms: `BASIC`. Zellij (`ZELLIJ` set) is not known
    ///    to pass them all on: `BASIC`. tmux 3.2 and later
    ///    (`TERM_PROGRAM=tmux` with `TERM_PROGRAM_VERSION`, which it
    ///    exports from 3.2) parses all three and passes each to its outer
    ///    terminal where that terminal's terminfo (or tmux's
    ///    `terminal-features`) has it — `Smulx`, `Setulc`, `Smol` — a
    ///    plain underline or nothing elsewhere: `EXTENDED`. Any other
    ///    multiplexer sign —
    ///    `TMUX` set, a `TERM` of `screen*` / `tmux*` — is an older or
    ///    unknown one: `BASIC`.
    /// 2. **All three:** a `TERM` naming kitty, foot, WezTerm, Ghostty or
    ///    mintty; `TERM_PROGRAM` WezTerm, ghostty, mintty or kitty;
    ///    `KITTY_WINDOW_ID`; VTE 0.60 and later (`VTE_VERSION` ≥ 6000).
    /// 3. **The underline styles and color:** Alacritty (`TERM=alacritty`
    ///    or `ALACRITTY_WINDOW_ID`; 0.11), VS Code's terminal
    ///    (`TERM_PROGRAM=vscode`, version 1.60 and later), VTE 0.52–0.59.
    /// 4. **The underline styles alone:** iTerm2 3.4 and later
    ///    (`TERM_PROGRAM=iTerm.app`; its SGR 58 is not in a release).
    ///
    /// `COLORTERM` says a terminal has 24-bit color, nothing about these;
    /// Windows Terminal draws all of them from 1.20, but its
    /// `WT_SESSION` carries no version: `BASIC`, as is any terminal not
    /// listed — set the capabilities with
    /// [`App::with_sgr_capabilities`](crate::App::with_sgr_capabilities).
    pub fn detect(var: impl Fn(&str) -> Option<String>) -> Self {
        let styled_and_color = Self::BASIC
            .with_styled_underline(true)
            .with_underline_color(true);
        let term = var("TERM").unwrap_or_default();
        let program = var("TERM_PROGRAM").unwrap_or_default();
        let version = || var("TERM_PROGRAM_VERSION").and_then(|v| major_minor(&v));
        let at_least = |want: (u32, u32)| version().is_some_and(|v| v >= want);
        // 1. Multiplexers.
        if var("STY").is_some() || var("ZELLIJ").is_some() {
            return Self::BASIC;
        }
        if program == "tmux" {
            return if at_least((3, 2)) {
                Self::EXTENDED
            } else {
                Self::BASIC
            };
        }
        if var("TMUX").is_some() || term.starts_with("screen") || term.starts_with("tmux") {
            return Self::BASIC;
        }
        // 2. Every extension.
        let extended_term = ["kitty", "foot", "wezterm", "ghostty", "mintty"]
            .iter()
            .any(|t| term.contains(t));
        let extended_program = ["WezTerm", "ghostty", "mintty", "kitty"]
            .iter()
            .any(|p| program.eq_ignore_ascii_case(p));
        let vte = var("VTE_VERSION").and_then(|v| v.trim().parse::<u32>().ok());
        if extended_term
            || extended_program
            || var("KITTY_WINDOW_ID").is_some()
            || vte.is_some_and(|v| v >= 6000)
        {
            return Self::EXTENDED;
        }
        // 3, 4. Some of them.
        if term.contains("alacritty")
            || var("ALACRITTY_WINDOW_ID").is_some()
            || (program == "vscode" && at_least((1, 60)))
            || vte.is_some_and(|v| v >= 5200)
        {
            return styled_and_color;
        }
        if program == "iTerm.app" && at_least((3, 4)) {
            return Self::BASIC.with_styled_underline(true);
        }
        Self::BASIC
    }
}

/// A `major.minor[.…]` version's first two numbers (`3.1c` is 3.1).
fn major_minor(v: &str) -> Option<(u32, u32)> {
    let mut parts = v.trim().split('.');
    let major = parts.next()?.parse().ok()?;
    let minor: String = parts
        .next()
        .unwrap_or("0")
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    Some((major, minor.parse().unwrap_or(0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            vars.iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.to_string())
        }
    }

    fn detect(vars: &[(&str, &str)]) -> SgrCapabilities {
        SgrCapabilities::detect(env(vars))
    }

    const STYLED_AND_COLOR: SgrCapabilities = SgrCapabilities::BASIC
        .with_styled_underline(true)
        .with_underline_color(true);

    /// The builders make any set; `EXTENDED` is all three.
    #[test]
    fn the_builders_make_a_custom_set() {
        let c = SgrCapabilities::BASIC.with_styled_underline(true);
        assert!(c.styled_underline && !c.underline_color && !c.overline);
        assert_eq!(
            STYLED_AND_COLOR.with_overline(true),
            SgrCapabilities::EXTENDED
        );
        assert_eq!(
            SgrCapabilities::EXTENDED
                .with_styled_underline(false)
                .with_underline_color(false)
                .with_overline(false),
            SgrCapabilities::BASIC
        );
    }

    /// The terminals known to read every extension get them; an unknown
    /// one the common subset (`COLORTERM` says nothing about them).
    #[test]
    fn detection_reads_the_terminal_from_the_environment() {
        let ext = SgrCapabilities::EXTENDED;
        assert_eq!(detect(&[("TERM", "xterm-kitty")]), ext);
        assert_eq!(detect(&[("KITTY_WINDOW_ID", "1")]), ext);
        assert_eq!(detect(&[("TERM", "foot")]), ext);
        assert_eq!(detect(&[("TERM", "xterm-ghostty")]), ext);
        assert_eq!(
            detect(&[("TERM", "xterm-256color"), ("TERM_PROGRAM", "WezTerm")]),
            ext
        );
        assert_eq!(detect(&[("VTE_VERSION", "6800")]), ext);
        assert_eq!(detect(&[("VTE_VERSION", "5402")]), STYLED_AND_COLOR);
        assert_eq!(
            detect(&[("TERM", "xterm-256color")]),
            SgrCapabilities::BASIC
        );
        assert_eq!(
            detect(&[("TERM", "xterm-256color"), ("COLORTERM", "truecolor")]),
            SgrCapabilities::BASIC
        );
        assert_eq!(detect(&[]), SgrCapabilities::BASIC);
    }

    /// Alacritty (0.11): the underline styles and color, no overline.
    /// iTerm2 (3.4): the underline styles only — its SGR 58 is not in a
    /// release. VS Code's xterm.js: the styles and color. An older or
    /// unversioned iTerm2 / VS Code, and Windows Terminal (whose
    /// `WT_SESSION` carries no version; 1.20 added the styles), get the
    /// common subset.
    #[test]
    fn detection_knows_the_partial_terminals() {
        assert_eq!(detect(&[("TERM", "alacritty")]), STYLED_AND_COLOR);
        assert_eq!(detect(&[("ALACRITTY_WINDOW_ID", "7")]), STYLED_AND_COLOR);
        assert_eq!(
            detect(&[
                ("TERM_PROGRAM", "iTerm.app"),
                ("TERM_PROGRAM_VERSION", "3.5.4")
            ]),
            SgrCapabilities::BASIC.with_styled_underline(true)
        );
        assert_eq!(
            detect(&[
                ("TERM_PROGRAM", "iTerm.app"),
                ("TERM_PROGRAM_VERSION", "3.3.12")
            ]),
            SgrCapabilities::BASIC
        );
        assert_eq!(
            detect(&[("TERM_PROGRAM", "iTerm.app")]),
            SgrCapabilities::BASIC
        );
        assert_eq!(
            detect(&[
                ("TERM_PROGRAM", "vscode"),
                ("TERM_PROGRAM_VERSION", "1.94.2")
            ]),
            STYLED_AND_COLOR
        );
        assert_eq!(
            detect(&[
                ("TERM_PROGRAM", "vscode"),
                ("TERM_PROGRAM_VERSION", "1.50.0")
            ]),
            SgrCapabilities::BASIC
        );
        assert_eq!(
            detect(&[("TERM", "xterm-256color"), ("WT_SESSION", "0f3c…")]),
            SgrCapabilities::BASIC
        );
    }

    /// A multiplexer is what the program writes to, so it wins over the
    /// outer terminal's variables it inherits: GNU screen does not read
    /// the colon forms (common subset); tmux 3.2+ (`TERM_PROGRAM=tmux`,
    /// its version) parses all three and passes each on where its outer
    /// terminal's terminfo has it (`Smulx`, `Setulc`, `Smol`; a plain
    /// underline or nothing elsewhere); an older tmux, which
    /// leaves the outer `TERM_PROGRAM`, gets the common subset.
    #[test]
    fn a_multiplexer_wins_over_the_outer_terminal() {
        let basic = SgrCapabilities::BASIC;
        assert_eq!(
            detect(&[("TERM", "screen-256color"), ("KITTY_WINDOW_ID", "1")]),
            basic
        );
        assert_eq!(
            detect(&[("TERM", "xterm-kitty"), ("STY", "123.pts-0.host")]),
            basic
        );
        assert_eq!(
            detect(&[
                ("TERM", "tmux-256color"),
                ("TERM_PROGRAM", "WezTerm"),
                ("KITTY_WINDOW_ID", "1")
            ]),
            basic
        );
        assert_eq!(
            detect(&[
                ("TERM", "xterm-256color"),
                ("TMUX", "/tmp/tmux-501/default,1,0"),
                ("VTE_VERSION", "7000")
            ]),
            basic
        );
        assert_eq!(
            detect(&[
                ("TERM", "tmux-256color"),
                ("TMUX", "/tmp/tmux-501/default,1,0"),
                ("TERM_PROGRAM", "tmux"),
                ("TERM_PROGRAM_VERSION", "3.4"),
                ("KITTY_WINDOW_ID", "1")
            ]),
            SgrCapabilities::EXTENDED
        );
        assert_eq!(
            detect(&[
                ("TERM", "screen"),
                ("TERM_PROGRAM", "tmux"),
                ("TERM_PROGRAM_VERSION", "3.1c")
            ]),
            basic
        );
    }

    /// C10G-API-SMALL: Zellij is a multiplexer too — it sets `ZELLIJ` (and
    /// `ZELLIJ_SESSION_NAME`) and leaves the outer terminal's `TERM` and
    /// variables — so the outer terminal's extensions are not assumed to
    /// reach it: the common subset.
    #[test]
    fn zellij_is_a_multiplexer() {
        assert_eq!(
            detect(&[
                ("TERM", "xterm-kitty"),
                ("KITTY_WINDOW_ID", "1"),
                ("ZELLIJ", "0"),
                ("ZELLIJ_SESSION_NAME", "dev")
            ]),
            SgrCapabilities::BASIC
        );
        assert_eq!(
            detect(&[
                ("TERM", "xterm-256color"),
                ("ZELLIJ", "0"),
                ("VTE_VERSION", "7000")
            ]),
            SgrCapabilities::BASIC
        );
    }
}
