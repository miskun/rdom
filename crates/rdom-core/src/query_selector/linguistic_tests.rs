//! C11-LINK-LANG — the location and linguistic pseudo-classes: `:link`,
//! `:any-link` and `:visited` (Selectors 4 §8.1–§8.2), `:lang()` (§7.2)
//! and `:dir()` (§7.1), with HTML's definitions of hyperlinks (§4.16.3),
//! of the language of a node (§3.2.6.2) and of directionality
//! (§3.2.6.4).

use crate::{Directionality, Dom, NodeId};

fn el(dom: &mut Dom, parent: NodeId, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
    let e = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(e, k, v).unwrap();
    }
    dom.append_child(parent, e).unwrap();
    e
}

fn text(dom: &mut Dom, parent: NodeId, data: &str) {
    let t = dom.create_text_node(data);
    dom.append_child(parent, t).unwrap();
}

fn is(dom: &Dom, id: NodeId, sel: &str) -> bool {
    dom.matches(id, sel)
        .unwrap_or_else(|e| panic!("{sel}: {e}"))
}

// ─── :link / :any-link / :visited ───────────────────────────────────

/// Selectors 4 §8.1–§8.2, HTML §4.16.3: `a` and `area` elements with an
/// `href` are the source anchors of hyperlinks — `:any-link`, and, never
/// visited (rdom keeps no history), `:link`. `:visited` parses and
/// matches nothing.
#[test]
fn links_are_a_and_area_with_href_and_never_visited() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "a", &[("href", "#x")]);
    let empty = el(&mut dom, root, "a", &[("href", "")]);
    let area = el(&mut dom, root, "area", &[("href", "/")]);
    let bare = el(&mut dom, root, "a", &[]);
    let link = el(&mut dom, root, "link", &[("href", "s.css")]);
    for id in [a, empty, area] {
        assert!(is(&dom, id, ":any-link"));
        assert!(is(&dom, id, ":link"));
        assert!(!is(&dom, id, ":visited"));
    }
    for id in [bare, link] {
        assert!(!is(&dom, id, ":any-link"));
        assert!(!is(&dom, id, ":link"));
    }
    assert!(is(&dom, a, "a:link, a:visited"));
    assert!(!is(&dom, a, ":link:visited"));
    assert_eq!(
        crate::selectors::parse(":any-link").unwrap().0[0].specificity(),
        (0, 1, 0)
    );
}

// ─── :lang() ────────────────────────────────────────────────────────

/// Selectors 4 §7.2: an element matches when its content language
/// matches one of the ranges by RFC 4647 §3.3.2 extended filtering,
/// ASCII case-insensitively — a range's subtags in order, any tag
/// subtags between them skipped except singletons, `*` matching any
/// first subtag.
#[test]
fn lang_matches_by_extended_filtering() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let langs = [
        "de",
        "de-DE",
        "de-Latn-DE",
        "de-DE-x-goethe",
        "de-x-DE",
        "fr-CH",
        "en-US",
        "EN",
    ];
    let ids: Vec<NodeId> = langs
        .iter()
        .map(|l| el(&mut dom, root, "p", &[("lang", l)]))
        .collect();
    let matching = |sel: &str| -> Vec<&str> {
        ids.iter()
            .zip(langs)
            .filter(|(id, _)| is(&dom, **id, sel))
            .map(|(_, l)| l)
            .collect()
    };
    assert_eq!(
        matching(":lang(de)"),
        ["de", "de-DE", "de-Latn-DE", "de-DE-x-goethe", "de-x-DE"]
    );
    assert_eq!(
        matching(":lang(de-DE)"),
        ["de-DE", "de-Latn-DE", "de-DE-x-goethe"]
    );
    assert_eq!(matching(":lang(\"*-CH\")"), ["fr-CH"]);
    assert_eq!(
        matching(r":lang(\*-DE)"),
        ["de-DE", "de-Latn-DE", "de-DE-x-goethe"]
    );
    assert_eq!(matching(":lang(en)"), ["en-US", "EN"]);
    assert_eq!(matching(":LANG(En-us)"), ["en-US"]);
    assert_eq!(matching(":lang(fr, 'en-US')"), ["fr-CH", "en-US"]);
    assert_eq!(matching(":lang(d)"), Vec::<&str>::new());
}

