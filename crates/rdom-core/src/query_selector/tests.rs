//! `query_selector` tests: the query APIs and the matcher.

use crate::{Dom, NodeId};

// Build:
//   root
//     div#a.outer
//       span.first
//       span.mid lang="en"
//       p.last
//         em "leaf"
fn build() -> (Dom, [NodeId; 5]) {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.set_attribute(div, "id", "a").unwrap();
    dom.add_class(div, "outer").unwrap();

    let s1 = dom.create_element("span");
    dom.add_class(s1, "first").unwrap();

    let s2 = dom.create_element("span");
    dom.add_class(s2, "mid").unwrap();
    dom.set_attribute(s2, "lang", "en-US").unwrap();

    let p = dom.create_element("p");
    dom.add_class(p, "last").unwrap();

    let em = dom.create_element("em");
    let t = dom.create_text_node("leaf");
    dom.append_child(em, t).unwrap();
    dom.append_child(p, em).unwrap();

    dom.append_child(div, s1).unwrap();
    dom.append_child(div, s2).unwrap();
    dom.append_child(div, p).unwrap();
    dom.append_child(root, div).unwrap();

    (dom, [div, s1, s2, p, em])
}

#[test]
fn matches_type() {
    let (dom, [div, ..]) = build();
    assert!(dom.matches(div, "div").unwrap());
    assert!(!dom.matches(div, "span").unwrap());
}

#[test]
fn matches_id() {
    let (dom, [div, s1, ..]) = build();
    assert!(dom.matches(div, "#a").unwrap());
    assert!(!dom.matches(s1, "#a").unwrap());
}

#[test]
fn matches_class() {
    let (dom, [_, s1, ..]) = build();
    assert!(dom.matches(s1, ".first").unwrap());
    assert!(!dom.matches(s1, ".missing").unwrap());
}

#[test]
fn matches_attribute_variants() {
    let (dom, [_, _, s2, ..]) = build();
    assert!(dom.matches(s2, "[lang]").unwrap());
    assert!(dom.matches(s2, "[lang=en-US]").unwrap());
    assert!(dom.matches(s2, "[lang|=en]").unwrap());
    assert!(dom.matches(s2, "[lang^=en]").unwrap());
    assert!(dom.matches(s2, "[lang$=US]").unwrap());
    assert!(dom.matches(s2, "[lang*=n-U]").unwrap());
    assert!(!dom.matches(s2, "[lang=fr]").unwrap());
}

#[test]
fn matches_compound() {
    let (dom, [div, ..]) = build();
    assert!(dom.matches(div, "div#a.outer").unwrap());
    assert!(!dom.matches(div, "div#b.outer").unwrap());
}

#[test]
fn query_selector_descendant() {
    let (dom, [_, s1, ..]) = build();
    let root = dom.root();
    assert_eq!(dom.query_selector_in(root, "div .first").unwrap(), Some(s1));
}

#[test]
fn query_selector_child_combinator() {
    let (dom, [_, _, _, _, em]) = build();
    let root = dom.root();
    // em is inside p which is inside div — only matches as descendant, not child of div.
    assert!(dom.query_selector_in(root, "div > em").unwrap().is_none());
    assert_eq!(dom.query_selector_in(root, "p > em").unwrap(), Some(em));
}

#[test]
fn query_selector_adjacent_sibling() {
    let (dom, [_, _, s2, ..]) = build();
    let root = dom.root();
    assert_eq!(
        dom.query_selector_in(root, ".first + .mid").unwrap(),
        Some(s2)
    );
    assert!(
        dom.query_selector_in(root, ".first + .last")
            .unwrap()
            .is_none()
    );
}

#[test]
fn query_selector_general_sibling() {
    let (dom, [_, _, _, p, _]) = build();
    let root = dom.root();
    assert_eq!(
        dom.query_selector_in(root, ".first ~ .last").unwrap(),
        Some(p)
    );
}

#[test]
fn query_selector_all_returns_document_order() {
    let (dom, [_, s1, s2, ..]) = build();
    let root = dom.root();
    let spans = dom.query_selector_all_in(root, "span").unwrap();
    assert_eq!(spans, vec![s1, s2]);
}

#[test]
fn query_selector_list_union() {
    let (dom, [_, _, _, p, em]) = build();
    let root = dom.root();
    let r = dom.query_selector_all_in(root, "p, em").unwrap();
    assert_eq!(r, vec![p, em]);
}

