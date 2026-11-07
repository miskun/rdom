//! The color math the look audits read: WCAG 2 relative luminance and
//! contrast ratio, APCA's lightness contrast (Lc), and OKLab distance for
//! near-duplicate colors. Inputs are sRGB as the profile shows them.

use crate::profile::Rgb;

/// A channel, 0–255, as linear light (sRGB's transfer function).
fn linear(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG 2's relative luminance (§ "relative luminance").
pub(crate) fn luminance([r, g, b]: Rgb) -> f64 {
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

/// WCAG 2's contrast ratio of two colors, 1–21.
pub(crate) fn wcag_ratio(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// APCA (0.0.98G-4g) lightness contrast of text `fg` on `bg`: positive for
/// dark text on light, negative for light on dark; |Lc| ≥ 60 is roughly
/// body text, ≥ 75 preferred.
pub(crate) fn apca_lc(fg: Rgb, bg: Rgb) -> f64 {
    // APCA's own simple luminance: each channel to the 2.4 power.
    let y = |[r, g, b]: Rgb| {
        let ch = |c: u8| (f64::from(c) / 255.0).powf(2.4);
        0.2126729 * ch(r) + 0.7151522 * ch(g) + 0.0721750 * ch(b)
    };
    let clamp = |y: f64| {
        if y > 0.022 {
            y
        } else {
            y + (0.022 - y).powf(1.414)
        }
    };
    let (t, b) = (clamp(y(fg)), clamp(y(bg)));
    if (b - t).abs() < 0.0005 {
        return 0.0;
    }
    let sapc = if b > t {
        (b.powf(0.56) - t.powf(0.57)) * 1.14
    } else {
        (b.powf(0.65) - t.powf(0.62)) * 1.14
    };
    let lc = if sapc.abs() < 0.1 {
        0.0
    } else if sapc > 0.0 {
        sapc - 0.027
    } else {
        sapc + 0.027
    };
    lc * 100.0
}

/// A color in OKLab.
pub(crate) fn oklab([r, g, b]: Rgb) -> [f64; 3] {
    let (r, g, b) = (linear(r), linear(g), linear(b));
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

/// The OKLab distance of two colors (ΔE_ok): about 0.02 is the smallest
/// difference people notice side by side.
pub(crate) fn delta_e(a: Rgb, b: Rgb) -> f64 {
    let (a, b) = (oklab(a), oklab(b));
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::{apca_lc, delta_e, oklab, wcag_ratio};

    #[test]
    fn the_wcag_ratio_matches_its_reference_values() {
        assert!((wcag_ratio([0, 0, 0], [255, 255, 255]) - 21.0).abs() < 1e-9);
        assert!((wcag_ratio([255, 255, 255], [255, 255, 255]) - 1.0).abs() < 1e-9);
        // #767676 on white is the classic 4.54:1.
        let r = wcag_ratio([0x76, 0x76, 0x76], [255, 255, 255]);
        assert!((r - 4.54).abs() < 0.01, "{r}");
    }

    #[test]
    fn apca_matches_its_reference_values() {
        // APCA's published examples: #888 on #fff ≈ Lc 63.1; #fff on #888
        // ≈ −68.5; black on white ≈ 106.0; white on black ≈ −107.9.
        let near = |a: f64, b: f64| (a - b).abs() < 0.2;
        assert!(
            near(apca_lc([0x88; 3], [0xff; 3]), 63.1),
            "{}",
            apca_lc([0x88; 3], [0xff; 3])
        );
        assert!(
            near(apca_lc([0xff; 3], [0x88; 3]), -68.5),
            "{}",
            apca_lc([0xff; 3], [0x88; 3])
        );
        assert!(near(apca_lc([0; 3], [0xff; 3]), 106.0));
        assert!(near(apca_lc([0xff; 3], [0; 3]), -107.9));
        assert_eq!(apca_lc([0x40; 3], [0x40; 3]), 0.0);
    }

    #[test]
    fn oklab_distance_tells_near_duplicates_apart() {
        let white = oklab([255, 255, 255]);
        assert!((white[0] - 1.0).abs() < 1e-3 && white[1].abs() < 1e-3);
        assert!(delta_e([200, 30, 30], [201, 30, 30]) < 0.01);
        assert!(delta_e([200, 30, 30], [30, 30, 200]) > 0.3);
    }
}
