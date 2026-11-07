use rdom_tui::{Buffer, Color, ColorDepth, Modifier, Rect, Style};

use super::{Raster, font, render};
use crate::capture::Grid;
use crate::profile::{Profile, Rgb};

fn grid(w: u16, text: &str, style: Style) -> Grid {
    let mut buf = Buffer::empty(Rect::new(0, 0, w, 1));
    buf.set_string(0, 0, text, style);
    Grid::from_buffer(&buf, ColorDepth::TrueColor)
}

fn draw(g: &Grid, profile: &Profile, scale: u32) -> super::Image {
    render(g, Raster::new(profile, scale).unwrap()).unwrap()
}

/// The pixel at `(x, y)`, which the test knows is inside the image.
fn px(img: &super::Image, x: u32, y: u32) -> Rgb {
    img.at(x, y).expect("inside the image")
}

/// The pixels of cell `cx` (8 wide at scale 1) that are `c`.
fn count(img: &super::Image, cx: u32, c: Rgb) -> usize {
    (0..16)
        .flat_map(|y| (0..8).map(move |x| (cx * 8 + x, y)))
        .filter(|&(x, y)| px(img, x, y) == c)
        .count()
}

// --- Geometry and glyphs -------------------------------------------------

#[test]
fn a_cell_is_the_profiles_cell_times_the_scale() {
    let p = Profile::default_profile();
    let g = grid(3, "ab", Style::default());
    let img = draw(&g, &p, 1);
    assert_eq!((img.width, img.height), (24, 16));
    let img2 = draw(&g, &p, 2);
    assert_eq!((img2.width, img2.height), (48, 32));
    assert_eq!(img2.pixels.len(), 48 * 32);
}

#[test]
fn a_glyph_is_its_font_bitmap_in_the_glyph_color() {
    let p = Profile::default_profile();
    let red = [200, 0, 0];
    let g = grid(1, "A", Style::default().fg(Color::Rgb(200, 0, 0)));
    let img = draw(&g, &p, 1);
    let a = font::glyph('A').expect("the font has A");
    for y in 0..16 {
        for x in 0..8 {
            let want = if a.at(x, y) { red } else { p.bg };
            assert_eq!(px(&img, x as u32, y as u32), want, "({x},{y})");
        }
    }
}

#[test]
fn scaling_repeats_each_font_pixel() {
    let p = Profile::default_profile();
    let g = grid(1, "A", Style::default());
    let one = draw(&g, &p, 1);
    let three = draw(&g, &p, 3);
    for y in 0..48 {
        for x in 0..24 {
            assert_eq!(px(&three, x, y), px(&one, x / 3, y / 3));
        }
    }
}

#[test]
fn a_wide_glyph_spans_two_cells_and_its_trailing_cell_keeps_it() {
    let p = Profile::default_profile();
    let g = grid(2, "⌛", Style::default().bg(Color::Rgb(0, 0, 90)));
    let img = draw(&g, &p, 1);
    let glyph = font::glyph('⌛').expect("the font has ⌛");
    assert!(glyph.wide);
    let right_half_inked = (0..16).any(|y| (8..16).any(|x| glyph.at(x, y)));
    assert!(right_half_inked);
    assert!(
        count(&img, 1, p.fg) > 0,
        "the trailing cell shows the glyph's right half"
    );
    assert!(img.missing.is_empty());
}

#[test]
fn a_wide_character_the_font_lacks_is_a_wide_box() {
    // unscii has no CJK: honest tofu over both cells, listed.
    let p = Profile::default_profile();
    let img = draw(&grid(2, "中", Style::default()), &p, 1);
    assert_eq!(img.missing, vec!['中']);
    assert!(
        (0..16).all(|x| px(&img, x, 0) == p.fg),
        "the box spans both cells"
    );
    assert!((0..16).all(|y| px(&img, 15, y) == p.fg));
}

