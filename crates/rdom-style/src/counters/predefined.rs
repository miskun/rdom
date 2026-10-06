//! The simple predefined counter styles (CSS Counter Styles 3 §6),
//! each defined by the descriptors an `@counter-style` rule would give
//! it (the spec's own definitions): numeric (§6.1, every script's ten
//! digits), alphabetic (§6.2), symbolic (§6.3), the additive ones
//! (roman, Hebrew, Armenian, Georgian) and the fixed CJK cycles. The
//! complex styles of §7 (Ethiopic, the CJK longhand forms) are not
//! defined, so they format as `decimal`.
//!
//! The table is data: each style's symbols are one string of
//! one-character symbols, an additive style's tuples one string of
//! `weight symbol` pairs.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::rule::{CounterStyleRule, System};

/// The names of the predefined counter styles, sorted.
pub fn predefined_names() -> &'static [&'static str] {
    NAMES
}

#[rustfmt::skip]
const NAMES: &[&str] = &[
    "arabic-indic",
    "armenian",
    "bengali",
    "cambodian",
    "circle",
    "cjk-decimal",
    "cjk-earthly-branch",
    "cjk-heavenly-stem",
    "decimal",
    "decimal-leading-zero",
    "devanagari",
    "disc",
    "disclosure-closed",
    "disclosure-open",
    "georgian",
    "gujarati",
    "gurmukhi",
    "hebrew",
    "hiragana",
    "hiragana-iroha",
    "kannada",
    "katakana",
    "katakana-iroha",
    "khmer",
    "lao",
    "lower-alpha",
    "lower-armenian",
    "lower-greek",
    "lower-latin",
    "lower-roman",
    "malayalam",
    "mongolian",
    "myanmar",
    "oriya",
    "persian",
    "square",
    "tamil",
    "telugu",
    "thai",
    "tibetan",
    "upper-alpha",
    "upper-armenian",
    "upper-latin",
    "upper-roman",
];

/// The predefined style named `name` (lowercase), if any. Built once.
pub fn predefined(name: &str) -> Option<&'static CounterStyleRule> {
    static TABLE: OnceLock<HashMap<&'static str, CounterStyleRule>> = OnceLock::new();
    TABLE.get_or_init(table).get(name)
}

/// A rule of `system` whose symbols are the characters of `symbols`.
fn chars(system: System, symbols: &str) -> CounterStyleRule {
    let symbols: Vec<String> = symbols.chars().map(String::from).collect();
    let symbols: Vec<&str> = symbols.iter().map(String::as_str).collect();
    CounterStyleRule::new(system, &symbols)
}

/// An additive rule from `"weight symbol weight symbol …"`, heaviest
/// first.
fn weighted(pairs: &str) -> CounterStyleRule {
    let words: Vec<&str> = pairs.split(' ').collect();
    let tuples: Vec<(u32, &str)> = words
        .chunks(2)
        .map(|p| (p[0].parse().expect("a table weight"), p[1]))
        .collect();
    CounterStyleRule::additive(&tuples)
}