#[test]
fn not_pseudo_excludes_matches() {
    let (dom, _) = build();
    let root = dom.root();
    let r = dom.query_selector_all_in(root, "span:not(.first)").unwrap();
    assert_eq!(r.len(), 1);
}

#[test]
fn where_pseudo_matches_like_is() {
    let (dom, [_div, s1, _s2, p, em]) = build();
    let root = dom.root();
    // :where(list) matches an element matching any item in the list.
    let r = dom
        .query_selector_all_in(root, ":where(.first, .last)")
        .unwrap();
    assert_eq!(r, vec![s1, p]);
    // Combinators inside :where() are honored (em is a descendant of div).
    assert!(dom.matches(em, ":where(div em)").unwrap());
    assert!(!dom.matches(s1, ":where(div em)").unwrap());
}

/// Selectors 4 §4.2 (`C1G-IS-PARSE`): `:is()` matches an element
/// matching any argument — combinators included, an invalid argument
/// dropped, an empty list matching nothing — inside `:not()` too.
#[test]
fn is_pseudo_matches_any_argument() {
    let (dom, [div, s1, s2, p, em]) = build();
    let root = dom.root();
    let r = dom
        .query_selector_all_in(root, ":is(.first, .last)")
        .unwrap();
    assert_eq!(r, vec![s1, p]);
    assert!(dom.matches(em, ":is(div em)").unwrap());
    assert!(!dom.matches(s1, ":is(div em)").unwrap());
    assert!(dom.matches(s1, ":is(!!bad, .first)").unwrap());
    let r = dom
        .query_selector_all_in(root, "span:not(:is(.first))")
        .unwrap();
    assert_eq!(r, vec![s2]);
    for id in [div, s1, s2, p, em] {
        assert!(
            !dom.matches(id, ":is(!!)").unwrap(),
            "empty :is() matches nothing"
        );
    }
}

#[test]
fn first_and_last_child_pseudos() {
    let (dom, [div, s1, _, p, em]) = build();
    let root = dom.root();
    // Every element that IS the first child of its parent: div (first of
    // root), s1 (first of div), em (first of p). Document-order pick: div.
    assert_eq!(
        dom.query_selector_in(root, ":first-child").unwrap(),
        Some(div)
    );
    // `span:first-child` scopes to spans — only s1 qualifies.
    assert_eq!(
        dom.query_selector_in(root, "span:first-child").unwrap(),
        Some(s1)
    );
    // Last children of each parent: div, p, em.
    let lasts = dom.query_selector_all_in(root, ":last-child").unwrap();
    assert!(lasts.contains(&p));
    assert!(lasts.contains(&em));
}

#[test]
fn only_child_pseudo() {
    let (dom, [_, _, _, _, em]) = build();
    let root = dom.root();
    // em is the only child of p.
    assert_eq!(
        dom.query_selector_in(root, "em:only-child").unwrap(),
        Some(em)
    );
}

#[test]
fn empty_pseudo() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let empty = dom.create_element("div");
    let not_empty = dom.create_element("div");
    let t = dom.create_text_node("x");
    dom.append_child(not_empty, t).unwrap();
    dom.append_child(root, empty).unwrap();
    dom.append_child(root, not_empty).unwrap();
    let r = dom.query_selector_all_in(root, "div:empty").unwrap();
    assert_eq!(r, vec![empty]);
}

/// Selectors 4 §14.2 (`P7G-CORE-SMALL-1`): `:empty` means no element
/// children and no text children with non-empty data — a zero-length
/// text node, a comment do not count; whitespace-only text does
/// (Level 3, and browsers today).
#[test]
fn empty_ignores_zero_length_text_and_comments_but_not_whitespace() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let with = |dom: &mut Dom, kids: &[Option<&str>]| {
        let div = dom.create_element("div");
        for k in kids {
            let child = match k {
                Some(text) => dom.create_text_node(text),
                None => dom.create_comment("c"),
            };
            dom.append_child(div, child).unwrap();
        }
        dom.append_child(root, div).unwrap();
        div
    };
    let zero_length = with(&mut dom, &[Some("")]);
    let comment = with(&mut dom, &[None]);
    let both = with(&mut dom, &[Some(""), None, Some("")]);
    let space = with(&mut dom, &[Some(" ")]);
    let text = with(&mut dom, &[Some(""), Some("x")]);
    for id in [zero_length, comment, both] {
        assert!(dom.matches(id, ":empty").unwrap(), "{id:?}");
    }
    for id in [space, text] {
        assert!(!dom.matches(id, ":empty").unwrap(), "{id:?}");
    }
}

