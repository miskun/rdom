//! Selector grammar tests.
use super::*;

fn s(v: &str) -> SimpleSelector {
    SimpleSelector::Type(v.to_string())
}

#[test]
fn parse_type_selector() {
    let sl = parse("div").unwrap();
    assert_eq!(sl.0.len(), 1);
    assert_eq!(sl.0[0].subject.simples, vec![s("div")]);
    assert!(sl.0[0].ancestors.is_empty());
}

#[test]
fn parse_universal() {
    let sl = parse("*").unwrap();
    assert_eq!(sl.0[0].subject.simples, vec![SimpleSelector::Universal]);
}

#[test]
fn parse_id() {
    let sl = parse("#main").unwrap();
    assert_eq!(
        sl.0[0].subject.simples,
        vec![SimpleSelector::Id("main".into())]
    );
}

#[test]
fn parse_class() {
    let sl = parse(".foo").unwrap();
    assert_eq!(
        sl.0[0].subject.simples,
        vec![SimpleSelector::Class("foo".into())]
    );
}

/// CSS Syntax 3 §4.2 (`P7G-CORE-SMALL-1`): a code point at or above
/// U+0080 is an ident code point, start included.
#[test]
fn non_ascii_idents_parse() {
    match &parse("[lang|=én]").unwrap().0[0].subject.simples[0] {
        SimpleSelector::Attribute { value, .. } => assert_eq!(value.as_deref(), Some("én")),
        other => panic!("expected an attribute selector, got {other:?}"),
    }
    assert_eq!(
        parse(".café#naïve").unwrap().0[0].subject.simples,
        vec![
            SimpleSelector::Class("café".into()),
            SimpleSelector::Id("naïve".into()),
        ]
    );
    assert_eq!(
        parse("élément").unwrap().0[0].subject.simples,
        vec![s("élément")]
    );
    assert_eq!(
        parse(".日本").unwrap().0[0].subject.simples,
        vec![SimpleSelector::Class("日本".into())]
    );
}

#[test]
fn parse_compound() {
    let sl = parse("div.foo#bar.baz").unwrap();
    assert_eq!(
        sl.0[0].subject.simples,
        vec![
            s("div"),
            SimpleSelector::Class("foo".into()),
            SimpleSelector::Id("bar".into()),
            SimpleSelector::Class("baz".into()),
        ]
    );
}

#[test]
fn parse_attribute_presence() {
    let sl = parse("[disabled]").unwrap();
    assert_eq!(
        sl.0[0].subject.simples,
        vec![SimpleSelector::Attribute {
            name: "disabled".into(),
            op: None,
            value: None,
            case: AttrCase::Default,
        }]
    );
}

#[test]
fn parse_attribute_exact_unquoted() {
    let sl = parse("[role=banner]").unwrap();
    match &sl.0[0].subject.simples[0] {
        SimpleSelector::Attribute {
            name, op, value, ..
        } => {
            assert_eq!(name, "role");
            assert_eq!(*op, Some(AttrOp::Exact));
            assert_eq!(value.as_deref(), Some("banner"));
        }
        _ => panic!("expected attribute"),
    }
}

