//! The xterm 256-color palette as sRGB, for computing with a palette
//! index (`color: 208`, `Color::Indexed`) — in a color function or a
//! transition.
//!
//! Indices 16–255 are fixed by xterm: a 6 × 6 × 6 color cube and a
//! 24-step gray ramp. Indices 0–15 follow the user's terminal theme;
//! they take xterm's default values here (DIVERGENCES).

/// xterm's default colors for indices 0–15.
const ANSI: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (205, 0, 0),
    (0, 205, 0),
    (205, 205, 0),
    (0, 0, 238),
    (205, 0, 205),
    (0, 205, 205),
    (229, 229, 229),
    (127, 127, 127),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (92, 92, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
];

/// The color cube's channel levels.
const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// The sRGB color of palette index `n`.
pub fn xterm_rgb(n: u8) -> (u8, u8, u8) {
    match n {
        0..=15 => ANSI[usize::from(n)],
        16..=231 => {
            let i = n - 16;
            (
                CUBE[usize::from(i / 36)],
                CUBE[usize::from(i / 6 % 6)],
                CUBE[usize::from(i % 6)],
            )
        }
        _ => {
            let gray = 8 + 10 * (n - 232);
            (gray, gray, gray)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cube and the ramp, at their ends and one known entry.
    #[test]
    fn cube_and_gray_ramp() {
        assert_eq!(xterm_rgb(16), (0, 0, 0));
        assert_eq!(xterm_rgb(208), (255, 135, 0));
        assert_eq!(xterm_rgb(231), (255, 255, 255));
        assert_eq!(xterm_rgb(232), (8, 8, 8));
        assert_eq!(xterm_rgb(255), (238, 238, 238));
        assert_eq!(xterm_rgb(9), (255, 0, 0));
    }
}