/// CSS Syntax 3 §4.2: non-ASCII code points are ident code points,
/// so unquoted values, classes and ids may use them.
#[test]
fn non_ascii_identifiers_match() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let el = dom.create_element("p");
    dom.set_attribute(el, "lang", "én-CA").unwrap();
    dom.set_attribute(el, "id", "naïve").unwrap();
    dom.add_class(el, "café").unwrap();
    dom.append_child(root, el).unwrap();
    for sel in ["[lang|=én]", "p.café", "#naïve", "[lang^=é]"] {
        assert!(dom.matches(el, sel).unwrap(), "{sel}");
    }
}

/// CSS Syntax 3 §4.3.7 "consume an escaped code point": `\` + 1–6
/// hex digits is that code point (one whitespace after the digits
/// belongs to the escape), `\` + any other code point is that code
/// point. Escapes work in type, class, id and attribute names and
/// in attribute values, quoted or not.
#[test]
fn escaped_identifiers_match() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let el = dom.create_element("p");
    dom.add_class(el, "10").unwrap();
    dom.set_attribute(el, "id", "a:b").unwrap();
    dom.set_attribute(el, "data-x", "q\"r").unwrap();
    dom.set_attribute(el, "da:ta", "1 2").unwrap();
    dom.append_child(root, el).unwrap();
    for sel in [
        r".\31 0",
        r"#a\:b",
        r"#a\3a b",
        r"#a\00003Ab",
        r"\70",
        r".\31\30",
        r#"[data-x="q\"r"]"#,
        r#"[data-x='q\22r']"#,
        r"[da\:ta=\31\ 2]",
    ] {
        assert!(dom.matches(el, sel).unwrap(), "{sel}");
    }
    assert_eq!(dom.query_selector(r".\31 0").map(|n| n.id()), Some(el));
    // `#a:b` is id `a` plus an (unknown) pseudo-class, not id `a:b`.
    assert!(dom.matches(el, "#a:b").is_err());
}

/// §4.3.7: a `\` followed by a newline is not a valid escape, so it
/// cannot continue an identifier — the selector is invalid.
#[test]
fn escaped_newline_is_not_an_identifier_escape() {
    assert!(crate::selectors::parse(".a\\\nb").is_err());
}

#[test]
fn root_pseudo() {
    let (dom, _) = build();
    let root = dom.root();
    // `:root` matches only the document root. query_selector scans
    // descendants, so it won't find root itself — use matches / closest.
    // But our root is a Fragment, not an Element. Build a new dom with
    // an element root to exercise :root.
    let mut dom2: Dom = Dom::with_root_tag("html");
    let root2 = dom2.root();
    let body = dom2.create_element("body");
    dom2.append_child(root2, body).unwrap();
    assert!(dom2.matches(root2, ":root").unwrap());
    assert!(!dom2.matches(body, ":root").unwrap());
    // In the original fragment-rooted tree, the root is a fragment so
    // matches_compound returns false regardless.
    assert!(!dom.matches(root, ":root").unwrap());
}

#[test]
fn matches_with_chain() {
    let (dom, [_, _, _, _, em]) = build();
    // em inside .outer via descendant combinator.
    assert!(dom.matches(em, ".outer em").unwrap());
    // child: em's parent is p, not .outer directly.
    assert!(!dom.matches(em, ".outer > em").unwrap());
}

#[test]
fn hover_pseudo_follows_set_hovered() {
    let (mut dom, [div, _, _, _, em]) = build();
    // Nothing hovered → no matches.
    assert!(!dom.matches(div, ":hover").unwrap());
    // Hover div — only div matches.
    dom.set_hovered(Some(div));
    assert!(dom.matches(div, ":hover").unwrap());
    assert!(!dom.matches(em, ":hover").unwrap());
    // Clear hover.
    dom.set_hovered(None);
    assert!(!dom.matches(div, ":hover").unwrap());
}

/// Selectors 4 §9.2: an element also matches `:hover` while one of
/// its descendants is the hovered element (`P7G-HOVER-ANCESTORS-1`).
#[test]
fn hover_matches_the_ancestors_of_the_hovered_element() {
    let (mut dom, [div, s1, s2, p, em]) = build();
    dom.set_hovered(Some(em));
    assert!(dom.matches(em, ":hover").unwrap(), "the hovered element");
    assert!(dom.matches(p, ":hover").unwrap(), "its parent");
    assert!(dom.matches(div, ":hover").unwrap(), "its grandparent");
    assert!(dom.matches(em, ".outer:hover em").unwrap());
    assert!(!dom.matches(s1, ":hover").unwrap(), "a sibling subtree");
    assert!(!dom.matches(s2, ":hover").unwrap(), "a sibling subtree");
    dom.set_hovered(None);
    assert!(!dom.matches(div, ":hover").unwrap());
}

