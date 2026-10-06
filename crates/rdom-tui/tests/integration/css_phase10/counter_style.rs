//! C10-COUNTER-STYLE — CSS Counter Styles 3: author `@counter-style`
//! rules (§3) and `symbols()` (§5), in `counter()` / `counters()`.

use super::{el, paint_tree, text_el};

/// Items `.i` under `.l` (resetting `c`), each `counter-increment: c`
/// and `::before { content: <before> "|" }`, `n` of them in `w` cells.
fn items(css: &str, before: &str, n: usize, w: u16) -> Vec<String> {
    let css = format!(
        ".l {{ counter-reset: c }} .i {{ display: block; counter-increment: c }}
         .i::before {{ content: {before} \"|\" }} {css}"
    );
    paint_tree(&css, w, n as u16, |dom, root| {
        let l = el(dom, root, "div", "l");
        for _ in 0..n {
            text_el(dom, l, "div", "i", "");
        }
    })
    .into_iter()
    .map(|r| r.trim_end().to_string())
    .collect()
}

/// §3.1.1 cyclic, §3.1.2 fixed (with its fallback past the end), §3.1.3
/// symbolic, §3.1.5 alphabetic, §3.1.4 numeric, §3.1.6 additive: an
/// author style of each system, named in `counter()`.
#[test]
fn author_styles_of_every_system() {
    let css = r#"
        @counter-style cyc { system: cyclic; symbols: "*" "+" }
        @counter-style fix { system: fixed 2; symbols: A B; fallback: lower-roman }
        @counter-style sym { system: symbolic; symbols: "*" "+" }
        @counter-style alp { system: alphabetic; symbols: x y }
        @counter-style num { system: numeric; symbols: "0" "1" }
        @counter-style add { system: additive; additive-symbols: 5 V, 1 I }"#;
    let col = |style: &str| -> Vec<String> {
        items(css, &format!("counter(c, {style})"), 5, 10)
            .into_iter()
            .map(|r| r.trim_end_matches('|').to_string())
            .collect()
    };
    assert_eq!(col("cyc"), ["*", "+", "*", "+", "*"]);
    assert_eq!(col("fix"), ["i", "A", "B", "iv", "v"]);
    assert_eq!(col("sym"), ["*", "+", "**", "++", "***"]);
    assert_eq!(col("alp"), ["x", "y", "xx", "xy", "yx"]);
    assert_eq!(col("num"), ["1", "10", "11", "100", "101"]);
    assert_eq!(col("add"), ["I", "II", "III", "IIII", "V"]);
}

/// §3.1.7 `extends`: the base's system and symbols with the rule's
/// descriptors over them; `negative`, `pad` and `range` apply to the
/// counter's text (§3.4–§3.6), `prefix` / `suffix` only to markers.
#[test]
fn extends_negative_pad_and_range() {
    let css = r#"
        @counter-style paren { system: extends decimal; negative: "(" ")"; pad: 3 "0";
                               prefix: "<"; suffix: ">"; range: -10 2 }
        .l { counter-reset: c -2 }"#;
    let rows = items(css, "counter(c, paren)", 5, 10);
    assert_eq!(rows, ["(1)|", "000|", "001|", "002|", "3|"]);
}

/// §5 `symbols()`: an anonymous style — symbolic by default, any other
/// simple system named first.
#[test]
fn the_symbols_function() {
    let rows = items("", r#"counter(c, symbols("*" "†"))"#, 3, 8);
    assert_eq!(rows, ["*|", "†|", "**|"]);
    let rows = items("", r#"counter(c, symbols(cyclic "a" "b"))"#, 3, 8);
    assert_eq!(rows, ["a|", "b|", "a|"]);
    let rows = items("", r#"counters(c, ".", symbols(numeric "0" "1"))"#, 2, 8);
    assert_eq!(rows, ["1|", "10|"]);
}

/// §3: the last definition of a name wins (by cascade layer first); a
/// predefined style other
/// than the protected six may be redefined; `decimal` may not (the rule
/// is dropped, so `strict` parsing refuses the sheet — not asserted
/// here; the parser tests pin it).
#[test]
fn the_last_definition_wins_and_predefined_styles_can_be_redefined() {
    let css = r#"
        @counter-style x { system: cyclic; symbols: A }
        @counter-style x { system: cyclic; symbols: B }
        @counter-style lower-roman { system: cyclic; symbols: R }"#;
    assert_eq!(items(css, "counter(c, x)", 1, 6), ["B|"]);
    assert_eq!(items(css, "counter(c, lower-roman)", 1, 6), ["R|"]);
    // CSS Cascade 5 §6.4.3: an unlayered definition beats a later layered one.
    let layered = r#"
        @counter-style y { system: cyclic; symbols: U }
        @layer late { @counter-style y { system: cyclic; symbols: L } }"#;
    assert_eq!(items(layered, "counter(c, y)", 1, 6), ["U|"]);
}

/// §3.7: a fallback chain that loops ends at `decimal`; §3.1.7: an
/// `extends` cycle extends `decimal`. A huge value in a symbolic or
/// additive style falls back rather than building past the 60-code-point
/// cap (§3.1).
#[test]
fn cycles_and_huge_values_are_bounded() {
    let css = r#"
        @counter-style a { system: fixed; symbols: A; fallback: b }
        @counter-style b { system: fixed; symbols: B; fallback: a }
        @counter-style e1 { system: extends e2; suffix: "!" }
        @counter-style e2 { system: extends e1 }
        @counter-style many { system: symbolic; symbols: "*" }
        @counter-style ones { system: additive; additive-symbols: 1 "|" }
        .l { counter-reset: c 99999 }"#;
    let one = |style: &str| items(css, &format!("counter(c, {style})"), 1, 12);
    assert_eq!(one("a"), ["100000|"]);
    assert_eq!(one("e1"), ["100000|"]);
    assert_eq!(one("many"), ["100000|"]);
    assert_eq!(one("ones"), ["100000|"]);
}