#[test]
fn box_drawing_comes_from_the_font_and_joins_across_cells() {
    let p = Profile::default_profile();
    let g = grid(2, "──", Style::default());
    let img = draw(&g, &p, 1);
    // A horizontal line through both cells: some row is lit edge to edge.
    let joined = (0..16).any(|y| (0..16).all(|x| px(&img, x, y) == p.fg));
    assert!(joined);
}

#[test]
fn a_character_the_font_lacks_is_a_box_and_is_listed() {
    let p = Profile::default_profile();
    let c = '\u{10FFFD}';
    assert!(font::glyph(c).is_none());
    let g = grid(2, &format!("{c}{c}"), Style::default());
    let img = draw(&g, &p, 1);
    assert_eq!(img.missing, vec![c], "listed once");
    // The box's frame.
    assert!((0..8).all(|x| px(&img, x, 0) == p.fg && px(&img, x, 15) == p.fg));
    assert!((0..16).all(|y| px(&img, 0, y) == p.fg && px(&img, 7, y) == p.fg));
}

// --- TUI glyph ranges (the spec's per-range fixtures) ---------------------

/// The rows inked at column `x` and the columns inked at row `y`.
fn column(c: char, x: usize) -> Vec<usize> {
    let g = font::glyph(c).unwrap();
    (0..16).filter(|&y| g.at(x, y)).collect()
}
fn row(c: char, y: usize) -> Vec<usize> {
    let g = font::glyph(c).unwrap();
    (0..8).filter(|&x| g.at(x, y)).collect()
}

#[test]
fn every_tui_range_is_in_the_font() {
    let ranges = [
        ('\u{2500}', '\u{257F}'),   // box drawing
        ('\u{2580}', '\u{259F}'),   // block elements
        ('\u{25A0}', '\u{25FF}'),   // geometric shapes
        ('\u{2800}', '\u{28FF}'),   // Braille
        ('\u{1FB00}', '\u{1FBCA}'), // legacy computing: sextants, wedges
        ('\u{E0B0}', '\u{E0B3}'),   // Powerline separators
        ('\u{00A0}', '\u{017F}'),   // Latin-1, Latin Extended-A
    ];
    for (from, to) in ranges {
        let missing: Vec<char> = (from..=to).filter(|&c| font::glyph(c).is_none()).collect();
        assert!(missing.is_empty(), "{from:?}..{to:?} lacks {missing:?}");
    }
}

#[test]
fn box_drawing_arms_meet_their_neighbours_at_every_weight() {
    // Each arm reaching an edge sits where the opposite edge's arm of the
    // same weight does — light, heavy, double — so any two box glyphs side
    // by side (or stacked) join without a gap or a step.
    let across = [vec![7, 8], vec![6, 7, 8, 9], vec![6, 8]];
    let down = [vec![3, 4], vec![2, 3, 4, 5], vec![1, 2, 4, 5]];
    let diagonals = ['╱', '╲', '╳'];
    for c in '\u{2500}'..='\u{257F}' {
        if diagonals.contains(&c) {
            continue;
        }
        for arm in [column(c, 0), column(c, 7)] {
            assert!(
                arm.is_empty() || across.contains(&arm),
                "{c}: side arm {arm:?}"
            );
        }
        for arm in [row(c, 0), row(c, 15)] {
            assert!(
                arm.is_empty() || down.contains(&arm),
                "{c}: end arm {arm:?}"
            );
        }
    }
    assert_eq!(column('┼', 0), column('┼', 7));
    assert_eq!(row('┼', 0), row('┼', 15));
}

#[test]
fn blocks_and_shades_cover_what_they_name() {
    let inked = |c: char| -> usize {
        let g = font::glyph(c).unwrap();
        (0..16)
            .map(|y| (0..8).filter(|&x| g.at(x, y)).count())
            .sum()
    };
    assert_eq!(inked('█'), 128);
    for half in ['▀', '▄', '▌', '▐', '▒'] {
        assert_eq!(inked(half), 64, "{half}");
    }
    assert_eq!((inked('░'), inked('▓')), (32, 96));
    let top = font::glyph('▀').unwrap();
    assert!((0..8).all(|y| (0..8).all(|x| top.at(x, y))));
}

