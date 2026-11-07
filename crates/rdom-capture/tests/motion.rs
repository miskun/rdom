//! CAPTURE phase 8 (`specs/CAPTURE.md` §Motion): films of apps with known
//! motion — a spinner, a flicker, a heavy frame — and what they report.

use std::error::Error;

use rdom_capture::motion::{FilmRequest, Link, cast};
use rdom_capture::{CaptureApp, Environment, Session};
use rdom_tui::render::TestBackend;
use rdom_tui::{App, ControlFlow, TuiDom};

/// A cell changing every tick (50 ms) through `glyphs`, in a cycle, in
/// red on blue.
struct Ticker(&'static [&'static str]);

impl CaptureApp for Ticker {
    fn build(
        &self,
        env: &Environment,
        _: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = dom.create_element("p");
        let t = dom.create_text_node(self.0[0]);
        dom.append_child(p, t)?;
        dom.append_child(root, p)?;
        let glyphs = self.0;
        let mut i = 0;
        Ok(env
            .app(
                dom,
                rdom_css::from_css("p { color: #ff0000; background-color: #000080; }"),
            )?
            .with_tick_rate(std::time::Duration::from_millis(50))
            .with_tick_handler(move |ctx| {
                i = (i + 1) % glyphs.len();
                ctx.dom.set_text_content(p, glyphs[i]).unwrap();
                ControlFlow::Continue
            }))
    }
}

fn film(glyphs: &'static [&'static str], request: FilmRequest) -> rdom_capture::motion::Film {
    let mut s = Session::new(Ticker(glyphs));
    s.open(Environment::default().with_viewport(10, 2), None)
        .unwrap();
    s.film(&request).unwrap()
}

#[test]
fn a_spinner_is_found_with_its_pace() {
    let f = film(
        &["|", "/", "-", "\\"],
        FilmRequest::of(800).with_step(50).with_cells(vec![[0, 0]]),
    );
    assert_eq!(f.spinners.len(), 1, "{:?}", f.spinners);
    let s = &f.spinners[0];
    assert_eq!((s.glyphs.len(), s.frame_ms, s.period_ms), (4, 50, 200));
    // Its timeline: a change each frame.
    assert_eq!(f.cells[0].changes.len(), f.frames.len());
    // 50 ms a glyph is inside the comfortable range; equal widths.
    assert!(
        !f.findings
            .iter()
            .any(|x| x.check == rdom_capture::audit::Check::Spinner),
        "{:?}",
        f.findings
    );
    let jitter = film(&["◐", "◓", "中", "◒"], FilmRequest::of(800).with_step(50));
    assert!(
        jitter
            .findings
            .iter()
            .any(|x| x.check == rdom_capture::audit::Check::Spinner && x.message.contains("width")),
        "{:?}",
        jitter.findings
    );
}

#[test]
fn a_cell_that_blinks_each_frame_is_flicker() {
    let f = film(&["x", " "], FilmRequest::of(400).with_step(50));
    assert!(f.spinners.is_empty());
    assert!(
        f.findings
            .iter()
            .any(|x| x.check == rdom_capture::audit::Check::Flicker),
        "{:?}",
        f.findings
    );
    let still = film(&["x"], FilmRequest::of(400).with_step(50));
    assert!(
        !still
            .findings
            .iter()
            .any(|x| x.check == rdom_capture::audit::Check::Flicker)
    );
    assert!(still.heatmap.is_empty());
}

#[test]
fn frames_a_link_cannot_carry_in_time_are_found() {
    let slow = film(
        &["a", "b", "c"],
        FilmRequest::of(200).with_step(50).with_link(Link::Bps(100)),
    );
    assert!(
        slow.findings
            .iter()
            .any(|x| x.check == rdom_capture::audit::Check::Cost),
        "{:?}",
        slow.findings
    );
    let local = film(
        &["a", "b", "c"],
        FilmRequest::of(200).with_step(50).with_link(Link::Local),
    );
    assert!(
        !local
            .findings
            .iter()
            .any(|x| x.check == rdom_capture::audit::Check::Cost)
    );
}