/// HTML §3.2.6.2: the language is the nearest inclusive ancestor's
/// `xml:lang` or `lang` (`xml:lang` first); an empty one means unknown,
/// as does none at all — `*` matches no unknown language, the empty
/// range only an unknown one.
#[test]
fn lang_is_inherited_and_an_empty_one_is_unknown() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let outer = el(&mut dom, root, "div", &[("lang", "fr")]);
    let inner = el(&mut dom, outer, "span", &[]);
    let xml = el(
        &mut dom,
        outer,
        "span",
        &[("lang", "en"), ("xml:lang", "de")],
    );
    let unknown = el(&mut dom, outer, "span", &[("lang", "")]);
    let untagged = el(&mut dom, root, "div", &[]);
    assert!(is(&dom, inner, ":lang(fr)"));
    assert!(is(&dom, xml, ":lang(de)"));
    assert!(!is(&dom, xml, ":lang(en)"));
    assert_eq!(dom.language(inner), Some("fr"));
    assert_eq!(dom.language(unknown), Some(""));
    assert_eq!(dom.language(untagged), None);
    for id in [unknown, untagged] {
        assert!(!is(&dom, id, ":lang(fr)"));
        assert!(!is(&dom, id, "[lang]:lang('*'), :lang('*')"));
        assert!(is(&dom, id, ":lang('')"));
    }
    assert!(is(&dom, inner, ":lang('*')"));
}

/// Selectors 4 §7.2: each range is an `<ident>` or a `<string>` — a bare
/// `*` must be escaped or quoted — and the list is not empty.
#[test]
fn lang_takes_identifiers_and_strings_only() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", &[("lang", "en")]);
    for bad in [
        ":lang()",
        ":lang(*)",
        ":lang(1)",
        ":lang(en,)",
        ":lang(en fr)",
    ] {
        assert!(dom.matches(p, bad).is_err(), "{bad}");
    }
}

// ─── :dir() ─────────────────────────────────────────────────────────

/// Selectors 4 §7.1 / HTML §3.2.6.4: `:dir()` matches the element's
/// directionality — its `dir` attribute's state (ASCII
/// case-insensitive), else its parent's, else `ltr`. `<input type=tel>`
/// without a `dir` is `ltr`. An argument other than `ltr` / `rtl`
/// parses and matches nothing.
#[test]
fn dir_follows_the_dir_attribute_and_inherits() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let rtl = el(&mut dom, root, "div", &[("dir", "RTL")]);
    let child = el(&mut dom, rtl, "p", &[]);
    let bogus = el(&mut dom, rtl, "p", &[("dir", "sideways")]);
    let ltr = el(&mut dom, rtl, "p", &[("dir", "ltr")]);
    let tel = el(&mut dom, rtl, "input", &[("type", "tel")]);
    let plain = el(&mut dom, root, "p", &[]);
    for (id, want) in [
        (rtl, Directionality::Rtl),
        (child, Directionality::Rtl),
        (bogus, Directionality::Rtl),
        (ltr, Directionality::Ltr),
        (tel, Directionality::Ltr),
        (plain, Directionality::Ltr),
    ] {
        assert_eq!(dom.directionality(id), want, "{id:?}");
        let rtl_now = want == Directionality::Rtl;
        assert_eq!(is(&dom, id, ":dir(rtl)"), rtl_now);
        assert_eq!(is(&dom, id, ":DIR(LTR)"), !rtl_now);
        assert!(!is(&dom, id, ":dir(auto)"));
    }
    assert!(dom.matches(plain, ":dir()").is_err());
    assert!(dom.matches(plain, ":dir(ltr rtl)").is_err());
}

/// HTML §3.2.6.4: `dir=auto` and a `<bdi>` without `dir` take the
/// direction of the first strong character of their text, skipping
/// descendants with their own `dir`, `<bdi>`, `<script>`, `<style>` and
/// `<textarea>`; `ltr` without one. A `<textarea dir=auto>` reads its
/// value.
#[test]
fn dir_auto_reads_the_first_strong_character() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let hebrew = el(&mut dom, root, "p", &[("dir", "auto")]);
    text(&mut dom, hebrew, "123 שלום abc");
    let latin = el(&mut dom, root, "p", &[("dir", "auto")]);
    text(&mut dom, latin, "123 abc שלום");
    let neutral = el(&mut dom, root, "p", &[("dir", "auto")]);
    text(&mut dom, neutral, "123 !?");
    let skipping = el(&mut dom, root, "p", &[("dir", "auto")]);
    let own = el(&mut dom, skipping, "span", &[("dir", "ltr")]);
    text(&mut dom, own, "abc");
    let script = el(&mut dom, skipping, "script", &[]);
    text(&mut dom, script, "var x");
    text(&mut dom, skipping, "مرحبا");
    let bdi = el(&mut dom, root, "bdi", &[]);
    text(&mut dom, bdi, "שלום");
    let area = el(&mut dom, root, "textarea", &[("dir", "auto")]);
    text(&mut dom, area, "שלום");
    let nested = el(&mut dom, hebrew, "em", &[]);
    for (id, want) in [
        (hebrew, Directionality::Rtl),
        (nested, Directionality::Rtl),
        (latin, Directionality::Ltr),
        (neutral, Directionality::Ltr),
        (skipping, Directionality::Rtl),
        (bdi, Directionality::Rtl),
        (area, Directionality::Rtl),
    ] {
        assert_eq!(dom.directionality(id), want, "{id:?}");
    }
}