#[rustfmt::skip]
fn table() -> HashMap<&'static str, CounterStyleRule> {
    HashMap::from([
        ("arabic-indic", chars(System::Numeric, "\u{660}\u{661}\u{662}\u{663}\u{664}\u{665}\u{666}\u{667}\u{668}\u{669}")),
        ("bengali", chars(System::Numeric, "\u{9e6}\u{9e7}\u{9e8}\u{9e9}\u{9ea}\u{9eb}\u{9ec}\u{9ed}\u{9ee}\u{9ef}")),
        ("cambodian", chars(System::Numeric, "\u{17e0}\u{17e1}\u{17e2}\u{17e3}\u{17e4}\u{17e5}\u{17e6}\u{17e7}\u{17e8}\u{17e9}")),
        ("circle", chars(System::Cyclic, "\u{25e6}").with_suffix(" ")),
        ("cjk-decimal", chars(System::Numeric, "\u{3007}\u{4e00}\u{4e8c}\u{4e09}\u{56db}\u{4e94}\u{516d}\u{4e03}\u{516b}\u{4e5d}").with_suffix("\u{3001}")),
        ("cjk-earthly-branch", chars(System::Fixed(1), "\u{5b50}\u{4e11}\u{5bc5}\u{536f}\u{8fb0}\u{5df3}\u{5348}\u{672a}\u{7533}\u{9149}\u{620c}\u{4ea5}").with_suffix("\u{3001}").with_fallback("cjk-decimal")),
        ("cjk-heavenly-stem", chars(System::Fixed(1), "\u{7532}\u{4e59}\u{4e19}\u{4e01}\u{620a}\u{5df1}\u{5e9a}\u{8f9b}\u{58ec}\u{7678}").with_suffix("\u{3001}").with_fallback("cjk-decimal")),
        ("decimal", chars(System::Numeric, "0123456789")),
        ("decimal-leading-zero", chars(System::Numeric, "0123456789").with_pad(2, "0")),
        ("devanagari", chars(System::Numeric, "\u{966}\u{967}\u{968}\u{969}\u{96a}\u{96b}\u{96c}\u{96d}\u{96e}\u{96f}")),
        ("disc", chars(System::Cyclic, "\u{2022}").with_suffix(" ")),
        ("disclosure-closed", chars(System::Cyclic, "\u{25b8}").with_suffix(" ")),
        ("disclosure-open", chars(System::Cyclic, "\u{25be}").with_suffix(" ")),
        ("gujarati", chars(System::Numeric, "\u{ae6}\u{ae7}\u{ae8}\u{ae9}\u{aea}\u{aeb}\u{aec}\u{aed}\u{aee}\u{aef}")),
        ("gurmukhi", chars(System::Numeric, "\u{a66}\u{a67}\u{a68}\u{a69}\u{a6a}\u{a6b}\u{a6c}\u{a6d}\u{a6e}\u{a6f}")),
        ("hiragana", chars(System::Alphabetic, "\u{3042}\u{3044}\u{3046}\u{3048}\u{304a}\u{304b}\u{304d}\u{304f}\u{3051}\u{3053}\u{3055}\u{3057}\u{3059}\u{305b}\u{305d}\u{305f}\u{3061}\u{3064}\u{3066}\u{3068}\u{306a}\u{306b}\u{306c}\u{306d}\u{306e}\u{306f}\u{3072}\u{3075}\u{3078}\u{307b}\u{307e}\u{307f}\u{3080}\u{3081}\u{3082}\u{3084}\u{3086}\u{3088}\u{3089}\u{308a}\u{308b}\u{308c}\u{308d}\u{308f}\u{3090}\u{3091}\u{3092}\u{3093}").with_suffix("\u{3001}")),
        ("hiragana-iroha", chars(System::Alphabetic, "\u{3044}\u{308d}\u{306f}\u{306b}\u{307b}\u{3078}\u{3068}\u{3061}\u{308a}\u{306c}\u{308b}\u{3092}\u{308f}\u{304b}\u{3088}\u{305f}\u{308c}\u{305d}\u{3064}\u{306d}\u{306a}\u{3089}\u{3080}\u{3046}\u{3090}\u{306e}\u{304a}\u{304f}\u{3084}\u{307e}\u{3051}\u{3075}\u{3053}\u{3048}\u{3066}\u{3042}\u{3055}\u{304d}\u{3086}\u{3081}\u{307f}\u{3057}\u{3091}\u{3072}\u{3082}\u{305b}\u{3059}").with_suffix("\u{3001}")),
        ("kannada", chars(System::Numeric, "\u{ce6}\u{ce7}\u{ce8}\u{ce9}\u{cea}\u{ceb}\u{cec}\u{ced}\u{cee}\u{cef}")),
        ("katakana", chars(System::Alphabetic, "\u{30a2}\u{30a4}\u{30a6}\u{30a8}\u{30aa}\u{30ab}\u{30ad}\u{30af}\u{30b1}\u{30b3}\u{30b5}\u{30b7}\u{30b9}\u{30bb}\u{30bd}\u{30bf}\u{30c1}\u{30c4}\u{30c6}\u{30c8}\u{30ca}\u{30cb}\u{30cc}\u{30cd}\u{30ce}\u{30cf}\u{30d2}\u{30d5}\u{30d8}\u{30db}\u{30de}\u{30df}\u{30e0}\u{30e1}\u{30e2}\u{30e4}\u{30e6}\u{30e8}\u{30e9}\u{30ea}\u{30eb}\u{30ec}\u{30ed}\u{30ef}\u{30f0}\u{30f1}\u{30f2}\u{30f3}").with_suffix("\u{3001}")),
        ("katakana-iroha", chars(System::Alphabetic, "\u{30a4}\u{30ed}\u{30cf}\u{30cb}\u{30db}\u{30d8}\u{30c8}\u{30c1}\u{30ea}\u{30cc}\u{30eb}\u{30f2}\u{30ef}\u{30ab}\u{30e8}\u{30bf}\u{30ec}\u{30bd}\u{30c4}\u{30cd}\u{30ca}\u{30e9}\u{30e0}\u{30a6}\u{30f0}\u{30ce}\u{30aa}\u{30af}\u{30e4}\u{30de}\u{30b1}\u{30d5}\u{30b3}\u{30a8}\u{30c6}\u{30a2}\u{30b5}\u{30ad}\u{30e6}\u{30e1}\u{30df}\u{30b7}\u{30f1}\u{30d2}\u{30e2}\u{30bb}\u{30b9}").with_suffix("\u{3001}")),
        ("khmer", chars(System::Numeric, "\u{17e0}\u{17e1}\u{17e2}\u{17e3}\u{17e4}\u{17e5}\u{17e6}\u{17e7}\u{17e8}\u{17e9}")),
        ("lao", chars(System::Numeric, "\u{ed0}\u{ed1}\u{ed2}\u{ed3}\u{ed4}\u{ed5}\u{ed6}\u{ed7}\u{ed8}\u{ed9}")),
        ("lower-alpha", chars(System::Alphabetic, "abcdefghijklmnopqrstuvwxyz")),
        ("lower-greek", chars(System::Alphabetic, "\u{3b1}\u{3b2}\u{3b3}\u{3b4}\u{3b5}\u{3b6}\u{3b7}\u{3b8}\u{3b9}\u{3ba}\u{3bb}\u{3bc}\u{3bd}\u{3be}\u{3bf}\u{3c0}\u{3c1}\u{3c3}\u{3c4}\u{3c5}\u{3c6}\u{3c7}\u{3c8}\u{3c9}")),
        ("lower-latin", chars(System::Alphabetic, "abcdefghijklmnopqrstuvwxyz")),
        ("malayalam", chars(System::Numeric, "\u{d66}\u{d67}\u{d68}\u{d69}\u{d6a}\u{d6b}\u{d6c}\u{d6d}\u{d6e}\u{d6f}")),
        ("mongolian", chars(System::Numeric, "\u{1810}\u{1811}\u{1812}\u{1813}\u{1814}\u{1815}\u{1816}\u{1817}\u{1818}\u{1819}")),
        ("myanmar", chars(System::Numeric, "\u{1040}\u{1041}\u{1042}\u{1043}\u{1044}\u{1045}\u{1046}\u{1047}\u{1048}\u{1049}")),
        ("oriya", chars(System::Numeric, "\u{b66}\u{b67}\u{b68}\u{b69}\u{b6a}\u{b6b}\u{b6c}\u{b6d}\u{b6e}\u{b6f}")),
        ("persian", chars(System::Numeric, "\u{6f0}\u{6f1}\u{6f2}\u{6f3}\u{6f4}\u{6f5}\u{6f6}\u{6f7}\u{6f8}\u{6f9}")),
        ("square", chars(System::Cyclic, "\u{25aa}").with_suffix(" ")),
        ("tamil", chars(System::Numeric, "\u{be6}\u{be7}\u{be8}\u{be9}\u{bea}\u{beb}\u{bec}\u{bed}\u{bee}\u{bef}")),
        ("telugu", chars(System::Numeric, "\u{c66}\u{c67}\u{c68}\u{c69}\u{c6a}\u{c6b}\u{c6c}\u{c6d}\u{c6e}\u{c6f}")),
        ("thai", chars(System::Numeric, "\u{e50}\u{e51}\u{e52}\u{e53}\u{e54}\u{e55}\u{e56}\u{e57}\u{e58}\u{e59}")),
        ("tibetan", chars(System::Numeric, "\u{f20}\u{f21}\u{f22}\u{f23}\u{f24}\u{f25}\u{f26}\u{f27}\u{f28}\u{f29}")),
        ("upper-alpha", chars(System::Alphabetic, "ABCDEFGHIJKLMNOPQRSTUVWXYZ")),
        ("upper-latin", chars(System::Alphabetic, "ABCDEFGHIJKLMNOPQRSTUVWXYZ")),
        ("lower-roman", weighted("1000 m 900 cm 500 d 400 cd 100 c 90 xc 50 l 40 xl 10 x 9 ix 5 v 4 iv 1 i").with_range(1, 3999)),
        ("upper-roman", weighted("1000 M 900 CM 500 D 400 CD 100 C 90 XC 50 L 40 XL 10 X 9 IX 5 V 4 IV 1 I").with_range(1, 3999)),
        ("armenian", weighted("9000 \u{554} 8000 \u{553} 7000 \u{552} 6000 \u{551} 5000 \u{550} 4000 \u{54f} 3000 \u{54e} 2000 \u{54d} 1000 \u{54c} 900 \u{54b} 800 \u{54a} 700 \u{549} 600 \u{548} 500 \u{547} 400 \u{546} 300 \u{545} 200 \u{544} 100 \u{543} 90 \u{542} 80 \u{541} 70 \u{540} 60 \u{53f} 50 \u{53e} 40 \u{53d} 30 \u{53c} 20 \u{53b} 10 \u{53a} 9 \u{539} 8 \u{538} 7 \u{537} 6 \u{536} 5 \u{535} 4 \u{534} 3 \u{533} 2 \u{532} 1 \u{531}").with_range(1, 9999)),
        ("upper-armenian", weighted("9000 \u{554} 8000 \u{553} 7000 \u{552} 6000 \u{551} 5000 \u{550} 4000 \u{54f} 3000 \u{54e} 2000 \u{54d} 1000 \u{54c} 900 \u{54b} 800 \u{54a} 700 \u{549} 600 \u{548} 500 \u{547} 400 \u{546} 300 \u{545} 200 \u{544} 100 \u{543} 90 \u{542} 80 \u{541} 70 \u{540} 60 \u{53f} 50 \u{53e} 40 \u{53d} 30 \u{53c} 20 \u{53b} 10 \u{53a} 9 \u{539} 8 \u{538} 7 \u{537} 6 \u{536} 5 \u{535} 4 \u{534} 3 \u{533} 2 \u{532} 1 \u{531}").with_range(1, 9999)),
        ("lower-armenian", weighted("9000 \u{584} 8000 \u{583} 7000 \u{582} 6000 \u{581} 5000 \u{580} 4000 \u{57f} 3000 \u{57e} 2000 \u{57d} 1000 \u{57c} 900 \u{57b} 800 \u{57a} 700 \u{579} 600 \u{578} 500 \u{577} 400 \u{576} 300 \u{575} 200 \u{574} 100 \u{573} 90 \u{572} 80 \u{571} 70 \u{570} 60 \u{56f} 50 \u{56e} 40 \u{56d} 30 \u{56c} 20 \u{56b} 10 \u{56a} 9 \u{569} 8 \u{568} 7 \u{567} 6 \u{566} 5 \u{565} 4 \u{564} 3 \u{563} 2 \u{562} 1 \u{561}").with_range(1, 9999)),
        ("georgian", weighted("10000 \u{10f5} 9000 \u{10f0} 8000 \u{10ef} 7000 \u{10f4} 6000 \u{10ee} 5000 \u{10ed} 4000 \u{10ec} 3000 \u{10eb} 2000 \u{10ea} 1000 \u{10e9} 900 \u{10e8} 800 \u{10e7} 700 \u{10e6} 600 \u{10e5} 500 \u{10e4} 400 \u{10f3} 300 \u{10e2} 200 \u{10e1} 100 \u{10e0} 90 \u{10df} 80 \u{10de} 70 \u{10dd} 60 \u{10f2} 50 \u{10dc} 40 \u{10db} 30 \u{10da} 20 \u{10d9} 10 \u{10d8} 9 \u{10d7} 8 \u{10f1} 7 \u{10d6} 6 \u{10d5} 5 \u{10d4} 4 \u{10d3} 3 \u{10d2} 2 \u{10d1} 1 \u{10d0}").with_range(1, 19999)),
        ("hebrew", weighted("10000 \u{5d9}\u{5f3} 9000 \u{5d8}\u{5f3} 8000 \u{5d7}\u{5f3} 7000 \u{5d6}\u{5f3} 6000 \u{5d5}\u{5f3} 5000 \u{5d4}\u{5f3} 4000 \u{5d3}\u{5f3} 3000 \u{5d2}\u{5f3} 2000 \u{5d1}\u{5f3} 1000 \u{5d0}\u{5f3} 400 \u{5ea} 300 \u{5e9} 200 \u{5e8} 100 \u{5e7} 90 \u{5e6} 80 \u{5e4} 70 \u{5e2} 60 \u{5e1} 50 \u{5e0} 40 \u{5de} 30 \u{5dc} 20 \u{5db} 19 \u{5d9}\u{5d8} 18 \u{5d9}\u{5d7} 17 \u{5d9}\u{5d6} 16 \u{5d8}\u{5d6} 15 \u{5d8}\u{5d5} 10 \u{5d9} 9 \u{5d8} 8 \u{5d7} 7 \u{5d6} 6 \u{5d5} 5 \u{5d4} 4 \u{5d3} 3 \u{5d2} 2 \u{5d1} 1 \u{5d0}").with_range(1, 10999)),
    ])
}