/// Selectors 4 §9.4: `:active` matches the activated element and,
/// like `:hover`, its ancestors.
#[test]
fn active_matches_the_activated_element_and_its_ancestors() {
    let (mut dom, [div, s1, _, p, em]) = build();
    assert!(!dom.matches(em, ":active").unwrap());
    dom.set_active(Some(em));
    assert_eq!(dom.active(), Some(em));
    assert!(dom.matches(em, ":active").unwrap());
    assert!(dom.matches(p, ":active").unwrap());
    assert!(dom.matches(div, "div:active").unwrap());
    assert!(!dom.matches(s1, ":active").unwrap());
    dom.set_active(None);
    assert!(!dom.matches(div, ":active").unwrap());
    assert!(!dom.matches(em, ":active").unwrap());
}

#[test]
fn focus_pseudo_follows_set_focused() {
    let (mut dom, [div, s1, _, _, _]) = build();
    dom.set_focused(Some(s1));
    assert!(dom.matches(s1, ":focus").unwrap());
    assert!(!dom.matches(div, ":focus").unwrap());
}

#[test]
fn focus_within_matches_focused_and_ancestor_elements() {
    // `:focus-within` matches the focused element AND every
    // ancestor element in the chain. Mirrors CSS Selectors L4
    // §10.1.4. The Document root is not an element so it
    // won't return true via `matches()` (which filters to
    // elements), but every element ancestor between root and
    // the focused node does.
    //
    // Tree shape (from `build`):
    //   root → div.outer → p.last → em
    let (mut dom, [div, _s1, _s2, p, em]) = build();
    dom.set_focused(Some(em));
    assert!(dom.matches(em, ":focus-within").unwrap(), "focused itself");
    assert!(dom.matches(p, ":focus-within").unwrap(), "parent");
    assert!(dom.matches(div, ":focus-within").unwrap(), "grandparent");
}

#[test]
fn focus_within_does_not_match_siblings_or_other_subtrees() {
    // Focus is on em (under p.last). Siblings of p (s1.first,
    // s2.mid) are in a different subtree — they must NOT match.
    let (mut dom, [_div, s1, s2, _p, em]) = build();
    dom.set_focused(Some(em));
    assert!(!dom.matches(s1, ":focus-within").unwrap());
    assert!(!dom.matches(s2, ":focus-within").unwrap());
}

#[test]
fn focus_within_clears_when_focus_cleared() {
    let (mut dom, [div, _, _, _, em]) = build();
    dom.set_focused(Some(em));
    assert!(dom.matches(div, ":focus-within").unwrap());
    dom.set_focused(None);
    assert!(!dom.matches(div, ":focus-within").unwrap());
    assert!(!dom.matches(em, ":focus-within").unwrap());
}

#[test]
fn hover_focus_combine_with_other_selectors() {
    let (mut dom, [_, s1, _, _, _]) = build();
    dom.set_hovered(Some(s1));
    // span:hover
    assert!(dom.matches(s1, "span:hover").unwrap());
    // span.first:hover
    assert!(dom.matches(s1, "span.first:hover").unwrap());
    // Non-hovered elements don't match.
    dom.set_hovered(None);
    assert!(!dom.matches(s1, "span:hover").unwrap());
}

#[test]
fn checked_pseudo_matches_attribute_presence() {
    let mut dom: Dom<()> = Dom::new();
    let root = dom.root();
    let cb = dom.create_element("input");
    dom.set_attribute(cb, "type", "checkbox").unwrap();
    dom.append_child(root, cb).unwrap();

    // Absent → no match.
    assert!(!dom.matches(cb, ":checked").unwrap());

    // Empty value (HTML boolean shorthand) → match.
    dom.set_attribute(cb, "checked", "").unwrap();
    assert!(dom.matches(cb, ":checked").unwrap());

    // Any value still matches (presence-only, like the HTML
    // boolean attribute model).
    dom.set_attribute(cb, "checked", "false").unwrap();
    assert!(dom.matches(cb, ":checked").unwrap());

    // Removed → no match.
    dom.remove_attribute(cb, "checked").unwrap();
    assert!(!dom.matches(cb, ":checked").unwrap());
}

