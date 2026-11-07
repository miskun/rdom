//! CAPTURE phase 7 (`specs/CAPTURE.md` §Audits): every check with a page
//! that fails it (a positive fixture) and one that passes (a negative
//! one), and the checks that cannot run saying why.

use std::error::Error;

use rdom_capture::audit::{Audit, AuditRequest, DesignSystem};
use rdom_capture::{CaptureApp, Environment, Session};
use rdom_tui::App;
use rdom_tui::render::TestBackend;

/// A page from markup and CSS.
struct Page(&'static str, &'static str);

impl CaptureApp for Page {
    fn build(
        &self,
        env: &Environment,
        _: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        let (dom, _) = rdom_parser::parse(self.0)?;
        Ok(env.app(dom, rdom_css::from_css(self.1))?)
    }
}

fn audit_with(markup: &'static str, css: &'static str, request: AuditRequest) -> Audit {
    let mut s = Session::new(Page(markup, css));
    s.open(Environment::default().with_viewport(30, 8), None)
        .unwrap();
    s.audit(&request).unwrap()
}

fn audit(markup: &'static str, css: &'static str, check: &str) -> Audit {
    audit_with(markup, css, AuditRequest::of(&[check]))
}

/// The findings of `check`, by message.
fn found(a: &Audit, check: &str) -> Vec<String> {
    assert!(
        a.checked.contains(&check) || a.not_checked.iter().any(|n| n.check == check),
        "{check} neither ran nor said why: {a:#?}"
    );
    a.findings
        .iter()
        .filter(|f| f.check == check)
        .map(|f| f.message.clone())
        .collect()
}

#[test]
fn contrast() {
    let bad = audit(
        "<p>faint</p>",
        "p { color: #777777; background-color: #888888; }",
        "contrast",
    );
    let f = found(&bad, "contrast");
    assert_eq!(f.len(), 1, "{bad:#?}");
    assert!(f[0].contains("under 4.5:1"), "{f:?}");
    let good = audit(
        "<p>clear</p>",
        "p { color: #ffffff; background-color: #000000; }",
        "contrast",
    );
    assert!(found(&good, "contrast").is_empty(), "{good:#?}");
}