#[test]
fn braille_dots_are_where_their_bits_say() {
    // Dot n of U+2800 + bits: the 2 × 4 grid of 4 × 4 pixel quarters.
    let place = [
        (0, 0),
        (0, 1),
        (0, 2),
        (1, 0),
        (1, 1),
        (1, 2),
        (0, 3),
        (1, 3),
    ];
    for bits in 0u32..256 {
        let c = char::from_u32(0x2800 + bits).unwrap();
        let g = font::glyph(c).unwrap();
        for (dot, (cx, ry)) in place.iter().enumerate() {
            let ink = (ry * 4..ry * 4 + 4).any(|y| (cx * 4..cx * 4 + 4).any(|x| g.at(x, y)));
            assert_eq!(ink, bits >> dot & 1 == 1, "{c} dot {}", dot + 1);
        }
    }
}

// --- Colors and attributes ------------------------------------------------

#[test]
fn colors_go_through_the_profile() {
    let g = grid(1, " ", Style::default().bg(Color::Indexed(4)));
    let dark = Profile::default_profile();
    let light = Profile::named("solarized-light").unwrap();
    assert_eq!(px(&draw(&g, &dark, 1), 0, 0), dark.palette[4]);
    assert_eq!(px(&draw(&g, &light, 1), 0, 0), light.palette[4]);
    assert_ne!(dark.palette[4], light.palette[4]);
}

#[test]
fn bold_smears_a_pixel_right_and_brightens_where_the_profile_does() {
    let g = |m| {
        grid(
            1,
            "l",
            Style::default().fg(Color::Indexed(1)).add_modifier(m),
        )
    };
    let xterm = Profile::default_profile();
    let plain = count(&draw(&g(Modifier::empty()), &xterm, 1), 0, xterm.palette[1]);
    let bold = count(&draw(&g(Modifier::BOLD), &xterm, 1), 0, xterm.palette[1]);
    assert!(bold > plain, "{bold} > {plain}");
    let vga = Profile::named("console-16").unwrap();
    let img = draw(&g(Modifier::BOLD), &vga, 1);
    assert!(count(&img, 0, vga.palette[9]) > 0, "bright red");
    assert_eq!(count(&img, 0, vga.palette[1]), 0);
}

#[test]
fn italic_shears_the_upper_half_within_the_cell() {
    let p = Profile::default_profile();
    let upright = draw(&grid(1, "I", Style::default()), &p, 1);
    let italic = draw(
        &grid(1, "I", Style::default().add_modifier(Modifier::ITALIC)),
        &p,
        1,
    );
    assert_ne!(upright.pixels, italic.pixels);
    // The lower half is unchanged.
    for y in 8..16 {
        for x in 0..8 {
            assert_eq!(px(&upright, x, y), px(&italic, x, y));
        }
    }
    let mut no_italic = p.clone();
    no_italic.italic = false;
    let shown = draw(
        &grid(1, "I", Style::default().add_modifier(Modifier::ITALIC)),
        &no_italic,
        1,
    );
    assert_eq!(
        shown.pixels, upright.pixels,
        "a profile without italic shows plain text"
    );
}

#[test]
fn underline_takes_its_own_color_and_style() {
    let p = Profile::default_profile();
    let style = Style::default()
        .add_modifier(Modifier::UNDERLINED)
        .underline_color(Color::Rgb(0, 200, 0));
    let img = draw(&grid(1, " ", style), &p, 1);
    assert!((0..8).all(|x| px(&img, x, 15) == [0, 200, 0]));
    let dotted = draw(
        &grid(1, " ", style.add_modifier(Modifier::UNDERLINE_DOTTED)),
        &p,
        1,
    );
    assert_eq!(px(&dotted, 0, 15), [0, 200, 0]);
    assert_eq!(px(&dotted, 1, 15), p.bg);
}