#[test]
fn checked_pseudo_combines_with_type_attribute_selector() {
    let mut dom: Dom<()> = Dom::new();
    let root = dom.root();
    let cb = dom.create_element("input");
    dom.set_attribute(cb, "type", "checkbox").unwrap();
    dom.set_attribute(cb, "checked", "").unwrap();
    dom.append_child(root, cb).unwrap();

    assert!(dom.matches(cb, "[type=checkbox]:checked").unwrap());
    assert!(!dom.matches(cb, "[type=radio]:checked").unwrap());
}

#[test]
fn placeholder_shown_matches_when_attribute_set_and_content_empty() {
    let mut dom: Dom<()> = Dom::new();
    let root = dom.root();
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "placeholder", "Search...").unwrap();
    dom.append_child(root, inp).unwrap();

    // No text content → match.
    assert!(dom.matches(inp, ":placeholder-shown").unwrap());

    // Add some content → no match.
    let t = dom.create_text_node("hi");
    dom.append_child(inp, t).unwrap();
    assert!(!dom.matches(inp, ":placeholder-shown").unwrap());
}

#[test]
fn placeholder_shown_requires_non_empty_placeholder_attribute() {
    let mut dom: Dom<()> = Dom::new();
    let root = dom.root();
    let inp = dom.create_element("input");
    dom.append_child(root, inp).unwrap();
    // No placeholder → no match.
    assert!(!dom.matches(inp, ":placeholder-shown").unwrap());

    // Empty placeholder → still no match (HTML rule — blank
    // placeholder doesn't count).
    dom.set_attribute(inp, "placeholder", "").unwrap();
    assert!(!dom.matches(inp, ":placeholder-shown").unwrap());
}

#[test]
fn placeholder_shown_with_whitespace_is_empty_content() {
    // Content::text() concatenates raw strings — whitespace is
    // NOT collapsed for this test. Text node with just spaces
    // is non-empty; matches browsers for real whitespace.
    let mut dom: Dom<()> = Dom::new();
    let root = dom.root();
    let inp = dom.create_element("input");
    dom.set_attribute(inp, "placeholder", "Hint").unwrap();
    let t = dom.create_text_node(" ");
    dom.append_child(inp, t).unwrap();
    dom.append_child(root, inp).unwrap();
    // Space is non-empty text → doesn't match.
    assert!(!dom.matches(inp, ":placeholder-shown").unwrap());
}

// ── :indeterminate + :open (Polish #4) ────────────────────────

#[test]
fn indeterminate_matches_progress_without_value() {
    let mut dom: Dom<()> = Dom::new();
    let root = dom.root();
    let p = dom.create_element("progress");
    dom.append_child(root, p).unwrap();
    assert!(dom.matches(p, ":indeterminate").unwrap());
    dom.set_attribute(p, "value", "0.5").unwrap();
    assert!(!dom.matches(p, ":indeterminate").unwrap());
}

#[test]
fn indeterminate_does_not_match_other_tags() {
    let mut dom: Dom<()> = Dom::new();
    let root = dom.root();
    let m = dom.create_element("meter");
    dom.append_child(root, m).unwrap();
    // `<meter>` without value is NOT indeterminate (unlike
    // progress) — meter always represents a known measurement.
    assert!(!dom.matches(m, ":indeterminate").unwrap());
}

#[test]
fn open_matches_elements_with_open_attribute() {
    let mut dom: Dom<()> = Dom::new();
    let root = dom.root();
    let d = dom.create_element("details");
    dom.append_child(root, d).unwrap();
    assert!(!dom.matches(d, ":open").unwrap());
    dom.set_attribute(d, "open", "").unwrap();
    assert!(dom.matches(d, ":open").unwrap());
}

#[test]
fn open_works_on_dialog_as_well() {
    let mut dom: Dom<()> = Dom::new();
    let root = dom.root();
    let d = dom.create_element("dialog");
    dom.set_attribute(d, "open", "").unwrap();
    dom.append_child(root, d).unwrap();
    assert!(dom.matches(d, ":open").unwrap());
}

#[test]
fn open_can_combine_with_other_selectors() {
    let mut dom: Dom<()> = Dom::new();
    let root = dom.root();
    let d = dom.create_element("details");
    dom.set_attribute(d, "open", "").unwrap();
    dom.append_child(root, d).unwrap();
    assert!(dom.matches(d, "details:open").unwrap());
    assert!(!dom.matches(d, "dialog:open").unwrap());
}

