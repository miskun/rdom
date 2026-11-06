//! CAPTURE phase 3: every rule records where it is written — the text it
//! came from and the line and column of its selector — so a tool can point
//! at the line to edit (`rdom_style::SourceLocation`).

use rdom_css::{parse, parse_at, parse_with_loader_at};
use rdom_style::{SourceFile, SourceLocation, Stylesheet, TuiStyle};

fn located(sheet: &Stylesheet) -> Vec<(String, Option<String>, u32, u32)> {
    sheet
        .rules()
        .iter()
        .map(|r| {
            let l = r.location.expect("every parsed rule has a location");
            let file = match l.file {
                SourceFile::Css(i) => sheet.source_name(i).map(str::to_string),
                other => panic!("a parsed rule is in CSS text: {other:?}"),
            };
            (r.source_text.clone(), file, l.line, l.column)
        })
        .collect()
}

#[test]
fn a_rule_records_its_selectors_line_and_column() {
    let css = "a { color: red; }\n\n  .panel,\n  .card { color: blue; }\n";
    let got = located(&parse_at(css, "app.css").stylesheet);
    let name = Some("app.css".to_string());
    assert_eq!(
        got,
        [
            ("a".to_string(), name.clone(), 1, 1),
            (".panel".to_string(), name.clone(), 3, 3),
            (".card".to_string(), name, 3, 3),
        ],
        "a selector list's rules share the list's position"
    );
}

#[test]
fn unnamed_text_has_a_source_with_no_name() {
    let sheet = parse("b { color: red; }").stylesheet;
    let l = sheet.rules()[0].location.unwrap();
    assert_eq!((l.line, l.column), (1, 1));
    let SourceFile::Css(i) = l.file else { panic!() };
    assert_eq!(sheet.source_name(i), None);
}

#[test]
fn a_nested_rule_records_its_own_position() {
    let css = ".card {\n  color: red;\n  & .title { color: blue; }\n}\n";
    let got = located(&parse_at(css, "n.css").stylesheet);
    let positions: Vec<(u32, u32)> = got.iter().map(|g| (g.2, g.3)).collect();
    assert!(positions.contains(&(1, 1)), "{got:?}");
    assert!(positions.contains(&(3, 3)), "{got:?}");
}

#[test]
fn an_imported_rule_records_the_imported_sheet() {
    let loader = |url: &str| match url {
        "theme.css" => Ok("\n.theme { color: red; }".to_string()),
        _ => Err("no such sheet".to_string()),
    };
    let sheet = parse_with_loader_at(
        "@import 'theme.css';\np { color: blue; }",
        "main.css",
        &loader,
    )
    .stylesheet;
    let got = located(&sheet);
    assert!(
        got.contains(&(".theme".into(), Some("theme.css".into()), 2, 1)),
        "{got:?}"
    );
    assert!(
        got.contains(&("p".into(), Some("main.css".into()), 2, 1)),
        "{got:?}"
    );
}

#[test]
fn appending_a_sheet_keeps_its_rules_sources() {
    let mut sheet = parse_at("a { color: red; }", "one.css").stylesheet;
    sheet.append(&parse_at("\nb { color: blue; }", "two.css").stylesheet);
    let got = located(&sheet);
    assert_eq!(got[0].1.as_deref(), Some("one.css"));
    assert_eq!((got[1].1.as_deref(), got[1].2), (Some("two.css"), 2));
    // `from_css_at` appends onto the UA sheet: the UA rules have none.
    let full = rdom_css::from_css_at("a { color: red; }", "app.css");
    assert!(
        full.rules().iter().any(|r| r.location.is_none()),
        "UA rules record none"
    );
    let a = full.rules().iter().find(|r| r.location.is_some()).unwrap();
    let SourceFile::Css(i) = a.location.unwrap().file else {
        panic!()
    };
    assert_eq!(full.source_name(i), Some("app.css"));
}

#[test]
fn a_rule_built_in_rust_records_the_call() {
    let line = line!() + 1;
    let sheet = Stylesheet::bare().rule_unchecked(".x", TuiStyle::new());
    let l: SourceLocation = sheet.rules()[0].location.unwrap();
    assert_eq!(l.file, SourceFile::Rust(file!()));
    assert_eq!(l.line, line);
}