#[test]
fn a_film_is_an_asciicast_of_what_was_sent() {
    let f = film(
        &["a", "b"],
        FilmRequest::of(100)
            .with_step(50)
            .with_reduced_motion(false),
    );
    let cast = cast::asciicast(&f);
    let lines: Vec<&str> = cast.lines().collect();
    assert_eq!(lines[0], r#"{"height":2,"version":2,"width":10}"#);
    assert_eq!(lines.len(), 1 + f.frames.len());
    assert!(
        lines[1].starts_with("[0.0,\"o\",\"\\u001b[2J"),
        "{}",
        lines[1]
    );
    assert!(lines[2].starts_with("[0.05,\"o\","), "{}", lines[2]);
    // Deterministic: the same film, the same file.
    let again = film(
        &["a", "b"],
        FilmRequest::of(100)
            .with_step(50)
            .with_reduced_motion(false),
    );
    assert_eq!(cast::asciicast(&again), cast);
    assert!(
        Session::new(Ticker(&["a"]))
            .film(&FilmRequest::of(10))
            .is_err(),
        "no app open"
    );
}

#[test]
fn a_cast_replays_frame_by_frame_in_a_terminal() {
    // Each event's bytes, applied in turn to a terminal emulator, show
    // what the film's frames hold — colors included, whatever the
    // backend left out of a frame as already set.
    let f = film(
        &["|", "/", "-", "\\"],
        FilmRequest::of(300)
            .with_step(50)
            .with_reduced_motion(false),
    );
    let mut screen = rdom_tui::render::VirtualScreen::new(f.size[0], f.size[1]);
    for (k, (bytes, grid)) in f.sent.iter().zip(&f.grids).enumerate() {
        screen.apply(bytes);
        assert_eq!(
            screen.row(0).trim_end(),
            grid.row_text(0).trim_end(),
            "frame {k}"
        );
        let cell = screen.cell(0, 0).unwrap();
        assert_eq!(
            cell.fg,
            rdom_tui::Color::Rgb(255, 0, 0),
            "frame {k}: the glyph's color"
        );
    }
}

#[test]
fn a_spinner_filmed_part_way_through_a_glyph_keeps_its_pace() {
    let mut s = Session::new(Ticker(&["a", "b", "c", "d"]));
    s.open(Environment::default().with_viewport(10, 2), None)
        .unwrap();
    s.advance(25).unwrap();
    let f = s
        .film(
            &FilmRequest::of(800)
                .with_step(25)
                .with_reduced_motion(false),
        )
        .unwrap();
    assert_eq!(f.spinners.len(), 1, "{:?}", f.spinners);
    assert_eq!(
        (f.spinners[0].glyphs.len(), f.spinners[0].frame_ms),
        (4, 50),
        "{:?}",
        f.spinners
    );
    assert!(
        !f.findings
            .iter()
            .any(|x| x.check == rdom_capture::audit::Check::Spinner)
    );
}

#[test]
fn a_film_already_under_reduced_motion_reports_what_moves() {
    let mut s = Session::new(Ticker(&["a", "b"]));
    let mut env = Environment::default().with_viewport(10, 2);
    env.reduced_motion = true;
    s.open(env, None).unwrap();
    let f = s.film(&FilmRequest::of(200).with_step(50)).unwrap();
    assert!(
        f.findings
            .iter()
            .any(|x| x.check == rdom_capture::audit::Check::ReducedMotion),
        "{:?}",
        f.findings
    );
    let off = s
        .film(
            &FilmRequest::of(200)
                .with_step(50)
                .with_reduced_motion(false),
        )
        .unwrap();
    assert!(off.not_checked.iter().any(|n| n.check == "reduced-motion"));
}

#[test]
fn a_film_with_no_step_or_a_dead_link_is_refused() {
    let mut s = Session::new(Ticker(&["a"]));
    s.open(Environment::default().with_viewport(10, 2), None)
        .unwrap();
    assert!(s.film(&FilmRequest::of(100).with_step(0)).is_err());
    assert!(
        s.film(&FilmRequest::of(100).with_link(Link::Bps(0)))
            .is_err()
    );
}
