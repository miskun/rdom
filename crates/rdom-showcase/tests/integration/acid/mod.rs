//! The acid test, stage 1: the static tiles (`specs/ACID.md`).
//!
//! Every tile of `rdom_showcase::demos::acid` is compared, cell by cell
//! and colour by colour, against a reference derived by hand from the
//! spec (`refs/`). One test per tile, so a filter runs one
//! (`cargo test -p rdom-showcase --test integration acid::tile_05`), and
//! `acid::report`, which checks them all and lists every tile's result —
//! one failing tile never hides another.
//!
//! A failing tile is a bug in rdom or a misreading of the spec, decided
//! by the spec: a bug is fixed in its crate (an `ACID-FIX-<n>` TDD item),
//! a misreading is fixed in the reference with the argument written into
//! its derivation. A reference is never edited to match rdom's output.

mod compare;
mod coverage;
mod interactive;
mod reference;
mod refs;

use compare::{FrameReport, TileReport, check, check_on, painted_page};
use rdom_showcase::demos::acid;
use reference::Reference;

fn assert_tile(reference: &'static Reference) {
    let report = check(reference);
    assert!(report.passed(), "\n{}", report.failure());
}

/// Every tile's result, page by page, in `refs::ALL` order within a page —
/// each page painted once for the run (`compare::painted_page`), and a page
/// whose paint panicked reported against each of its tiles, so one page
/// never hides another's.
fn static_results() -> Vec<TileReport> {
    let mut reports: Vec<TileReport> = Vec::new();
    for page in 1..=acid::page_count() {
        let screen = painted_page(page).map(|p| &p.screen);
        for reference in refs::ALL {
            let Some(t) = acid::TILES
                .iter()
                .copied()
                .find(|t| t.id == reference.tile && t.page == page)
            else {
                continue;
            };
            reports.push(match screen {
                Ok(screen) => check_on(screen, reference),
                Err(e) => TileReport {
                    tile: t,
                    spec: reference.spec,
                    error: Some(e.to_string()),
                    diffs: Vec::new(),
                    painted: String::new(),
                },
            });
        }
    }
    reports
}

/// Every page's frame: the replay against the frame drawn, and what shows
/// outside the tiles.
fn frame_results() -> Vec<FrameReport> {
    (1..=acid::page_count()).map(FrameReport::of).collect()
}

/// A frame's result as one summary line.
fn frame_line(r: &FrameReport) -> String {
    format!(
        "  page {:<2} {}\n",
        r.page,
        if r.passed() {
            "pass".to_string()
        } else if let Some(e) = &r.error {
            format!("ERROR {e}")
        } else {
            format!(
                "FAIL ({} replay, {} outside cells)",
                r.round_trip.len(),
                r.outside.len()
            )
        }
    )
}

/// A tile's result as one summary line.
fn tile_line(r: &TileReport) -> String {
    format!(
        "  {:>3} {:<38} p{:<2} {}\n",
        r.tile.id,
        r.tile.title,
        r.tile.page,
        if r.passed() {
            "pass".to_string()
        } else if let Some(e) = &r.error {
            format!("ERROR {e}")
        } else {
            format!("FAIL ({} cells)", r.diffs.len())
        }
    )
}

/// Every tile, page by page: a line per tile, then each failure's report.
#[test]
fn report() {
    let reports = static_results();
    let mut out = String::from("\nacid static tiles:\n");
    for r in &reports {
        out.push_str(&tile_line(r));
    }
    let failed: Vec<&TileReport> = reports.iter().filter(|r| !r.passed()).collect();
    for r in &failed {
        out.push('\n');
        out.push_str(&r.failure());
    }
    assert!(failed.is_empty(), "{out}");
}