#[test]
fn parse_attribute_exact_quoted() {
    let sl = parse(r#"[title="hello world"]"#).unwrap();
    match &sl.0[0].subject.simples[0] {
        SimpleSelector::Attribute { value, .. } => {
            assert_eq!(value.as_deref(), Some("hello world"));
        }
        _ => panic!(),
    }
}

#[test]
fn parse_attribute_operators() {
    for (src, op) in [
        ("[a~=b]", AttrOp::Includes),
        ("[a|=b]", AttrOp::DashMatch),
        ("[a^=b]", AttrOp::Prefix),
        ("[a$=b]", AttrOp::Suffix),
        ("[a*=b]", AttrOp::Substring),
    ] {
        let sl = parse(src).unwrap();
        match &sl.0[0].subject.simples[0] {
            SimpleSelector::Attribute { op: got, .. } => assert_eq!(*got, Some(op)),
            _ => panic!("{src}"),
        }
    }
}

#[test]
fn parse_descendant() {
    let sl = parse("a b").unwrap();
    let complex = &sl.0[0];
    assert_eq!(complex.subject.simples, vec![s("b")]);
    assert_eq!(complex.ancestors.len(), 1);
    assert_eq!(complex.ancestors[0].0, Combinator::Descendant);
    assert_eq!(complex.ancestors[0].1.simples, vec![s("a")]);
}

#[test]
fn parse_child_combinator() {
    let sl = parse("a > b").unwrap();
    let c = &sl.0[0];
    assert_eq!(c.ancestors[0].0, Combinator::Child);
}

#[test]
fn parse_adjacent_and_general_sibling() {
    let sl = parse("a + b").unwrap();
    assert_eq!(sl.0[0].ancestors[0].0, Combinator::AdjacentSibling);
    let sl = parse("a ~ b").unwrap();
    assert_eq!(sl.0[0].ancestors[0].0, Combinator::GeneralSibling);
}

#[test]
fn parse_chain_right_to_left() {
    let sl = parse("a > b c").unwrap();
    let c = &sl.0[0];
    // Subject = c, ancestors (closest→outer) = [(Descendant, b), (Child, a)]
    assert_eq!(c.subject.simples, vec![s("c")]);
    assert_eq!(c.ancestors.len(), 2);
    assert_eq!(c.ancestors[0].0, Combinator::Descendant);
    assert_eq!(c.ancestors[0].1.simples, vec![s("b")]);
    assert_eq!(c.ancestors[1].0, Combinator::Child);
    assert_eq!(c.ancestors[1].1.simples, vec![s("a")]);
}

#[test]
fn parse_selector_list() {
    let sl = parse("a, b, c.foo").unwrap();
    assert_eq!(sl.0.len(), 3);
}

#[test]
fn parse_not_pseudo() {
    let sl = parse("div:not(.foo)").unwrap();
    match &sl.0[0].subject.simples[1] {
        SimpleSelector::Not(inner) => {
            assert_eq!(inner.0.len(), 1);
            assert_eq!(
                inner.0[0].subject.simples,
                vec![SimpleSelector::Class("foo".into())]
            );
        }
        _ => panic!(),
    }
}

#[test]
fn parse_where_pseudo() {
    let sl = parse("div:where(.foo, #bar)").unwrap();
    match &sl.0[0].subject.simples[1] {
        SimpleSelector::Where(inner) => assert_eq!(inner.0.len(), 2),
        other => panic!("expected Where, got {other:?}"),
    }
    // Combinators are allowed inside the argument.
    let sl = parse(":where(table:focus td)").unwrap();
    assert!(matches!(
        sl.0[0].subject.simples[0],
        SimpleSelector::Where(_)
    ));
}

#[test]
fn parse_structural_pseudos() {
    for (src, p) in [
        (":first-child", PseudoClass::FirstChild),
        (":last-child", PseudoClass::LastChild),
        (":only-child", PseudoClass::OnlyChild),
        (":empty", PseudoClass::Empty),
        (":root", PseudoClass::Root),
    ] {
        let sl = parse(src).unwrap();
        assert_eq!(sl.0[0].subject.simples, vec![SimpleSelector::Pseudo(p)]);
    }
}

/// Selectors 4 §3.1 / CSS Values 4 §2.1: pseudo-class names are
/// ASCII case-insensitive.
#[test]
fn pseudo_class_names_are_case_insensitive() {
    let sl = parse("a:HOVER:First-Child:NOT(.x)").unwrap();
    assert_eq!(
        sl.0[0].subject.simples[1..3],
        [
            SimpleSelector::Pseudo(PseudoClass::Hover),
            SimpleSelector::Pseudo(PseudoClass::FirstChild)
        ]
    );
    assert!(matches!(sl.0[0].subject.simples[3], SimpleSelector::Not(_)));
}

#[test]
fn parse_whitespace_tolerance() {
    parse("  a  ,  b  ").unwrap();
    parse("a   >   b").unwrap();
    parse("[ role = \"x\" ]").unwrap();
}

#[test]
fn parse_errors_on_empty_input() {
    assert!(parse("").is_err());
    assert!(parse("   ").is_err());
}

#[test]
fn parse_errors_on_trailing_garbage() {
    assert!(parse("div ))").is_err());
}

#[test]
fn parse_errors_on_unknown_pseudo() {
    assert!(parse(":banana").is_err());
}

#[test]
fn parse_errors_on_unterminated_attr() {
    assert!(parse("[foo=").is_err());
    assert!(parse("[foo=bar").is_err());
}

#[test]
fn parse_errors_on_pseudo_element() {
    assert!(parse("::before").is_err());
}

// ── Nested rule selectors (CSS Nesting 1 §2) ─────────────────────────

fn nested(input: &str, parent: &str) -> SelectorList {
    parse_nested(input, &parse(parent).unwrap()).unwrap()
}

/// §2: `&` alone splices a one-item parent in: `.a .b { & {} }` is
/// `.a .b`, the same selector and specificity.
#[test]
fn nesting_selector_splices_a_single_parent() {
    assert_eq!(nested("&", ".a .b"), parse(".a .b").unwrap());
    assert_eq!(nested("&.c", ".a .b"), parse(".a .b.c").unwrap());
    assert_eq!(nested("& > p", ".a"), parse(".a > p").unwrap());
}

/// §2: no `&` and no leading combinator — anchored with a descendant
/// combinator; a leading combinator anchors with itself.
#[test]
fn relative_nested_selectors_are_anchored() {
    assert_eq!(nested("p", ".a"), parse(".a p").unwrap());
    assert_eq!(nested("> p", ".a"), parse(".a > p").unwrap());
    assert_eq!(nested("+ p, ~ q", ".a"), parse(".a + p, .a ~ q").unwrap());
}

/// §2: with a multi-item parent `&` is `:is(<parent>)`, whose
/// specificity is the most specific item's.
#[test]
fn nesting_selector_is_an_is_list() {
    let list = nested("& p", "#x, .y");
    let complex = &list.0[0];
    assert_eq!(complex.specificity(), (1, 0, 1));
    let (_, outer) = &complex.ancestors[0];
    assert!(matches!(&outer.simples[..], [SimpleSelector::Is(inner)] if inner.0.len() == 2));
}

/// §2: `&` anywhere — after a compound, inside `:not()` — makes the
/// selector absolute (no implicit anchor).
#[test]
fn nesting_selector_in_any_position() {
    assert_eq!(nested(".w &", ".a"), parse(".w .a").unwrap());
    let not = nested(":not(&)", ".a");
    assert_eq!(not.0[0].ancestors.len(), 0);
    assert_eq!(not.0[0].specificity(), (0, 1, 0));
}

/// `&div` is invalid: a type selector must start a compound.
#[test]
fn nesting_selector_errors() {
    assert!(parse("&div").is_err());
    assert!(parse_nested("&div", &parse(".a").unwrap()).is_err());
}

// ── `:scope` and scoped selectors (Selectors 4 §14.3, Cascade 6 §2.5.2) ──

/// `:scope` parses as a pseudo-class with pseudo-class specificity;
/// outside a nested rule `&` is `:scope` (CSS Nesting 1 §2).
#[test]
fn scope_pseudo_class_parses() {
    let list = parse(":scope > p").unwrap();
    assert_eq!(list.0[0].specificity(), (0, 1, 1));
    assert_eq!(parse("&").unwrap(), parse(":scope").unwrap());
}

/// Cascade 6 §2.5.2: a scoped rule's selector is relative to
/// `:where(:scope)` (zero specificity) unless it holds `:scope` or `&`
/// (itself `:where(:scope)`) or starts with a combinator.
#[test]
fn scoped_selectors_are_relative_to_where_scope() {
    let where_scope = parse(":where(:scope)").unwrap();
    let p = parse_scoped("p").unwrap();
    assert_eq!(p.0[0].specificity(), (0, 0, 1));
    assert_eq!(p.0[0].ancestors.len(), 1);
    assert_eq!(p.0[0].ancestors[0].0, Combinator::Descendant);
    assert_eq!(p.0[0].ancestors[0].1, where_scope.0[0].subject);
    let child = parse_scoped("> p").unwrap();
    assert_eq!(child.0[0].ancestors[0].0, Combinator::Child);
    assert_eq!(
        parse_scoped(":scope > p").unwrap(),
        parse(":scope > p").unwrap()
    );
    assert_eq!(
        parse_scoped("& > p").unwrap(),
        parse(":where(:scope) > p").unwrap()
    );
}

// ── `:is()` (Selectors 4 §4.2, C1G-IS-PARSE) ────────────────────────

/// §4.2: `:is(<forgiving-selector-list>)` parses into
/// `SimpleSelector::Is`, the same node as the nesting `&`.
#[test]
fn is_pseudo_parses_into_is() {
    let sl = parse("p:is(.a, #b > em)").unwrap();
    assert!(matches!(
        &sl.0[0].subject.simples[..],
        [SimpleSelector::Type(_), SimpleSelector::Is(inner)] if inner.0.len() == 2
    ));
    assert_eq!(parse(":IS(.a)").unwrap(), parse(":is(.a)").unwrap());
}

/// §17: `:is()` counts the specificity of its most specific argument.
#[test]
fn is_specificity_is_its_most_specific_argument() {
    assert_eq!(parse(":is(.a, #b)").unwrap().0[0].specificity(), (1, 0, 0));
    assert_eq!(
        parse(":is(p, .a) span").unwrap().0[0].specificity(),
        (0, 1, 1)
    );
    assert_eq!(
        parse(":not(:is(.a, #b))").unwrap().0[0].specificity(),
        (1, 0, 0)
    );
}

/// §4.2 forgiving selector list: an argument that does not parse is
/// dropped and the rest stand; with none left, `:is()` is valid and
/// matches nothing. Nothing outside the parentheses is forgiven.
#[test]
fn is_arguments_are_forgiving() {
    let sl = parse(":is(.a, !!bad, :unknown-thing, .b)").unwrap();
    assert!(matches!(
        &sl.0[0].subject.simples[..],
        [SimpleSelector::Is(inner)] if *inner.0 == parse(".a, .b").unwrap().0
    ));
    let empty = parse("p:is(!!, [x=], \"a)\")").unwrap();
    assert!(matches!(
        &empty.0[0].subject.simples[..],
        [_, SimpleSelector::Is(inner)] if inner.0.is_empty()
    ));
    assert!(parse(":is(.a").is_err(), "unclosed");
    assert!(parse(":is(.a) !!").is_err());
    assert!(parse(":not(!!, .a)").is_err(), ":not() is not forgiving");
}

/// Selectors 4 §6.3: the case flag is part of the attribute selector,
/// and adds nothing to its specificity (§15).
#[test]
fn attribute_case_flag_parses_into_the_selector() {
    let flag = |src: &str| match &parse(src).unwrap().0[0].subject.simples[0] {
        SimpleSelector::Attribute { case, .. } => *case,
        other => panic!("{src}: {other:?}"),
    };
    assert_eq!(flag("[a=b]"), AttrCase::Default);
    assert_eq!(flag("[a=b i]"), AttrCase::AsciiInsensitive);
    assert_eq!(flag("[a=b s]"), AttrCase::Sensitive);
    assert_eq!(parse("[a=b i]").unwrap().0[0].specificity(), (0, 1, 0));
}
