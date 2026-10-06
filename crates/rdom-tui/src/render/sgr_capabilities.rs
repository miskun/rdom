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

    /// The capabilities of the terminal this process writes to, guessed
    /// from its environment (`TERM`, `TERM_PROGRAM`, `VTE_VERSION`,
    /// `KITTY_WINDOW_ID`): [`EXTENDED`](Self::EXTENDED) for the terminals
    /// known to support the extensions, VTE 0.52–0.59's underline styles
    /// and color without the overline, else [`BASIC`](Self::BASIC).
    pub fn from_env() -> Self {
        Self::detect(|name| std::env::var(name).ok())
    }

    /// [`from_env`](Self::from_env) reading the variables through `var`.
    pub fn detect(var: impl Fn(&str) -> Option<String>) -> Self {
        let term = var("TERM").unwrap_or_default();
        let program = var("TERM_PROGRAM").unwrap_or_default();
        let extended_term = ["kitty", "foot", "wezterm", "ghostty", "mintty"]
            .iter()
            .any(|t| term.contains(t));
        let extended_program = ["WezTerm", "ghostty", "mintty", "kitty"]
            .iter()
            .any(|p| program.eq_ignore_ascii_case(p));
        if extended_term || extended_program || var("KITTY_WINDOW_ID").is_some() {
            return Self::EXTENDED;
        }
        match var("VTE_VERSION").and_then(|v| v.trim().parse::<u32>().ok()) {
            Some(v) if v >= 6000 => Self::EXTENDED,
            Some(v) if v >= 5200 => SgrCapabilities {
                overline: false,
                ..Self::EXTENDED
            },
            _ => Self::BASIC,
        }
    }
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

    /// The terminals known to read the extensions get them; an unknown
    /// one the common subset.
    #[test]
    fn detection_reads_the_terminal_from_the_environment() {
        assert_eq!(
            SgrCapabilities::detect(env(&[("TERM", "xterm-kitty")])),
            SgrCapabilities::EXTENDED
        );
        assert_eq!(
            SgrCapabilities::detect(env(&[
                ("TERM", "xterm-256color"),
                ("TERM_PROGRAM", "WezTerm")
            ])),
            SgrCapabilities::EXTENDED
        );
        assert_eq!(
            SgrCapabilities::detect(env(&[("VTE_VERSION", "5402")])),
            SgrCapabilities {
                overline: false,
                ..SgrCapabilities::EXTENDED
            }
        );
        assert_eq!(
            SgrCapabilities::detect(env(&[("TERM", "xterm-256color")])),
            SgrCapabilities::BASIC
        );
        assert_eq!(SgrCapabilities::detect(env(&[])), SgrCapabilities::BASIC);
    }
}