#[test]
fn strikethrough_and_overline_follow_the_profile() {
    let style = Style::default().add_modifier(Modifier::CROSSED_OUT | Modifier::OVERLINED);
    let p = Profile::default_profile();
    let img = draw(&grid(1, " ", style), &p, 1);
    assert!((0..8).all(|x| px(&img, x, 8) == p.fg && px(&img, x, 0) == p.fg));
    let mut bare = p.clone();
    bare.strikethrough = false;
    bare.overline = false;
    let img = draw(&grid(1, " ", style), &bare, 1);
    assert!((0..8).all(|x| px(&img, x, 8) == p.bg && px(&img, x, 0) == p.bg));
}

#[test]
fn hidden_text_shows_only_its_background() {
    let p = Profile::default_profile();
    let style = Style::default()
        .bg(Color::Rgb(9, 9, 9))
        .add_modifier(Modifier::HIDDEN);
    let img = draw(&grid(1, "A", style), &p, 1);
    assert!(img.pixels.iter().all(|&c| c == [9, 9, 9]));
}

// --- PNG ------------------------------------------------------------------

fn decode(bytes: &[u8]) -> (png::OutputInfo, Vec<u8>) {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().expect("a PNG header");
    let mut buf = vec![0; reader.output_buffer_size().expect("a size")];
    let info = reader.next_frame(&mut buf).expect("a PNG frame");
    buf.truncate(info.buffer_size());
    (info, buf)
}

fn rgb_bytes(img: &super::Image) -> Vec<u8> {
    img.pixels.iter().flatten().copied().collect()
}

#[test]
fn the_png_decodes_to_the_image() {
    let p = Profile::default_profile();
    let style = Style::default()
        .fg(Color::Rgb(250, 200, 10))
        .bg(Color::Indexed(4))
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let img = draw(&grid(12, "Hello, 世界 ─┼─", style), &p, 2);
    let (info, pixels) = decode(&img.png());
    assert_eq!((info.width, info.height), (img.width, img.height));
    assert_eq!(info.color_type, png::ColorType::Rgb);
    assert_eq!(pixels, rgb_bytes(&img));
}

#[test]
fn an_image_of_many_colors_is_written_as_rgb() {
    let pixels: Vec<Rgb> = (0..40 * 40)
        .map(|i| [(i % 251) as u8, (i / 7) as u8, (i % 13) as u8 * 19])
        .collect();
    let img = super::Image {
        width: 40,
        height: 40,
        pixels,
        missing: Vec::new(),
    };
    let bytes = img.png();
    assert_eq!(bytes[25], 2, "color type: RGB");
    let (_, decoded) = decode(&bytes);
    assert_eq!(decoded, rgb_bytes(&img));
}

#[test]
fn a_frame_png_is_indexed_compressed_and_deterministic() {
    let p = Profile::default_profile();
    let img = draw(
        &grid(40, "a terminal frame, mostly background", Style::default()),
        &p,
        1,
    );
    let bytes = img.png();
    assert_eq!(bytes[25], 3, "color type: indexed");
    assert!(
        bytes.len() < (img.width * img.height) as usize / 4,
        "{} bytes",
        bytes.len()
    );
    assert_eq!(bytes, img.png());
}

#[test]
fn long_matches_and_far_distances_round_trip() {
    // A repeating pattern longer than the longest match and rows further
    // apart than most distance codes.
    let pixels: Vec<Rgb> = (0..300 * 120)
        .map(|i| {
            if (i / 7) % 3 == 0 {
                [1, 2, 3]
            } else {
                [(i % 200) as u8, 0, 0]
            }
        })
        .collect();
    let img = super::Image {
        width: 300,
        height: 120,
        pixels,
        missing: Vec::new(),
    };
    let (_, decoded) = decode(&img.png());
    assert_eq!(decoded, rgb_bytes(&img));
}

