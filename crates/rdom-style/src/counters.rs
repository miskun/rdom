//! CSS counters (CSS Lists 3 §3): the `counter-reset` /
//! `counter-increment` declarations and the `counter()` value used
//! by `content`. The cascade in `rdom-tui` keeps the per-element
//! counter state; this module owns the data model and the number
//! formatting.

/// One `name [<integer>]` item of `counter-reset` / `counter-increment`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CounterOp {
    pub name: String,
    /// The reset value, or the increment delta.
    pub value: i32,
}

/// `<counter-style>` subset accepted by `counter(name, style)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CounterStyle {
    #[default]
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

impl CounterStyle {
    /// Parse a `<counter-style>` keyword (`decimal`, `lower-alpha`,
    /// `upper-alpha`, `lower-roman`, `upper-roman`, plus the
    /// `lower-latin` / `upper-latin` aliases).
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "decimal" => CounterStyle::Decimal,
            "lower-alpha" | "lower-latin" => CounterStyle::LowerAlpha,
            "upper-alpha" | "upper-latin" => CounterStyle::UpperAlpha,
            "lower-roman" => CounterStyle::LowerRoman,
            "upper-roman" => CounterStyle::UpperRoman,
            _ => return None,
        })
    }

    /// The CSS keyword.
    pub fn as_str(self) -> &'static str {
        match self {
            CounterStyle::Decimal => "decimal",
            CounterStyle::LowerAlpha => "lower-alpha",
            CounterStyle::UpperAlpha => "upper-alpha",
            CounterStyle::LowerRoman => "lower-roman",
            CounterStyle::UpperRoman => "upper-roman",
        }
    }

    /// Render `n` in this style. Alphabetic and roman styles fall back
    /// to decimal outside their range, as CSS Counter Styles 3
    /// prescribes for `system: alphabetic` (n ≥ 1) and the roman range
    /// (1..=3999).
    pub fn format(self, n: i32) -> String {
        match self {
            CounterStyle::Decimal => n.to_string(),
            CounterStyle::LowerAlpha | CounterStyle::UpperAlpha if n >= 1 => {
                let mut out = Vec::new();
                let mut k = n as u32;
                while k > 0 {
                    k -= 1;
                    out.push(b'a' + (k % 26) as u8);
                    k /= 26;
                }
                out.reverse();
                let s = String::from_utf8(out).expect("ascii");
                if self == CounterStyle::UpperAlpha {
                    s.to_ascii_uppercase()
                } else {
                    s
                }
            }
            CounterStyle::LowerRoman | CounterStyle::UpperRoman if (1..=3999).contains(&n) => {
                const TABLE: [(i32, &str); 13] = [
                    (1000, "m"),
                    (900, "cm"),
                    (500, "d"),
                    (400, "cd"),
                    (100, "c"),
                    (90, "xc"),
                    (50, "l"),
                    (40, "xl"),
                    (10, "x"),
                    (9, "ix"),
                    (5, "v"),
                    (4, "iv"),
                    (1, "i"),
                ];
                let mut k = n;
                let mut s = String::new();
                for (v, sym) in TABLE {
                    while k >= v {
                        s.push_str(sym);
                        k -= v;
                    }
                }
                if self == CounterStyle::UpperRoman {
                    s.to_ascii_uppercase()
                } else {
                    s
                }
            }
            _ => n.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CounterStyle;

    #[test]
    fn formats_follow_css_counter_styles() {
        assert_eq!(CounterStyle::Decimal.format(12), "12");
        assert_eq!(CounterStyle::LowerAlpha.format(1), "a");
        assert_eq!(CounterStyle::LowerAlpha.format(27), "aa");
        assert_eq!(CounterStyle::UpperAlpha.format(28), "AB");
        assert_eq!(
            CounterStyle::LowerAlpha.format(0),
            "0",
            "out of range → decimal"
        );
        assert_eq!(CounterStyle::LowerRoman.format(1994), "mcmxciv");
        assert_eq!(CounterStyle::UpperRoman.format(4), "IV");
        assert_eq!(CounterStyle::UpperRoman.format(4000), "4000");
        assert_eq!(
            CounterStyle::parse("Upper-Latin"),
            Some(CounterStyle::UpperAlpha)
        );
        assert_eq!(CounterStyle::parse("disc"), None);
    }
}
