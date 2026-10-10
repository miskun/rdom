//! The acid test, stage 2: the interactive script (`specs/ACID.md`
//! Stage 2).
//!
//! Each step drives one acid page in a fresh headless `App` with scripted
//! input on a controllable clock and compares the tiles it affects
//! against per-checkpoint references derived from the spec
//! ([`session`] has the driver and the fresh-page decision). One test
//! per step (`acid::interactive::step_i01_…`), plus `report`, which runs
//! every step and lists each one's result.

mod session;
mod steps;

use session::{Session, Step, StepResult, run};

use super::reference::Reference;

fn assert_step(step: &'static Step) {
    let result = run(step);
    assert!(result.passed(), "\n{}", result.failure());
}

/// Every step's result: a line per step, then each failure's report.
pub fn results() -> Vec<StepResult> {
    steps::ALL
        .iter()
        .map(|step| {
            std::panic::catch_unwind(|| run(step)).unwrap_or_else(|panic| {
                let why = panic
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                StepResult {
                    step,
                    checkpoints: 0,
                    failures: vec![format!("step {} panicked: {why}", step.id)],
                }
            })
        })
        .collect()
}

/// Each step as one summary line.
pub fn summary_line(r: &StepResult) -> String {
    format!(
        "  {:>4} {:<44} {}\n",
        r.step.id,
        r.step.title,
        if r.passed() {
            format!("pass ({} checkpoints)", r.checkpoints)
        } else {
            format!(
                "FAIL ({} of {} checkpoints)",
                r.failures.len(),
                r.checkpoints
            )
        }
    )
}

#[test]
fn report() {
    let results = results();
    let mut out = String::from("\nacid interactive steps:\n");
    for r in &results {
        out.push_str(&summary_line(r));
    }
    let failed: Vec<&StepResult> = results.iter().filter(|r| !r.passed()).collect();
    for r in &failed {
        out.push('\n');
        out.push_str(&r.failure());
    }
    assert!(failed.is_empty(), "{out}");
}

/// Every step drives an existing page, and steps keep `ACID.md`'s ids.
#[test]
fn steps_are_numbered_and_on_pages() {
    for (i, step) in steps::ALL.iter().enumerate() {
        assert!(step.id.starts_with('I'), "step {i}: id {:?}", step.id);
        assert!(
            (1..=rdom_showcase::demos::acid::page_count()).contains(&step.page),
            "step {}: no page {}",
            step.id,
            step.page
        );
        assert!(!step.spec.is_empty(), "step {} cites no spec", step.id);
    }
}

/// The harness's own case: a checkpoint whose reference expects a wrong
/// cell, and a fact check that does not hold, each fail — the report names
/// the step, the checkpoint, the tile, the cell and the spec — and the
/// session goes on to the next checkpoint.
#[test]
fn harness_reports_step_checkpoint_tile_and_cells() {
    static WRONG: Reference = Reference {
        tile: "34",
        spec: &["test spec §0"],
        legend: &[('R', "bg #ff0000")],
        grid: r#"
|abkidcd sib far                       |
|R.....................................|
|press log:                            |
|......................................|
|hit blk 0 0                           |
|......................................|
"#,
    };
    fn wrong(s: &mut Session) {
        s.expect("at rest", &[&WRONG]);
        s.check("a fact", false, "it did not hold", "test spec §1");
        s.expect("still at rest", &[&super::refs::t34_pointer::REF]);
    }
    static STEP: Step = Step {
        id: "I0",
        title: "Harness self-test",
        page: 10,
        spec: &["test step spec"],
        run: wrong,
        configure: None,
    };
    let result = run(&STEP);
    assert_eq!(result.checkpoints, 3);
    assert_eq!(result.failures.len(), 2, "{}", result.failure());
    let paint = &result.failures[0];
    assert!(paint.starts_with("step I0 \"Harness self-test\" (page 10; test step spec)"));
    assert!(paint.contains("checkpoint 1 \"at rest\""), "{paint}");
    assert!(
        paint.contains("tile 34 \"Hover, press, pointer-events\""),
        "{paint}"
    );
    assert!(paint.contains("test spec §0"), "{paint}");
    assert!(paint.contains("1 cells differ"), "{paint}");
    assert!(
        paint.contains("(0, 0) expected \"a\" fg default bg #ff0000"),
        "{paint}"
    );
    let fact = &result.failures[1];
    assert!(
        fact.contains("checkpoint 2 \"a fact\" — spec: test spec §1"),
        "{fact}"
    );
    assert!(fact.contains("seen: it did not hold"), "{fact}");
}

#[test]
fn step_i01_hover() {
    assert_step(&steps::i01_hover::STEP);
}

#[test]
fn step_i02_active() {
    assert_step(&steps::i02_active::STEP);
}

#[test]
fn step_i03_focus() {
    assert_step(&steps::i03_focus::STEP);
}

#[test]
fn step_i04_validity() {
    assert_step(&steps::i04_validity::STEP);
}

#[test]
fn step_i05_toggles() {
    assert_step(&steps::i05_toggles::STEP);
}

#[test]
fn step_i06_transitions() {
    assert_step(&steps::i06_transitions::STEP);
}

#[test]
fn step_i07_cssom() {
    assert_step(&steps::i07_cssom::STEP);
}

#[test]
fn step_i08_smooth_scroll() {
    assert_step(&steps::i08_smooth_scroll::STEP);
}

#[test]
fn step_i09_caret() {
    assert_step(&steps::i09_caret::STEP);
}

#[test]
fn step_i10_pointer_events() {
    assert_step(&steps::i10_pointer_events::STEP);
}

#[test]
fn step_i11_snap() {
    assert_step(&steps::i11_snap::STEP);
}

#[test]
fn step_i12_pseudo() {
    assert_step(&steps::i12_pseudo::STEP);
}

#[test]
fn step_i13_invalidation() {
    assert_step(&steps::i13_invalidation::STEP);
}

#[test]
fn step_i14_user_validity() {
    assert_step(&steps::i14_user_validity::STEP);
}

#[test]
fn step_i15_light_dismiss() {
    assert_step(&steps::i15_light_dismiss::STEP);
}

#[test]
fn step_i16_popover_motion() {
    assert_step(&steps::i16_popover_motion::STEP);
}

#[test]
fn step_i17_keyframes() {
    assert_step(&steps::i17_keyframes::STEP);
}

#[test]
fn step_i18_scroll_driven() {
    assert_step(&steps::i18_scroll_driven::STEP);
}

#[test]
fn step_i19_resize() {
    assert_step(&steps::i19_resize::STEP);
}

#[test]
fn step_i20_slide() {
    assert_step(&steps::i20_slide::STEP);
}

#[test]
fn step_i21_anchor_resize() {
    assert_step(&steps::i21_anchor_resize::STEP);
}

#[test]
fn step_i22_clip_hit() {
    assert_step(&steps::i22_clip_hit::STEP);
}

#[test]
fn step_i23_fragments() {
    assert_step(&steps::i23_fragments::STEP);
}

#[test]
fn step_i24_overflow_hit() {
    assert_step(&steps::i24_overflow_hit::STEP);
}