/// Every page's frame (`compare::round_trip`, `compare::check_frame`): the
/// replayed screen is the frame the `App` drew, with no sequence it could
/// not model, and outside the tiles only their labels show.
#[test]
fn page_frames() {
    let frames = frame_results();
    let mut out = String::from("\nacid page frames:\n");
    for r in &frames {
        out.push_str(&frame_line(r));
    }
    let failed: Vec<&FrameReport> = frames.iter().filter(|r| !r.passed()).collect();
    for r in &failed {
        out.push('\n');
        out.push_str(&r.failure());
    }
    assert!(failed.is_empty(), "{out}");
}

/// The whole acid run in one list: every static tile and every stage-2
/// step with its result (printed with `--nocapture`), then every failure's
/// report.
#[test]
fn summary() {
    let tiles = static_results();
    let frames = frame_results();
    let steps = interactive::results();
    let mut out = format!(
        "\nacid: {} tiles on {} pages, {} interactive steps\n\nstage 1 — static tiles:\n",
        tiles.len(),
        acid::page_count(),
        steps.len()
    );
    for r in &tiles {
        out.push_str(&tile_line(r));
    }
    out.push_str("\nstage 1 — page frames:\n");
    for r in &frames {
        out.push_str(&frame_line(r));
    }
    out.push_str("\nstage 2 — interactive steps:\n");
    for r in &steps {
        out.push_str(&interactive::summary_line(r));
    }
    let failed_tiles: Vec<&TileReport> = tiles.iter().filter(|r| !r.passed()).collect();
    let failed_frames: Vec<&FrameReport> = frames.iter().filter(|r| !r.passed()).collect();
    let failed_steps: Vec<_> = steps.iter().filter(|r| !r.passed()).collect();
    out.push_str(&format!(
        "\n{} of {} tiles, {} of {} page frames and {} of {} steps pass\n",
        tiles.len() - failed_tiles.len(),
        tiles.len(),
        frames.len() - failed_frames.len(),
        frames.len(),
        steps.len() - failed_steps.len(),
        steps.len()
    ));
    println!("{out}");
    for r in &failed_tiles {
        out.push('\n');
        out.push_str(&r.failure());
    }
    for r in &failed_frames {
        out.push('\n');
        out.push_str(&r.failure());
    }
    for r in &failed_steps {
        out.push('\n');
        out.push_str(&r.failure());
    }
    assert!(
        failed_tiles.is_empty() && failed_frames.is_empty() && failed_steps.is_empty(),
        "{out}"
    );
}

/// The example's pager (`acid::page_through`): Ctrl+N and Ctrl+P swap the
/// page shown for the next and the previous one, wrapping at the ends.
#[test]
fn the_example_pages_through_every_page() {
    use crossterm::event::{Event as CtEvent, KeyCode, KeyEvent, KeyModifiers};
    use rdom_tui::TuiDom;
    use rdom_tui::render::{Terminal, TestBackend};
    use rdom_tui::runtime::app::App;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let page_root = acid::build_page(&mut dom, 1);
    dom.append_child(root, page_root).unwrap();
    let terminal = Terminal::new(TestBackend::new(acid::PAGE_WIDTH, acid::PAGE_HEIGHT)).unwrap();
    let mut app = App::with_backend(dom, acid::stylesheet(), terminal)
        .unwrap()
        .with_import_loader(acid::import_loader());
    app.push_stylesheet(acid::late_stylesheet());
    acid::page_through(&mut app, 1);
    let shown = |app: &App<TestBackend>| -> Vec<u8> {
        (1..=acid::page_count())
            .filter(|p| {
                let dom = app.dom();
                dom.query_selector_in(dom.root(), &format!(".acid-page-{p}"))
                    .expect("a valid selector")
                    .is_some()
            })
            .collect()
    };
    let ctrl = |app: &mut App<TestBackend>, c: char| {
        app.handle_event(CtEvent::Key(KeyEvent::new(
            KeyCode::Char(c),
            KeyModifiers::CONTROL,
        )));
        for _ in 0..3 {
            app.advance(0).unwrap();
        }
    };
    for _ in 0..3 {
        app.advance(0).unwrap();
    }
    assert_eq!(shown(&app), [1]);
    ctrl(&mut app, 'n');
    assert_eq!(shown(&app), [2]);
    ctrl(&mut app, 'p');
    ctrl(&mut app, 'p');
    assert_eq!(
        shown(&app),
        [acid::page_count()],
        "wrapped to the last page"
    );
    ctrl(&mut app, 'n');
    assert_eq!(shown(&app), [1], "and back");
}

