//! The An+B microsyntax (CSS Syntax 3 §6): the argument of
//! `:nth-child()` and its family.
//!
//! The grammar is defined over tokens, so white space matters in
//! places: `2n + 1`, `2n+ 1`, `2n +1` and `2n- 1` are all `2n+1`, but no
//! white space may sit between a sign and the `n` it applies to (`+ n`,
//! `- n`), nor between a coefficient and its `n` (`2 n`) — there the
//! sign or number and the `n` form one token. This scanner reads the
//! token shapes the grammar names directly from the text.

/// `An+B` with both terms, clamped to `i32` (as Blink and Gecko clamp).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AnB {
    pub a: i32,
    pub b: i32,
}

/// Parse an `<an+b>` at the start of `src` (leading white space
/// skipped). Returns it and the bytes consumed — up to the end of its
/// last token, so the caller sees what follows (white space, `of`,
/// `)`). `Err` with a message when `src` does not start with one.
pub(super) fn parse_anb(src: &str) -> Result<(AnB, usize), String> {
    let mut s = Scanner { src, pos: 0 };
    s.skip_ws();
    let at = s.pos;
    // `+` binds only to an immediately following `n` (a `+n…` ident) or
    // digit (a signed integer).
    let plus = s.eat(b'+');
    let (a, rest) = if let Some(digits) = s.number() {
        let signed = if plus {
            digits
        } else {
            s.signed_from(at, digits)
        };
        match s.ident() {
            // A bare integer: `B` alone.
            None => {
                return Ok((
                    AnB {
                        a: 0,
                        b: clamp(signed),
                    },
                    s.pos,
                ));
            }
            // A dimension: its unit is `n`, `n-`, or `n-<digits>`.
            Some(unit) => (signed, unit),
        }
    } else {
        let ident = s.ident().ok_or("expected an+b")?;
        if plus && ident.starts_with('-') {
            return Err("`+` must be followed by `n`".into());
        }
        let lower = ident.to_ascii_lowercase();
        if !plus {
            match lower.as_str() {
                "odd" => return Ok((AnB { a: 2, b: 1 }, s.pos)),
                "even" => return Ok((AnB { a: 2, b: 0 }, s.pos)),
                _ => {}
            }
        }
        match lower.strip_prefix('-') {
            Some(unit) => (-1, unit.to_string()),
            None => (1, lower),
        }
    };
    let unit = rest.to_ascii_lowercase();
    let after_n = unit.strip_prefix('n').ok_or("expected `n`")?;
    let b = match after_n {
        // `An` — then an optional `± B` or a signed `B`.
        "" => s.trailing_b()?,
        // `An-` — then white space and an unsigned `B`.
        "-" => -s.signless()?,
        // `An-B` in one token.
        digits => {
            let digits = digits.strip_prefix('-').ok_or("expected `-` after `n`")?;
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err("expected digits after `n-`".into());
            }
            -parse_digits(digits)
        }
    };
    Ok((
        AnB {
            a: clamp(a),
            b: clamp(b),
        },
        s.pos,
    ))
}

struct Scanner<'a> {
    src: &'a str,
    pos: usize,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<u8> {
        self.src.as_bytes().get(self.pos).copied()
    }

    fn eat(&mut self, b: u8) -> bool {
        let hit = self.peek() == Some(b);
        if hit {
            self.pos += 1;
        }
        hit
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(|b| b.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    /// Digits at the cursor (after an optional `-` that a digit
    /// follows), as an `i64`; `None` without one. A fraction or an
    /// exponent makes a non-integer number, which no An+B form takes:
    /// it is left for the caller to reject.
    fn number(&mut self) -> Option<i64> {
        let start = self.pos;
        if self.peek() == Some(b'-')
            && self
                .src
                .as_bytes()
                .get(self.pos + 1)
                .is_some_and(u8::is_ascii_digit)
        {
            self.pos += 1;
        }
        let digits_at = self.pos;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.pos == digits_at {
            self.pos = start;
            return None;
        }
        Some(parse_digits(&self.src[digits_at..self.pos]))
    }

    /// The number read from `at` with its sign applied.
    fn signed_from(&self, at: usize, digits: i64) -> i64 {
        if self.src.as_bytes()[at] == b'-' {
            -digits
        } else {
            digits
        }
    }

    /// An identifier at the cursor (letters, digits, `-`, `_`, non-ASCII),
    /// starting with a letter, `_`, non-ASCII, or `-` followed by one.
    fn ident(&mut self) -> Option<String> {
        let bytes = self.src.as_bytes();
        let start = self.pos;
        let first = *bytes.get(start)?;
        let starts = |b: u8| b.is_ascii_alphabetic() || b == b'_' || !b.is_ascii();
        let ok =
            starts(first) || (first == b'-' && bytes.get(start + 1).is_some_and(|&b| starts(b)));
        if !ok {
            return None;
        }
        let mut end = start;
        while bytes
            .get(end)
            .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || !b.is_ascii())
        {
            end += 1;
        }
        self.pos = end;
        Some(self.src[start..end].to_string())
    }

    /// After `An`: nothing, `['+' | '-'] <signless-integer>` (white
    /// space around the sign allowed), or a `<signed-integer>`.
    fn trailing_b(&mut self) -> Result<i64, String> {
        let before = self.pos;
        self.skip_ws();
        let sign = match self.peek() {
            Some(b'+') => 1,
            Some(b'-') => -1,
            _ => {
                self.pos = before;
                return Ok(0);
            }
        };
        self.pos += 1;
        Ok(sign * self.signless()?)
    }

    /// White space, then an unsigned integer.
    fn signless(&mut self) -> Result<i64, String> {
        self.skip_ws();
        let at = self.pos;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.pos == at {
            return Err("expected an unsigned integer".into());
        }
        Ok(parse_digits(&self.src[at..self.pos]))
    }
}

/// ASCII digits as an `i64`, saturating.
fn parse_digits(digits: &str) -> i64 {
    digits.bytes().fold(0i64, |v, d| {
        v.saturating_mul(10).saturating_add(i64::from(d - b'0'))
    })
}

fn clamp(v: i64) -> i32 {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}
