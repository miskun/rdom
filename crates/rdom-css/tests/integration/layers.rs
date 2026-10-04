//! `@layer` (CSS Cascade 5 §6.4.1): the statement form declares layer
//! names, the block form (named or anonymous) parses its rules into a
//! layer, `a.b` and nested blocks name sublayers, and an invalid
//! prelude drops the rule with a warning while the rules around it
//! survive.

use rdom_css::{WarningKind, parse};

fn selectors(src: &str) -> Vec<String> {
    parse(src)
        .stylesheet
        .rules()
        .iter()
        .map(|r| r.source_text.clone())
        .collect()
}

/// Block and statement forms parse; their rules are kept, in order.
#[test]
fn layer_blocks_keep_their_rules() {
    let src = "@layer reset, base; @layer base { p { color: red } } \
               @layer { q { color: blue } } @LAYER a.b { @layer c { r {} } } s {}";
    let r = parse(src);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(selectors(src), vec!["p", "q", "r", "s"]);
}

fn layer_path(sheet: &rdom_style::Stylesheet, rule: usize) -> Option<String> {
    let mut id = sheet.rules()[rule].layer?;
    let mut segments = Vec::new();
    loop {
        let layer = &sheet.layers()[id.index()];
        segments.push(layer.name.clone().unwrap_or_else(|| "<anon>".into()));
        match layer.parent {
            Some(p) => id = p,
            None => break,
        }
    }
    segments.reverse();
    Some(segments.join("."))
}

/// §6.4.1: the statement form declares names in order (a later
/// mention is the same layer); `a.b` and a nested block both name the
/// sublayer `b` of `a`; each anonymous block is a new layer.
#[test]
fn rules_record_their_layer() {
    let r = parse(
        "@layer b, a; @layer a { p {} } @layer a.c { q {} } \
         @layer a { @layer c { r {} } } @layer { s {} } @layer { t {} } u {}",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let sheet = &r.stylesheet;
    let names: Vec<Option<String>> = (0..sheet.rules().len())
        .map(|i| layer_path(sheet, i))
        .collect();
    assert_eq!(
        names,
        vec![
            Some("a".into()),
            Some("a.c".into()),
            Some("a.c".into()),
            Some("<anon>".into()),
            Some("<anon>".into()),
            None,
        ]
    );
    assert_ne!(sheet.rules()[3].layer, sheet.rules()[4].layer);
    let top: Vec<Option<&str>> = sheet
        .layers()
        .iter()
        .filter(|l| l.parent.is_none())
        .map(|l| l.name.as_deref())
        .collect();
    assert_eq!(top, vec![Some("b"), Some("a"), None, None]);
}

/// An invalid prelude — two names in a block, a missing comma,
/// whitespace around a dot, a reserved CSS-wide keyword, a non-ident —
/// drops the whole rule with a warning; the rules around it survive.
#[test]
fn invalid_layer_preludes_drop_the_rule() {
    for src in [
        "@layer a, b { x {} } y {}",
        "@layer a b; y {}",
        "@layer a .b { x {} } y {}",
        "@layer revert; y {}",
        "@layer \"a\" { x {} } y {}",
        "@layer a,; y {}",
    ] {
        let r = parse(src);
        assert_eq!(selectors(src), vec!["y"], "{src}");
        assert!(
            matches!(
                r.warnings.as_slice(),
                [w] if matches!(&w.kind, WarningKind::InvalidAtRulePrelude { name, .. } if name == "layer")
            ),
            "{src}: {:?}",
            r.warnings
        );
    }
}

/// CSS Syntax 3 §5.4.7: EOF closes an open `@layer` block, keeping
/// its rules.
#[test]
fn eof_closes_a_layer_block() {
    let src = "@layer a { p { color: red }";
    let r = parse(src);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(selectors(src), vec!["p"]);
    assert!(r.stylesheet.rules()[0].layer.is_some());
}