// --- Limits and contracts -------------------------------------------------

#[test]
fn an_image_past_the_pixel_limit_is_refused_not_overflowed() {
    // 2048 × 1024 cells at ×4: 65536 × 65536 pixels, 2^32 — an overflow in
    // u32 and gigabytes in memory.
    let g = Grid {
        width: 2048,
        height: 1024,
        rows: Vec::new(),
    };
    let p = Profile::default_profile();
    let err = render(&g, Raster::new(&p, 4).unwrap()).unwrap_err();
    assert_eq!(
        err,
        super::RasterError::TooLarge {
            width: 65536,
            height: 65536
        }
    );
    assert!(err.to_string().contains("smaller"), "{err}");
}

#[test]
fn an_empty_grid_and_a_scale_out_of_range_are_refused() {
    let p = Profile::default_profile();
    let g = Grid {
        width: 0,
        height: 3,
        rows: Vec::new(),
    };
    assert_eq!(
        render(&g, Raster::new(&p, 1).unwrap()).unwrap_err(),
        super::RasterError::Empty
    );
    assert_eq!(
        Raster::new(&p, 0).unwrap_err(),
        super::RasterError::Scale(0)
    );
    assert_eq!(
        Raster::new(&p, 5).unwrap_err(),
        super::RasterError::Scale(5)
    );
    assert_eq!(Raster::new(&p, 3).unwrap().cell_px(), [24, 48]);
}

#[test]
fn what_a_cluster_adds_past_its_first_character_is_listed() {
    // `e` + COMBINING ACUTE ACCENT: the `e` is drawn, the accent is not —
    // and is listed rather than dropped unsaid.
    let p = Profile::default_profile();
    let img = draw(&grid(2, "e\u{301}x", Style::default()), &p, 1);
    assert_eq!(img.missing(), ['\u{301}']);
}

#[test]
fn an_apps_own_profile_draws_its_palette() {
    let mut palette = Profile::default_profile().palette;
    palette[2] = [1, 2, 3];
    let mine = Profile::new("mine", palette, [200, 200, 200], [10, 10, 10]);
    let img = draw(
        &grid(1, " ", Style::default().bg(Color::Indexed(2))),
        &mine,
        1,
    );
    assert_eq!(px(&img, 0, 0), [1, 2, 3]);
    let blank = draw(&grid(1, " ", Style::default()), &mine, 1);
    assert_eq!(px(&blank, 0, 0), [10, 10, 10]);
}

#[test]
#[ignore = "a measurement: cargo test --release -p rdom-capture --lib raster_cost -- --ignored --nocapture"]
fn raster_cost() {
    // A busy 200 × 60 frame at ×1 and ×4: render and encode, timed.
    let mut buf = Buffer::empty(Rect::new(0, 0, 200, 60));
    for y in 0..60 {
        let line: String = (0..200)
            .map(|x| char::from(b'!' + ((x + y) % 90) as u8))
            .collect();
        let style = Style::default()
            .fg(Color::Indexed((y % 16) as u8))
            .bg(Color::Rgb((y * 4) as u8, 20, 40));
        buf.set_string(0, y, &line, style);
    }
    let g = Grid::from_buffer(&buf, ColorDepth::TrueColor);
    let p = Profile::default_profile();
    for scale in [1, 4] {
        let t = std::time::Instant::now();
        let img = render(&g, Raster::new(&p, scale).unwrap()).unwrap();
        let drawn = t.elapsed();
        let bytes = img.png();
        println!(
            "200×60 ×{scale}: {}×{} px, render {drawn:?}, png {:?} ({} bytes)",
            img.width(),
            img.height(),
            t.elapsed() - drawn,
            bytes.len()
        );
    }
}