#[test]
fn truncation() {
    let css = ".b { width: 4; overflow: hidden; white-space: nowrap; }";
    let cut = audit(r#"<div class="b">abcdefgh</div>"#, css, "truncation");
    assert_eq!(found(&cut, "truncation").len(), 1, "{cut:#?}");
    let css = ".b { width: 4; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }";
    let ellipsis = audit(r#"<div class="b">abcdefgh</div>"#, css, "truncation");
    assert!(found(&ellipsis, "truncation").is_empty(), "{ellipsis:#?}");
    let fits = audit(
        r#"<div class="b">abc</div>"#,
        ".b { width: 4; }",
        "truncation",
    );
    assert!(found(&fits, "truncation").is_empty(), "{fits:#?}");
}

#[test]
fn occlusion() {
    let css = ".cover { position: absolute; top: 0; left: 0; width: 6; height: 1; background-color: #800000; }";
    let covered = audit(
        r#"<p>hello world</p><div class="cover"></div>"#,
        css,
        "occlusion",
    );
    assert_eq!(found(&covered, "occlusion").len(), 1, "{covered:#?}");
    let css = ".cover { position: absolute; top: 3; left: 0; width: 6; height: 1; background-color: #800000; }";
    let clear = audit(
        r#"<p>hello world</p><div class="cover"></div>"#,
        css,
        "occlusion",
    );
    assert!(found(&clear, "occlusion").is_empty(), "{clear:#?}");
}

#[test]
fn alignment() {
    let off = audit(
        r#"<div class="a">one</div><div class="b">two</div>"#,
        ".a, .b { width: 10; } .b { margin-left: 1; }",
        "alignment",
    );
    // Moved right a column, both its edges miss the box above by one.
    assert_eq!(found(&off, "alignment").len(), 2, "{off:#?}");
    let aligned = audit(
        r#"<div class="a">one</div><div class="b">two</div>"#,
        ".a, .b { width: 10; } .b { margin-left: 2; width: 8; }",
        "alignment",
    );
    assert!(found(&aligned, "alignment").is_empty(), "{aligned:#?}");
}

#[test]
fn palette() {
    let near = audit(
        r#"<p class="a">a</p><p class="b">b</p>"#,
        ".a { color: #c80000; } .b { color: #c90101; }",
        "palette",
    );
    assert_eq!(found(&near, "palette").len(), 1, "{near:#?}");
    let distinct = audit(
        r#"<p class="a">a</p><p class="b">b</p>"#,
        ".a { color: #c80000; } .b { color: #0000c8; }",
        "palette",
    );
    assert!(found(&distinct, "palette").is_empty(), "{distinct:#?}");
}

#[test]
fn depth_loss() {
    // #af0000 on #5f0000 reads (1.9:1) at 24 bits; at 16 colors both are ANSI red.
    let lost = audit(
        "<p>dim</p>",
        "p { color: #af0000; background-color: #5f0000; }",
        "depth-loss",
    );
    assert_eq!(found(&lost, "depth-loss").len(), 1, "{lost:#?}");
    let kept = audit(
        "<p>bright</p>",
        "p { color: #ffffff; background-color: #000000; }",
        "depth-loss",
    );
    assert!(found(&kept, "depth-loss").is_empty(), "{kept:#?}");
}

#[test]
fn profile() {
    let markup = r#"<p>plain <i>stress</i> plain</p>"#;
    let console = AuditRequest::of(&["profile"]).with_profile("console-16");
    let lost = audit_with(markup, "", console.clone());
    assert_eq!(found(&lost, "profile").len(), 1, "{lost:#?}");
    let drawn = audit(markup, "", "profile");
    assert!(
        found(&drawn, "profile").is_empty(),
        "xterm draws italic: {drawn:#?}"
    );
    let colored = audit_with(
        r#"<p>plain <i>stress</i> plain</p>"#,
        "i { color: #ff0000; }",
        console,
    );
    assert!(
        found(&colored, "profile").is_empty(),
        "a color sets it apart: {colored:#?}"
    );
}

#[test]
fn rhythm() {
    let css = ".a { padding-left: 3; }";
    let none = audit(r#"<div class="a">x</div>"#, css, "rhythm");
    assert!(
        none.not_checked
            .iter()
            .any(|n| n.check == "rhythm" && n.reason.contains("spacing"))
    );
    let scale = |spacing: Vec<u16>| {
        AuditRequest::of(&["rhythm"]).with_design(DesignSystem::default().with_spacing(spacing))
    };
    let off = audit_with(r#"<div class="a">x</div>"#, css, scale(vec![1, 2, 4]));
    assert_eq!(found(&off, "rhythm").len(), 1, "{off:#?}");
    let on = audit_with(r#"<div class="a">x</div>"#, css, scale(vec![1, 3]));
    assert!(found(&on, "rhythm").is_empty(), "{on:#?}");
}

#[test]
fn focus_visible() {
    let hidden = audit(
        r#"<div tabindex="0">go</div>"#,
        "div:focus, div:focus-visible { outline: none; }",
        "focus-visible",
    );
    assert_eq!(found(&hidden, "focus-visible").len(), 1, "{hidden:#?}");
    let shown = audit(
        r#"<div tabindex="0">go</div>"#,
        "div:focus-visible { background-color: #0000ff; }",
        "focus-visible",
    );
    assert!(found(&shown, "focus-visible").is_empty(), "{shown:#?}");
}

#[test]
fn reachability() {
    // Visual order: b above a; tab order (tabindex 1 first): a, then b —
    // up the screen.
    let against = audit(
        r#"<div tabindex="2">b</div><div tabindex="1">a</div>"#,
        "",
        "reachability",
    );
    assert_eq!(found(&against, "reachability").len(), 1, "{against:#?}");
    let along = audit(
        r#"<button>a</button><button>b</button>"#,
        "",
        "reachability",
    );
    assert!(found(&along, "reachability").is_empty(), "{along:#?}");
}

#[test]
fn targets() {
    let small = audit(
        r#"<button class="x">x</button>"#,
        ".x { width: 1; }",
        "targets",
    );
    assert_eq!(found(&small, "targets").len(), 1, "{small:#?}");
    let large = audit(r#"<button>press</button>"#, "", "targets");
    assert!(found(&large, "targets").is_empty(), "{large:#?}");
}

#[test]
fn landmarks() {
    let skipped = audit("<h1>a</h1><h3>c</h3>", "", "landmarks");
    assert_eq!(found(&skipped, "landmarks").len(), 1, "{skipped:#?}");
    let unnamed = audit("<nav>x</nav><nav>y</nav>", "", "landmarks");
    assert_eq!(found(&unnamed, "landmarks").len(), 1, "{unnamed:#?}");
    let fine = audit(
        r#"<h1>a</h1><h2>b</h2><nav aria-label="main">x</nav><nav aria-label="side">y</nav>"#,
        "",
        "landmarks",
    );
    assert!(found(&fine, "landmarks").is_empty(), "{fine:#?}");
}

#[test]
fn tokens() {
    let css = ":root { --fg: #c0c0c0; --bg: #101010; } p { color: var(--fg); background-color: var(--bg); } .odd { color: #c1c1c1; }";
    let off = audit(r#"<p>ok</p><p class="odd">off</p>"#, css, "tokens");
    let f = found(&off, "tokens");
    assert_eq!(f.len(), 1, "{off:#?}");
    assert!(f[0].contains("#c1c1c1") && f[0].contains("--fg"), "{f:?}");
    let on = audit(r#"<p>ok</p>"#, css, "tokens");
    assert!(found(&on, "tokens").is_empty(), "{on:#?}");
    let none = audit("<p>x</p>", "", "tokens");
    assert!(
        none.not_checked.iter().any(|n| n.check == "tokens"),
        "{none:#?}"
    );
}

#[test]
fn a_full_audit_runs_every_check_or_says_why_not() {
    let a = audit_with("<p>hello</p>", "", AuditRequest::all());
    for check in rdom_capture::audit::CHECKS {
        assert!(
            a.checked.contains(&check) || a.not_checked.iter().any(|n| n.check == check),
            "{check}"
        );
    }
    let bad = Session::new(Page("<p>x</p>", ""));
    let mut bad = bad;
    bad.open(Environment::default(), None).unwrap();
    assert!(bad.audit(&AuditRequest::of(&["nope"])).is_err());
}

#[test]
fn whatif_ranks_candidates_by_the_findings_they_fix() {
    use rdom_capture::variants::whatif::Candidate;
    use rdom_capture::{EditTarget, Step};
    let mut s = Session::new(Page(
        r#"<p class="a">faint</p><p class="b">fine</p>"#,
        ".a { color: #777777; background-color: #888888; } .b { color: #ffffff; }",
    ));
    s.open(Environment::default().with_viewport(30, 8), None)
        .unwrap();
    let set = |selector: &str, value: &str| Step::Edit {
        target: EditTarget::NewRule {
            selector: selector.into(),
        },
        property: "color".into(),
        value: Some(value.into()),
        important: true,
    };
    let outcomes = s
        .whatif(
            &[
                Candidate::new("worse", vec![set(".b", "#111111")]),
                Candidate::new("fix", vec![set(".a", "#000000")]),
            ],
            0,
            Some(&AuditRequest::of(&["contrast"])),
        )
        .unwrap();
    assert_eq!(outcomes[0].name, "fix", "{outcomes:#?}");
    let fix = outcomes[0].findings.as_ref().unwrap();
    assert_eq!(
        (fix.score, fix.fixed.len(), fix.introduced.len()),
        (1, 1, 0)
    );
    let worse = outcomes[1].findings.as_ref().unwrap();
    assert_eq!(worse.score, -1, "{worse:#?}");
}