/// HTML §4.16.2: attribute selectors treat the values of `type`,
/// `method`, `enctype`, `lang`, … on HTML elements as ASCII
/// case-insensitive; every other attribute stays case-sensitive.
#[test]
fn html_case_insensitive_attribute_values_match_regardless_of_case() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let cb = dom.create_element("input");
    dom.set_attribute(cb, "type", "CheckBox").unwrap();
    dom.set_attribute(cb, "data-kind", "Big").unwrap();
    dom.append_child(root, cb).unwrap();
    let form = dom.create_element("form");
    dom.set_attribute(form, "method", "POST").unwrap();
    dom.set_attribute(form, "lang", "EN-us").unwrap();
    dom.append_child(root, form).unwrap();

    assert!(dom.matches(cb, "input[type=checkbox]").unwrap());
    assert!(dom.matches(cb, "[type^=check]").unwrap());
    assert!(dom.matches(cb, ":not([type=radio])").unwrap());
    assert!(dom.matches(form, "[method=post]").unwrap());
    assert!(dom.matches(form, "[lang|=en]").unwrap());
    assert!(
        !dom.matches(cb, "[data-kind=big]").unwrap(),
        "attributes outside the HTML list stay case-sensitive"
    );
    assert!(dom.matches(cb, "[data-kind=Big]").unwrap());
}

/// Every attribute operator honors HTML §4.16.2's case-insensitive
/// values — `=`, `~=`, `|=`, `^=`, `$=`, `*=` — on a listed attribute
/// (`rel`, `lang`, `type`), and stays case-sensitive on others.
#[test]
fn every_attribute_operator_is_ascii_case_insensitive_on_listed_attributes() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    dom.set_attribute(a, "rel", "NoOpener External").unwrap();
    dom.set_attribute(a, "lang", "EN-GB").unwrap();
    dom.set_attribute(a, "type", "Text/HTML").unwrap();
    dom.set_attribute(a, "title", "Text/HTML").unwrap();
    dom.append_child(root, a).unwrap();
    for sel in [
        "[type='text/html']",
        "[rel~=external]",
        "[rel~=NOOPENER]",
        "[lang|=en]",
        "[lang|=en-gb]",
        "[type^=TEXT]",
        "[type$='/html']",
        "[type*='T/h']",
    ] {
        assert!(dom.matches(a, sel).unwrap(), "{sel}");
    }
    for sel in [
        "[rel~=noop]",
        "[lang|=e]",
        "[lang|=gb]",
        "[type^=html]",
        "[type$=text]",
        "[type*=xml]",
        "[type^='']",
    ] {
        assert!(!dom.matches(a, sel).unwrap(), "{sel}");
    }
    for sel in [
        "[title='text/html']",
        "[title^=text]",
        "[title$='/html']",
        "[title*='t/h']",
    ] {
        assert!(
            !dom.matches(a, sel).unwrap(),
            "{sel}: title is case-sensitive"
        );
    }
    assert!(dom.matches(a, "[title*='t/H']").unwrap());
    // Non-ASCII letters never fold.
    dom.set_attribute(a, "lang", "ÉN").unwrap();
    assert!(!dom.matches(a, "[lang|='én']").unwrap());
    assert!(dom.matches(a, "[lang|='ÉN']").unwrap());
}

#[test]
fn closest_walks_up() {
    let (dom, [div, _, _, _, em]) = build();
    // closest(".outer") from em returns div.
    assert_eq!(dom.closest(em, ".outer").unwrap(), Some(div));
    // closest("#nope") from em returns None.
    assert!(dom.closest(em, "#nope").unwrap().is_none());
    // closest("em") from em returns em (inclusive self).
    assert_eq!(dom.closest(em, "em").unwrap(), Some(em));
}

#[test]
fn invalid_selector_errors() {
    let (dom, _) = build();
    let root = dom.root();
    assert!(dom.query_selector_in(root, ":nope").is_err());
    assert!(dom.query_selector_all_in(root, "").is_err());
}

// ── P7-VALIDATION-SELECTORS-1 ─────────────────────────────────────