/// The example's footer (C16G-ACID-PAGER; Phase 16 gate API N6): a row of
/// its own below the 120 × 50 page — page number, keys — that follows the
/// pager, and says so when the terminal is too small for the page.
#[test]
fn the_example_shows_the_page_and_the_keys_below_the_page() {
    use crossterm::event::{Event as CtEvent, KeyCode, KeyEvent, KeyModifiers};
    use rdom_tui::TuiDom;
    use rdom_tui::render::{Terminal, TestBackend};
    use rdom_tui::runtime::app::App;

    let run = |w: u16, h: u16, next: bool| -> Vec<String> {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let page_root = acid::build_page(&mut dom, 1);
        dom.append_child(root, page_root).unwrap();
        acid::add_pager_footer(&mut dom, 1);
        let terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        let mut app = App::with_backend(dom, acid::stylesheet(), terminal)
            .unwrap()
            .with_import_loader(acid::import_loader());
        app.push_stylesheet(acid::late_stylesheet());
        app.push_stylesheet(acid::pager_stylesheet());
        acid::page_through(&mut app, 1);
        for _ in 0..3 {
            app.advance(0).unwrap();
        }
        if next {
            app.handle_event(CtEvent::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::CONTROL,
            )));
            for _ in 0..3 {
                app.advance(0).unwrap();
            }
        }
        let buf = app.terminal().buffer();
        (0..h)
            .map(|y| (0..w).map(|x| buf.cell(x, y).unwrap().symbol()).collect())
            .collect()
    };
    let rows = run(acid::PAGE_WIDTH, acid::PAGE_HEIGHT + 1, false);
    let footer = rows.last().unwrap();
    assert!(
        footer.starts_with(" acid page 1/13 · Ctrl+N next · Ctrl+P previous · Ctrl+C quit"),
        "{footer:?}"
    );
    assert!(!footer.contains("too small"), "{footer:?}");
    let rows = run(acid::PAGE_WIDTH, acid::PAGE_HEIGHT + 1, true);
    assert!(
        rows.last().unwrap().starts_with(" acid page 2/13"),
        "the pager moves it"
    );
    let rows = run(100, 40, false);
    assert!(
        rows.last()
            .unwrap()
            .contains("too small: the pages are 120 × 50, plus this row"),
        "{:?}",
        rows.last()
    );
}

/// Every tile has a reference and every reference a tile.
#[test]
fn every_tile_has_a_reference() {
    for t in acid::TILES {
        assert!(
            refs::ALL.iter().any(|r| r.tile == t.id),
            "acid tile {} has no reference",
            t.id
        );
    }
    for r in refs::ALL {
        assert!(
            acid::TILES.iter().any(|t| t.id == r.tile),
            "reference for unknown tile {}",
            r.tile
        );
    }
}

/// The tiles stay on their page and off each other and their labels.
#[test]
fn tiles_fit_their_pages_without_overlap() {
    for t in acid::TILES {
        assert!(t.y >= 1, "tile {}: no row for its label", t.id);
        assert!(t.x + t.w <= acid::PAGE_WIDTH && t.y + t.h <= acid::PAGE_HEIGHT);
    }
    for a in acid::TILES {
        for b in acid::TILES {
            if a.id == b.id || a.page != b.page {
                continue;
            }
            // Each tile's area with its label row.
            let apart = a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h < b.y || b.y + b.h < a.y;
            assert!(
                apart,
                "tiles {} and {} overlap on page {}",
                a.id, b.id, a.page
            );
        }
    }
}

