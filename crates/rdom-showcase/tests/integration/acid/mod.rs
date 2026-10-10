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

use compare::{TileReport, check, check_on, paint_page};
use rdom_showcase::demos::acid;
use reference::Reference;

fn assert_tile(reference: &'static Reference) {
    let report = check(reference);
    assert!(report.passed(), "\n{}", report.failure());
}

/// Every tile, page by page: a line per tile, then each failure's report.
#[test]
fn report() {
    let mut reports: Vec<TileReport> = Vec::new();
    for page in 1..=acid::page_count() {
        let screen = paint_page(page);
        for reference in refs::ALL {
            if acid::TILES
                .iter()
                .any(|t| t.id == reference.tile && t.page == page)
            {
                reports.push(check_on(&screen, reference));
            }
        }
    }
    let mut out = String::from("\nacid static tiles:\n");
    for r in &reports {
        out.push_str(&format!(
            "  {:>3} {:<36} {}\n",
            r.tile.id,
            r.tile.title,
            if r.passed() {
                "pass".to_string()
            } else if let Some(e) = &r.error {
                format!("ERROR {e}")
            } else {
                format!("FAIL ({} cells)", r.diffs.len())
            }
        ));
    }
    let failed: Vec<&TileReport> = reports.iter().filter(|r| !r.passed()).collect();
    for r in &failed {
        out.push('\n');
        out.push_str(&r.failure());
    }
    assert!(failed.is_empty(), "{out}");
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