/// HTML §4.16.3: `:required` matches an `<input>` (in a state
/// `required` applies to), `<select>` or `<textarea>` with
/// `required`; `:optional` the rest of those three.
#[test]
fn required_and_optional_match_the_three_form_controls() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let mk = |dom: &mut Dom, tag: &str, attrs: &[(&str, &str)]| {
        let e = dom.create_element(tag);
        for (k, v) in attrs {
            dom.set_attribute(e, k, v).unwrap();
        }
        dom.append_child(root, e).unwrap();
        e
    };
    let req_text = mk(&mut dom, "input", &[("required", "")]);
    let req_box = mk(&mut dom, "input", &[("type", "checkbox"), ("required", "")]);
    let req_range = mk(&mut dom, "input", &[("type", "range"), ("required", "")]);
    let req_hidden = mk(&mut dom, "input", &[("type", "hidden"), ("required", "")]);
    let plain = mk(&mut dom, "input", &[]);
    let req_select = mk(&mut dom, "select", &[("required", "")]);
    let area = mk(&mut dom, "textarea", &[]);
    let req_div = mk(&mut dom, "div", &[("required", "")]);
    let button = mk(&mut dom, "button", &[("required", "")]);
    for id in [req_text, req_box, req_select] {
        assert!(dom.matches(id, ":required").unwrap(), "{id:?}");
        assert!(!dom.matches(id, ":optional").unwrap(), "{id:?}");
    }
    for id in [req_range, req_hidden, plain, area] {
        assert!(dom.matches(id, ":optional").unwrap(), "{id:?}");
        assert!(!dom.matches(id, ":required").unwrap(), "{id:?}");
    }
    for id in [req_div, button] {
        assert!(!dom.matches(id, ":required").unwrap(), "{id:?}");
        assert!(!dom.matches(id, ":optional").unwrap(), "{id:?}");
    }
}

/// Test backend: a candidate is invalid while it has `data-bad`.
/// Selectors 4 §13.2: `:focus-visible` matches the focused element
/// while the UA judges its focus should be evident
/// (`Dom::focus_visible`, on until a backend says otherwise); it
/// never matches an unfocused element.
#[test]
fn focus_visible_matches_the_focused_element_while_focus_is_evident() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let a = dom.create_element("button");
    let b = dom.create_element("button");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    assert!(dom.focus_visible(), "focus is evident by default");
    assert!(
        !dom.matches(a, ":focus-visible").unwrap(),
        "nothing focused"
    );

    dom.set_focused(Some(a));
    assert!(dom.matches(a, ":focus-visible").unwrap());
    assert!(!dom.matches(b, ":focus-visible").unwrap());

    dom.set_focus_visible(false);
    assert!(dom.matches(a, ":focus").unwrap());
    assert!(!dom.matches(a, ":focus-visible").unwrap());
    assert!(dom.matches(a, ":focus:not(:focus-visible)").unwrap());
}

fn bad_attr_hook(dom: &Dom, id: NodeId) -> bool {
    !dom.has_attribute(id, "data-bad")
}

/// `P7G-PUBLIC-SURFACE-2`: rdom-core has no validity states, and a
/// `Dom` without a validity hook answers "valid" for every candidate
/// — in a debug build as in a release build (it used to panic in
/// debug only). The documented contract (`Dom::set_validity_hook`)
/// is that a backend matching these pseudo-classes installs one.
#[test]
fn a_candidate_without_a_validity_hook_is_valid_in_every_build() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.append_child(root, input).unwrap();
    assert!(!dom.matches(input, ":invalid").unwrap());
    assert!(dom.matches(input, ":valid").unwrap());
}

/// Elements that are not candidates answer `:valid` / `:invalid`
/// without consulting the hook, so they need none.
#[test]
fn non_candidates_match_neither_validity_class_without_a_hook() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let hidden = dom.create_element("input");
    dom.set_attribute(hidden, "type", "hidden").unwrap();
    dom.append_child(root, hidden).unwrap();
    for id in [div, hidden] {
        assert!(!dom.matches(id, ":valid").unwrap());
        assert!(!dom.matches(id, ":invalid").unwrap());
    }
}