#[test]
fn tile_01_cascade() {
    assert_tile(&refs::t01_cascade::REF);
}

/// The comparator's own case: a reference that expects the wrong colour,
/// glyph and background in tile 1 fails at exactly those cells, and the
/// report names the tile, the cell, both sides and the spec.
#[test]
fn comparator_reports_each_wrong_cell() {
    static WRONG: Reference = Reference {
        tile: "1",
        spec: &["test spec §0"],
        legend: &[
            ('g', "fg #00a000"),
            ('r', "fg #c00000"),
            ('b', "bg #000080"),
        ],
        grid: r#"
|ub late-sheet style-el style-spec     |
|rg.gggggggggg.gggggggg.gggggggggg....b|
|inline imp-sheet imp-inline           |
|gggggg.ggggggggg.gggggggggg...........|
|order imp-main spec-main invalid      |
|ggggg.gggggggg.ggggggggg.ggggggg......|
|unlayered imp-layer layer-id          |
|ggggggggg.ggggggggg.gggggggg..........|
|                                      |
|......................................|
"#,
    };
    let report = check(&WRONG);
    assert_eq!(report.diffs.len(), 3, "{}", report.failure());
    assert!(report.diffs[0].starts_with("(0, 0) expected \"u\" fg #c00000"));
    assert!(report.diffs[1].starts_with("(1, 0) expected \"b\""));
    assert!(report.diffs[2].starts_with("(37, 0)"));
    let text = report.failure();
    assert!(text.contains("tile 1 \"Cascade order\"") && text.contains("test spec §0"));
}

#[test]
fn tile_02_specificity() {
    assert_tile(&refs::t02_specificity::REF);
}

#[test]
fn tile_03_inheritance() {
    assert_tile(&refs::t03_inheritance::REF);
}

#[test]
fn tile_04_selectors() {
    assert_tile(&refs::t04_selectors::REF);
}

#[test]
fn tile_05_box_model() {
    assert_tile(&refs::t05_box_model::REF);
}

#[test]
fn tile_06_margins() {
    assert_tile(&refs::t06_margins::REF);
}

#[test]
fn tile_07_flex() {
    assert_tile(&refs::t07_flex::REF);
}

#[test]
fn tile_08_inline() {
    assert_tile(&refs::t08_inline::REF);
}

#[test]
fn tile_09a_generated() {
    assert_tile(&refs::t09a_generated::REF);
}

#[test]
fn tile_09b_lists() {
    assert_tile(&refs::t09b_lists::REF);
}

#[test]
fn tile_09c_first() {
    assert_tile(&refs::t09c_first::REF);
}

#[test]
fn tile_10_positioning() {
    assert_tile(&refs::t10_positioning::REF);
}

#[test]
fn tile_11_stacking() {
    assert_tile(&refs::t11_stacking::REF);
}

#[test]
fn tile_12_opacity() {
    assert_tile(&refs::t12_opacity::REF);
}

#[test]
fn tile_13_overflow() {
    assert_tile(&refs::t13_overflow::REF);
}

#[test]
fn tile_14_tables() {
    assert_tile(&refs::t14_tables::REF);
}

#[test]
fn tile_15a_forms() {
    assert_tile(&refs::t15a_forms::REF);
}

#[test]
fn tile_15b_top_layer() {
    assert_tile(&refs::t15b_top_layer::REF);
}

#[test]
fn tile_15c_modal() {
    assert_tile(&refs::t15c_modal::REF);
}

#[test]
fn tile_16_display() {
    assert_tile(&refs::t16_display::REF);
}

#[test]
fn tile_17_selection() {
    assert_tile(&refs::t17_selection::REF);
}

#[test]
fn tile_18_grid() {
    assert_tile(&refs::t18_grid::REF);
}

#[test]
fn tile_19_floats() {
    assert_tile(&refs::t19_floats::REF);
}

#[test]
fn tile_20_media() {
    assert_tile(&refs::t20_media::REF);
}

#[test]
fn tile_21_supports() {
    assert_tile(&refs::t21_supports::REF);
}

#[test]
fn tile_22_container() {
    assert_tile(&refs::t22_container::REF);
}

#[test]
fn tile_23_transforms() {
    assert_tile(&refs::t23_transforms::REF);
}

#[test]
fn tile_24_effects() {
    assert_tile(&refs::t24_effects::REF);
}

#[test]
fn tile_25_multicol() {
    assert_tile(&refs::t25_multicol::REF);
}

#[test]
fn tile_26_anchor() {
    assert_tile(&refs::t26_anchor::REF);
}

#[test]
fn tile_27_logical() {
    assert_tile(&refs::t27_logical::REF);
}

#[test]
fn tile_28_boxes() {
    assert_tile(&refs::t28_boxes::REF);
}

#[test]
fn tile_29_text() {
    assert_tile(&refs::t29_text::REF);
}

#[test]
fn tile_30_layout() {
    assert_tile(&refs::t30_layout::REF);
}

#[test]
fn tile_31_motion() {
    assert_tile(&refs::t31_motion::REF);
}

#[test]
fn tile_32_scroll() {
    assert_tile(&refs::t32_scroll::REF);
}

#[test]
fn tile_33_states() {
    assert_tile(&refs::t33_states::REF);
}

#[test]
fn tile_34_pointer() {
    assert_tile(&refs::t34_pointer::REF);
}

#[test]
fn tile_35_focus() {
    assert_tile(&refs::t35_focus::REF);
}

#[test]
fn tile_36_form_state() {
    assert_tile(&refs::t36_form_state::REF);
}

#[test]
fn tile_37_transitions() {
    assert_tile(&refs::t37_transitions::REF);
}

#[test]
fn tile_38_cssom() {
    assert_tile(&refs::t38_cssom::REF);
}

#[test]
fn tile_39_smooth_scroll() {
    assert_tile(&refs::t39_smooth_scroll::REF);
}

#[test]
fn tile_40_caret() {
    assert_tile(&refs::t40_caret::REF);
}

#[test]
fn tile_41_snap() {
    assert_tile(&refs::t41_snap::REF);
}

#[test]
fn tile_42_user_validity() {
    assert_tile(&refs::t42_user_validity::REF);
}

#[test]
fn tile_43_popovers() {
    assert_tile(&refs::t43_popovers::REF);
}

#[test]
fn tile_44_popover_motion() {
    assert_tile(&refs::t44_popover_motion::REF);
}

#[test]
fn tile_45_keyframes() {
    assert_tile(&refs::t45_keyframes::REF);
}

#[test]
fn tile_46_scroll_driven() {
    assert_tile(&refs::t46_scroll_driven::REF);
}

#[test]
fn tile_47_fold() {
    assert_tile(&refs::t47_fold::REF);
}

#[test]
fn tile_48_slide() {
    assert_tile(&refs::t48_slide::REF);
}

#[test]
fn tile_49_picker() {
    assert_tile(&refs::t49_picker::REF);
}

#[test]
fn tile_50_table_span() {
    assert_tile(&refs::t50_table_span::REF);
}

#[test]
fn tile_51_controls_rest() {
    assert_tile(&refs::t51_controls_rest::REF);
}

#[test]
fn tile_52_grid_rest() {
    assert_tile(&refs::t52_grid_rest::REF);
}

#[test]
fn tile_53_floats_rest() {
    assert_tile(&refs::t53_floats_rest::REF);
}

#[test]
fn tile_54_multicol_rest() {
    assert_tile(&refs::t54_multicol_rest::REF);
}

#[test]
fn tile_55_anchor_rest() {
    assert_tile(&refs::t55_anchor_rest::REF);
}