/// HTML §4.16.3: `:valid` / `:invalid` match candidates for
/// constraint validation by the backend's verdict (the validity
/// hook), forms by their owned candidates and fieldsets by their
/// descendant candidates; barred and other elements match neither.
#[test]
fn valid_and_invalid_follow_the_validity_hook() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    dom.append_child(root, form).unwrap();
    let fieldset = dom.create_element("fieldset");
    dom.append_child(form, fieldset).unwrap();
    let bad = dom.create_element("input");
    dom.set_attribute(bad, "data-bad", "").unwrap();
    dom.append_child(fieldset, bad).unwrap();
    let good = dom.create_element("textarea");
    dom.append_child(form, good).unwrap();
    let barred = dom.create_element("input");
    dom.set_attribute(barred, "data-bad", "").unwrap();
    dom.set_attribute(barred, "disabled", "").unwrap();
    dom.append_child(form, barred).unwrap();
    let div = dom.create_element("div");
    dom.append_child(form, div).unwrap();
    let empty_form = dom.create_element("form");
    dom.append_child(root, empty_form).unwrap();

    dom.set_validity_hook(Some(bad_attr_hook));
    assert!(dom.matches(bad, ":invalid").unwrap());
    assert!(!dom.matches(bad, ":valid").unwrap());
    assert!(dom.matches(good, ":valid").unwrap());
    assert!(dom.matches(fieldset, ":invalid").unwrap());
    assert!(dom.matches(form, ":invalid").unwrap());
    assert!(dom.matches(empty_form, ":valid").unwrap());
    for id in [barred, div] {
        assert!(!dom.matches(id, ":valid").unwrap(), "{id:?}");
        assert!(!dom.matches(id, ":invalid").unwrap(), "{id:?}");
    }
    assert_eq!(dom.constraint_validity(bad), Some(false));
    assert_eq!(dom.constraint_validity(div), None);

    dom.remove_attribute(bad, "data-bad").unwrap();
    assert!(dom.matches(form, ":valid").unwrap());
    assert!(dom.matches(fieldset, ":valid").unwrap());
}

/// Selectors 4 §14.3: `:scope` is the scoping root passed to
/// `matches_list_in_scope`, and `:root` without one.
#[test]
fn scope_matches_the_scoping_root() {
    let (dom, ids) = build();
    let list = crate::selectors::parse(":scope > *").unwrap();
    let child_of = |root| {
        ids.iter()
            .filter(|&&id| dom.matches_list_in_scope(id, &list, Some(root)))
            .count()
    };
    assert!(child_of(ids[0]) > 0);
    let scope = crate::selectors::parse(":scope").unwrap();
    assert!(dom.matches_list_in_scope(ids[1], &scope, Some(ids[1])));
    assert!(!dom.matches_list_in_scope(ids[1], &scope, Some(ids[0])));
    assert_eq!(
        dom.matches_list(ids[1], &scope),
        dom.matches(ids[1], ":root").unwrap()
    );
}

// ─── Attribute case flags (C11-ATTR-FLAGS) ──────────────────────────

/// Selectors 4 §6.3: `[a=b i]` compares the value ASCII
/// case-insensitively and `[a=b s]` case-sensitively, whatever HTML
/// §4.16.2's list says about the attribute; the flag itself is ASCII
/// case-insensitive, may follow a string or an identifier with or
/// without white space, and applies to every value operator.
#[test]
fn attribute_case_flags_override_the_default_case() {
    let mut dom: Dom = Dom::new();
    let ol = dom.create_element("ol");
    dom.set_attribute(ol, "type", "A").unwrap();
    dom.set_attribute(ol, "data-x", "FooBar").unwrap();
    dom.append_child(dom.root(), ol).unwrap();
    for (sel, want) in [
        // `data-x` is case-sensitive by default; `i` folds case.
        ("[data-x=foobar]", false),
        ("[data-x=foobar i]", true),
        ("[data-x='foobar' I]", true),
        ("[data-x=\"foobar\"i]", true),
        ("[data-x^=foo i]", true),
        ("[data-x$=BAR i]", true),
        ("[data-x*=ob i]", true),
        ("[data-x~=FOOBAR i]", true),
        ("[data-x|=foobar i]", true),
        ("[ data-x = foobar  i ]", true),
        // `type` is case-insensitive by default; `s` makes it exact.
        ("[type=a]", true),
        ("[type=a s]", false),
        ("[type=A s]", true),
        ("[type=A S]", true),
        ("[type^=a s]", false),
    ] {
        assert_eq!(dom.matches(ol, sel), Ok(want), "{sel}");
    }
}

/// Selectors 4 §6.3: a flag needs a value (`[a i]` is invalid), and
/// only `i` / `s` are flags; `[a=bi]` is the value `bi`.
#[test]
fn attribute_case_flags_reject_what_the_grammar_does_not_allow() {
    let mut dom: Dom = Dom::new();
    let p = dom.create_element("p");
    dom.set_attribute(p, "a", "bi").unwrap();
    dom.append_child(dom.root(), p).unwrap();
    for bad in ["[a i]", "[a=b x]", "[a=b i s]", "[a=b ii]", "[a=b 'i']"] {
        assert!(dom.matches(p, bad).is_err(), "{bad}");
    }
    assert_eq!(dom.matches(p, "[a=bi]"), Ok(true));
}
