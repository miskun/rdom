use super::*;
use crate::TuiDom;

fn dom_with(tag: &str) -> (TuiDom, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element(tag);
    dom.append_child(root, el).unwrap();
    (dom, el)
}

// ── value() ──────────────────────────────────────────────────

#[test]
fn value_on_div_is_none() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).value().is_none());
}

#[test]
fn value_on_text_input_reads_seeded_text_child() {
    let (mut dom, input) = dom_with("input");
    dom.set_attribute(input, "value", "hello").unwrap();
    crate::runtime::builtins::input::seed_all(&mut dom);
    assert_eq!(dom.node(input).value(), Some("hello".to_string()));
}

#[test]
fn value_on_unseeded_input_is_empty_string_not_none() {
    let (mut dom, input) = dom_with("input");
    dom.set_attribute(input, "value", "ignored-without-seed")
        .unwrap();
    assert_eq!(dom.node(input).value(), Some(String::new()));
}

#[test]
fn value_on_textarea_concatenates_text_descendants() {
    let (mut dom, ta) = dom_with("textarea");
    let t = dom.create_text_node("multi-line\ncontent");
    dom.append_child(ta, t).unwrap();
    assert_eq!(
        dom.node(ta).value(),
        Some("multi-line\ncontent".to_string())
    );
}

#[test]
fn value_on_select_returns_selected_option_value() {
    let (mut dom, sel) = dom_with("select");
    let opt1 = dom.create_element("option");
    dom.set_attribute(opt1, "value", "a").unwrap();
    let opt2 = dom.create_element("option");
    dom.set_attribute(opt2, "value", "b").unwrap();
    dom.set_attribute(opt2, "selected", "").unwrap();
    dom.append_child(sel, opt1).unwrap();
    dom.append_child(sel, opt2).unwrap();
    assert_eq!(dom.node(sel).value(), Some("b".to_string()));
}

// ── checked() / indeterminate() ──────────────────────────────

#[test]
fn checked_reflects_attribute_presence() {
    let (mut dom, input) = dom_with("input");
    assert!(!dom.node(input).checked());
    dom.set_attribute(input, "checked", "").unwrap();
    assert!(dom.node(input).checked());
}

#[test]
fn checked_on_div_with_attribute_still_reads_presence() {
    // Reflective accessor — no tag gating. Matches how the
    // `:checked` pseudo-class matches regardless of tag in
    // selector engine.
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "checked", "").unwrap();
    assert!(dom.node(div).checked());
}

#[test]
fn indeterminate_reflects_attribute_presence() {
    let (mut dom, input) = dom_with("input");
    assert!(!dom.node(input).indeterminate());
    dom.set_attribute(input, "indeterminate", "").unwrap();
    assert!(dom.node(input).indeterminate());
}

// ── disabled() / read_only() / inert() ───────────────────────

#[test]
fn disabled_reflects_attribute_presence() {
    let (mut dom, btn) = dom_with("button");
    assert!(!dom.node(btn).disabled());
    dom.set_attribute(btn, "disabled", "").unwrap();
    assert!(dom.node(btn).disabled());
}

#[test]
fn read_only_reads_html_readonly_attribute() {
    let (mut dom, input) = dom_with("input");
    assert!(!dom.node(input).read_only());
    // HTML attribute name is `readonly` (no underscore); the
    // accessor name is `read_only` for Rust style.
    dom.set_attribute(input, "readonly", "").unwrap();
    assert!(dom.node(input).read_only());
}

#[test]
fn inert_reflects_attribute_presence() {
    let (mut dom, div) = dom_with("div");
    assert!(!dom.node(div).inert());
    dom.set_attribute(div, "inert", "").unwrap();
    assert!(dom.node(div).inert());
}

// ── is_content_editable() ────────────────────────────────────

#[test]
fn is_content_editable_false_by_default() {
    let (dom, div) = dom_with("div");
    assert!(!dom.node(div).is_content_editable());
}

#[test]
fn is_content_editable_true_for_explicit_true() {
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "contenteditable", "true").unwrap();
    assert!(dom.node(div).is_content_editable());
}

#[test]
fn is_content_editable_true_for_html_boolean_shorthand() {
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "contenteditable", "").unwrap();
    assert!(dom.node(div).is_content_editable());
}

#[test]
fn is_content_editable_inherits_from_ancestor() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    let inner = dom.create_element("span");
    dom.append_child(root, outer).unwrap();
    dom.append_child(outer, inner).unwrap();
    dom.set_attribute(outer, "contenteditable", "true").unwrap();
    assert!(dom.node(inner).is_content_editable());
}

#[test]
fn is_content_editable_false_overrides_inherited_true() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    let inner = dom.create_element("span");
    dom.append_child(root, outer).unwrap();
    dom.append_child(outer, inner).unwrap();
    dom.set_attribute(outer, "contenteditable", "true").unwrap();
    dom.set_attribute(inner, "contenteditable", "false")
        .unwrap();
    assert!(!dom.node(inner).is_content_editable());
}

#[test]
fn is_content_editable_does_not_match_native_input() {
    // `<input>` is editable via `TuiNodeExt::is_editable` but
    // does NOT satisfy HTMLElement.isContentEditable.
    let (dom, input) = dom_with("input");
    assert!(!dom.node(input).is_content_editable());
}

#[test]
fn is_content_editable_plaintext_only_is_true() {
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "contenteditable", "plaintext-only")
        .unwrap();
    assert!(dom.node(div).is_content_editable());
}

// ── effective_tab_index() ────────────────────────────────────

#[test]
fn effective_tab_index_none_for_plain_div() {
    let (dom, div) = dom_with("div");
    assert_eq!(dom.node(div).effective_tab_index(), None);
}

#[test]
fn effective_tab_index_zero_for_implicit_focusable_input() {
    let (dom, input) = dom_with("input");
    assert_eq!(dom.node(input).effective_tab_index(), Some(0));
}

#[test]
fn effective_tab_index_zero_for_implicit_focusable_button() {
    let (dom, btn) = dom_with("button");
    assert_eq!(dom.node(btn).effective_tab_index(), Some(0));
}

#[test]
fn effective_tab_index_reads_explicit_attribute() {
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "tabindex", "3").unwrap();
    assert_eq!(dom.node(div).effective_tab_index(), Some(3));
}

#[test]
fn effective_tab_index_negative_via_explicit_attribute() {
    // Programmatic-only focusability still returns the raw value.
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "tabindex", "-1").unwrap();
    assert_eq!(dom.node(div).effective_tab_index(), Some(-1));
}

#[test]
fn effective_tab_index_disabled_overrides_everything() {
    let (mut dom, btn) = dom_with("button");
    dom.set_attribute(btn, "disabled", "").unwrap();
    // Disabled element is NEVER focusable, even with an explicit
    // tabindex.
    dom.set_attribute(btn, "tabindex", "5").unwrap();
    assert_eq!(dom.node(btn).effective_tab_index(), None);
}

#[test]
fn effective_tab_index_input_type_hidden_is_not_focusable() {
    let (mut dom, input) = dom_with("input");
    dom.set_attribute(input, "type", "hidden").unwrap();
    assert_eq!(dom.node(input).effective_tab_index(), None);
}

#[test]
fn effective_tab_index_anchor_needs_href() {
    let (mut dom, a) = dom_with("a");
    // No href → no implicit focus.
    assert_eq!(dom.node(a).effective_tab_index(), None);
    dom.set_attribute(a, "href", "/somewhere").unwrap();
    assert_eq!(dom.node(a).effective_tab_index(), Some(0));
}

// ── Per-tag accessors: <input> + <textarea> (step 30a) ───────

#[test]
fn input_value_reads_seeded_text_child() {
    let (mut dom, input) = dom_with("input");
    dom.set_attribute(input, "value", "hello").unwrap();
    crate::runtime::builtins::input::seed_all(&mut dom);
    assert_eq!(dom.node(input).input_value(), Some("hello".to_string()));
}

#[test]
fn input_value_returns_none_on_wrong_tag() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).input_value().is_none());
}

#[test]
fn input_type_defaults_to_text_when_attribute_absent() {
    // HTML default for an `<input>` without `type` is "text".
    let (dom, input) = dom_with("input");
    assert_eq!(dom.node(input).input_type(), Some("text".to_string()));
}

#[test]
fn input_type_reads_attribute_when_set() {
    let (mut dom, input) = dom_with("input");
    dom.set_attribute(input, "type", "password").unwrap();
    assert_eq!(dom.node(input).input_type(), Some("password".to_string()));
}

#[test]
fn input_type_returns_none_on_wrong_tag() {
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "type", "fake").unwrap();
    assert!(dom.node(div).input_type().is_none());
}

#[test]
fn input_name_reads_attribute() {
    let (mut dom, input) = dom_with("input");
    assert!(dom.node(input).input_name().is_none());
    dom.set_attribute(input, "name", "username").unwrap();
    assert_eq!(dom.node(input).input_name(), Some("username".to_string()));
}

#[test]
fn input_name_returns_none_on_wrong_tag() {
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "name", "x").unwrap();
    assert!(dom.node(div).input_name().is_none());
}

#[test]
fn input_placeholder_reads_attribute() {
    let (mut dom, input) = dom_with("input");
    dom.set_attribute(input, "placeholder", "Search…").unwrap();
    assert_eq!(
        dom.node(input).input_placeholder(),
        Some("Search…".to_string())
    );
}

#[test]
fn input_form_walks_to_form_ancestor() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let input = dom.create_element("input");
    dom.append_child(form, input).unwrap();
    dom.append_child(root, form).unwrap();
    assert_eq!(dom.node(input).input_form(), Some(form));
}

#[test]
fn input_form_returns_none_without_form_ancestor() {
    let (dom, input) = dom_with("input");
    assert_eq!(dom.node(input).input_form(), None);
}

#[test]
fn input_form_returns_none_on_wrong_tag() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let div = dom.create_element("div");
    dom.append_child(form, div).unwrap();
    dom.append_child(root, form).unwrap();
    // <div> is inside a form, but div_form/input_form is gated
    // by tag — non-input returns None even with a form ancestor.
    assert!(dom.node(div).input_form().is_none());
}

#[test]
fn textarea_value_concatenates_text_content() {
    let (mut dom, ta) = dom_with("textarea");
    let t = dom.create_text_node("line one\nline two");
    dom.append_child(ta, t).unwrap();
    assert_eq!(
        dom.node(ta).textarea_value(),
        Some("line one\nline two".to_string())
    );
}

#[test]
fn textarea_value_returns_none_on_wrong_tag() {
    let (mut dom, div) = dom_with("div");
    let t = dom.create_text_node("text");
    dom.append_child(div, t).unwrap();
    assert!(dom.node(div).textarea_value().is_none());
}

#[test]
fn textarea_name_reads_attribute() {
    let (mut dom, ta) = dom_with("textarea");
    dom.set_attribute(ta, "name", "comment").unwrap();
    assert_eq!(dom.node(ta).textarea_name(), Some("comment".to_string()));
}

#[test]
fn textarea_form_walks_to_form_ancestor() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let ta = dom.create_element("textarea");
    dom.append_child(form, ta).unwrap();
    dom.append_child(root, form).unwrap();
    assert_eq!(dom.node(ta).textarea_form(), Some(form));
}

// ── Per-tag accessors: <select> + <option> (step 30b) ────────

/// Build a `<select>` with three `<option value="…">` children.
/// `selected_idx` is the option index that gets the `selected`
/// attribute (None → none selected).
fn dom_with_select(selected_idx: Option<usize>) -> (TuiDom, NodeId, Vec<NodeId>) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let sel = dom.create_element("select");
    let mut opts = Vec::with_capacity(3);
    for (i, val) in ["a", "b", "c"].iter().enumerate() {
        let opt = dom.create_element("option");
        dom.set_attribute(opt, "value", val).unwrap();
        if Some(i) == selected_idx {
            dom.set_attribute(opt, "selected", "").unwrap();
        }
        dom.append_child(sel, opt).unwrap();
        opts.push(opt);
    }
    dom.append_child(root, sel).unwrap();
    (dom, sel, opts)
}

#[test]
fn select_value_returns_selected_option_value() {
    let (dom, sel, _) = dom_with_select(Some(1));
    assert_eq!(dom.node(sel).select_value(), Some("b".to_string()));
}

#[test]
fn select_value_empty_when_nothing_selected() {
    let (dom, sel, _) = dom_with_select(None);
    assert_eq!(dom.node(sel).select_value(), Some(String::new()));
}

#[test]
fn select_value_returns_none_on_wrong_tag() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).select_value().is_none());
}

#[test]
fn select_options_lists_all_option_descendants() {
    let (dom, sel, opts) = dom_with_select(None);
    let listed = dom.node(sel).select_options().unwrap();
    assert_eq!(listed, opts);
}

#[test]
fn select_options_some_empty_for_select_without_options() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let sel = dom.create_element("select");
    dom.append_child(root, sel).unwrap();
    assert_eq!(dom.node(sel).select_options(), Some(vec![]));
}

#[test]
fn select_options_returns_none_on_wrong_tag() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).select_options().is_none());
}

#[test]
fn select_selected_options_filters_to_selected() {
    let (dom, sel, opts) = dom_with_select(Some(2));
    let selected = dom.node(sel).select_selected_options().unwrap();
    assert_eq!(selected, vec![opts[2]]);
}

#[test]
fn select_selected_options_empty_when_none() {
    let (dom, sel, _) = dom_with_select(None);
    assert!(dom.node(sel).select_selected_options().unwrap().is_empty());
}

#[test]
fn select_selected_index_finds_first_selected() {
    let (dom, sel, _) = dom_with_select(Some(1));
    assert_eq!(dom.node(sel).select_selected_index(), Some(1));
}

#[test]
fn select_selected_index_is_minus_one_when_none() {
    // Browser HTMLSelectElement.selectedIndex returns -1 when
    // no option is selected.
    let (dom, sel, _) = dom_with_select(None);
    assert_eq!(dom.node(sel).select_selected_index(), Some(-1));
}

#[test]
fn select_selected_index_returns_none_on_wrong_tag() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).select_selected_index().is_none());
}

#[test]
fn select_form_walks_to_form_ancestor() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let sel = dom.create_element("select");
    dom.append_child(form, sel).unwrap();
    dom.append_child(root, form).unwrap();
    assert_eq!(dom.node(sel).select_form(), Some(form));
}

#[test]
fn option_value_reads_attribute() {
    let (dom, _sel, opts) = dom_with_select(None);
    assert_eq!(dom.node(opts[0]).option_value(), Some("a".to_string()));
}

#[test]
fn option_value_falls_back_to_text_when_attribute_absent() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let opt = dom.create_element("option");
    let t = dom.create_text_node("Pick me");
    dom.append_child(opt, t).unwrap();
    dom.append_child(root, opt).unwrap();
    assert_eq!(dom.node(opt).option_value(), Some("Pick me".to_string()));
}

#[test]
fn option_value_returns_none_on_wrong_tag() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).option_value().is_none());
}

#[test]
fn option_label_reads_attribute_then_text() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let opt = dom.create_element("option");
    dom.set_attribute(opt, "label", "Pretty Label").unwrap();
    let t = dom.create_text_node("Inner text");
    dom.append_child(opt, t).unwrap();
    dom.append_child(root, opt).unwrap();
    // Attribute wins.
    assert_eq!(
        dom.node(opt).option_label(),
        Some("Pretty Label".to_string())
    );
    // Without attribute → text content.
    dom.remove_attribute(opt, "label").unwrap();
    assert_eq!(dom.node(opt).option_label(), Some("Inner text".to_string()));
}

#[test]
fn option_selected_reflects_attribute() {
    let (dom, _sel, opts) = dom_with_select(Some(1));
    assert!(!dom.node(opts[0]).option_selected());
    assert!(dom.node(opts[1]).option_selected());
    assert!(!dom.node(opts[2]).option_selected());
}

#[test]
fn option_selected_false_on_wrong_tag() {
    let (mut dom, div) = dom_with("div");
    // Even if a non-option has [selected], option_selected
    // gates by tag.
    dom.set_attribute(div, "selected", "").unwrap();
    assert!(!dom.node(div).option_selected());
}

// ── Per-tag accessors: <details>/<dialog>/<button>/<label> ──
//                                              (step 30c)

#[test]
fn details_open_reflects_attribute() {
    let (mut dom, d) = dom_with("details");
    assert!(!dom.node(d).details_open());
    dom.set_attribute(d, "open", "").unwrap();
    assert!(dom.node(d).details_open());
}

#[test]
fn details_open_false_on_wrong_tag() {
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "open", "").unwrap();
    assert!(!dom.node(div).details_open());
}

#[test]
fn set_details_open_toggles_attribute() {
    let (mut dom, d) = dom_with("details");
    dom.node_mut(d).set_details_open(true).unwrap();
    assert!(dom.node(d).has_attribute("open"));
    dom.node_mut(d).set_details_open(false).unwrap();
    assert!(!dom.node(d).has_attribute("open"));
}

#[test]
fn set_details_open_no_op_on_wrong_tag() {
    let (mut dom, div) = dom_with("div");
    let r = dom.node_mut(div).set_details_open(true);
    assert!(r.is_ok());
    assert!(!dom.node(div).has_attribute("open"));
}

#[test]
fn dialog_open_reflects_attribute() {
    let (mut dom, dlg) = dom_with("dialog");
    assert!(!dom.node(dlg).dialog_open());
    dom.set_attribute(dlg, "open", "").unwrap();
    assert!(dom.node(dlg).dialog_open());
}

#[test]
fn dialog_open_false_on_wrong_tag() {
    let (mut dom, details) = dom_with("details");
    dom.set_attribute(details, "open", "").unwrap();
    // <details> has its own open semantics — dialog_open
    // gates by tag.
    assert!(!dom.node(details).dialog_open());
}

#[test]
fn dialog_return_value_empty_before_set() {
    let (dom, dlg) = dom_with("dialog");
    assert_eq!(dom.node(dlg).dialog_return_value(), Some(String::new()));
}

#[test]
fn dialog_return_value_returns_none_on_wrong_tag() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).dialog_return_value().is_none());
}

#[test]
fn set_dialog_return_value_stores_and_reads_back() {
    let (mut dom, dlg) = dom_with("dialog");
    dom.node_mut(dlg)
        .set_dialog_return_value("confirm")
        .unwrap();
    assert_eq!(
        dom.node(dlg).dialog_return_value(),
        Some("confirm".to_string())
    );
}

#[test]
fn set_dialog_return_value_no_op_on_wrong_tag() {
    let (mut dom, div) = dom_with("div");
    let r = dom.node_mut(div).set_dialog_return_value("x");
    assert!(r.is_ok());
    assert!(dom.node(div).dialog_return_value().is_none());
}

#[test]
fn button_form_walks_to_form_ancestor() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let btn = dom.create_element("button");
    dom.append_child(form, btn).unwrap();
    dom.append_child(root, form).unwrap();
    assert_eq!(dom.node(btn).button_form(), Some(form));
}

#[test]
fn button_form_returns_none_outside_form() {
    let (dom, btn) = dom_with("button");
    assert!(dom.node(btn).button_form().is_none());
}

#[test]
fn button_form_returns_none_on_wrong_tag() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let div = dom.create_element("div");
    dom.append_child(form, div).unwrap();
    dom.append_child(root, form).unwrap();
    assert!(dom.node(div).button_form().is_none());
}

#[test]
fn label_html_for_reads_attribute() {
    let (mut dom, lbl) = dom_with("label");
    assert!(dom.node(lbl).label_html_for().is_none());
    dom.set_attribute(lbl, "for", "username").unwrap();
    assert_eq!(dom.node(lbl).label_html_for(), Some("username".to_string()));
}

#[test]
fn label_html_for_returns_none_on_wrong_tag() {
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "for", "x").unwrap();
    assert!(dom.node(div).label_html_for().is_none());
}

#[test]
fn label_control_resolves_explicit_for_attribute() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let lbl = dom.create_element("label");
    dom.set_attribute(lbl, "for", "name-field").unwrap();
    let input = dom.create_element("input");
    dom.set_attribute(input, "id", "name-field").unwrap();
    dom.append_child(root, lbl).unwrap();
    dom.append_child(root, input).unwrap();
    assert_eq!(dom.node(lbl).label_control(), Some(input));
}

#[test]
fn label_control_falls_back_to_labelable_descendant() {
    // HTML implicit-wrap: <label>Name <input></label>
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let lbl = dom.create_element("label");
    let input = dom.create_element("input");
    dom.append_child(lbl, input).unwrap();
    dom.append_child(root, lbl).unwrap();
    assert_eq!(dom.node(lbl).label_control(), Some(input));
}

#[test]
fn label_control_returns_none_on_wrong_tag() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).label_control().is_none());
}

// ── Per-tag accessors: <progress> + <meter> (step 30d) ───────

#[test]
fn progress_value_defaults_to_zero_when_attribute_absent() {
    // HTML indeterminate progress: no `value` attribute.
    // IDL `progress.value` returns 0 in that case.
    let (dom, p) = dom_with("progress");
    assert_eq!(dom.node(p).progress_value(), Some(0.0));
}

#[test]
fn progress_value_parses_attribute() {
    let (mut dom, p) = dom_with("progress");
    dom.set_attribute(p, "value", "0.42").unwrap();
    assert_eq!(dom.node(p).progress_value(), Some(0.42));
}

#[test]
fn progress_value_returns_none_on_wrong_tag() {
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "value", "0.5").unwrap();
    assert!(dom.node(div).progress_value().is_none());
}

#[test]
fn progress_max_defaults_to_one_when_attribute_absent() {
    let (dom, p) = dom_with("progress");
    assert_eq!(dom.node(p).progress_max(), Some(1.0));
}

#[test]
fn progress_max_parses_attribute() {
    let (mut dom, p) = dom_with("progress");
    dom.set_attribute(p, "max", "100").unwrap();
    assert_eq!(dom.node(p).progress_max(), Some(100.0));
}

#[test]
fn meter_value_defaults_to_zero() {
    let (dom, m) = dom_with("meter");
    assert_eq!(dom.node(m).meter_value(), Some(0.0));
}

#[test]
fn meter_value_parses_attribute() {
    let (mut dom, m) = dom_with("meter");
    dom.set_attribute(m, "value", "0.75").unwrap();
    assert_eq!(dom.node(m).meter_value(), Some(0.75));
}

#[test]
fn meter_min_max_defaults() {
    let (dom, m) = dom_with("meter");
    assert_eq!(dom.node(m).meter_min(), Some(0.0));
    assert_eq!(dom.node(m).meter_max(), Some(1.0));
}

#[test]
fn meter_low_falls_back_to_min() {
    let (mut dom, m) = dom_with("meter");
    dom.set_attribute(m, "min", "10").unwrap();
    // No `low` attribute → defaults to `min`.
    assert_eq!(dom.node(m).meter_low(), Some(10.0));
}

#[test]
fn meter_low_parses_attribute() {
    let (mut dom, m) = dom_with("meter");
    dom.set_attribute(m, "low", "0.3").unwrap();
    assert_eq!(dom.node(m).meter_low(), Some(0.3));
}

#[test]
fn meter_high_falls_back_to_max() {
    let (mut dom, m) = dom_with("meter");
    dom.set_attribute(m, "max", "100").unwrap();
    // No `high` attribute → defaults to `max`.
    assert_eq!(dom.node(m).meter_high(), Some(100.0));
}

#[test]
fn meter_high_parses_attribute() {
    let (mut dom, m) = dom_with("meter");
    dom.set_attribute(m, "high", "0.8").unwrap();
    assert_eq!(dom.node(m).meter_high(), Some(0.8));
}

#[test]
fn meter_optimum_defaults_to_midpoint() {
    // No min/max/optimum attrs → (0 + 1) / 2 = 0.5.
    let (dom, m) = dom_with("meter");
    assert_eq!(dom.node(m).meter_optimum(), Some(0.5));
}

#[test]
fn meter_optimum_midpoint_respects_custom_range() {
    let (mut dom, m) = dom_with("meter");
    dom.set_attribute(m, "min", "10").unwrap();
    dom.set_attribute(m, "max", "30").unwrap();
    // No optimum → (10 + 30) / 2 = 20.
    assert_eq!(dom.node(m).meter_optimum(), Some(20.0));
}

#[test]
fn meter_optimum_parses_attribute() {
    let (mut dom, m) = dom_with("meter");
    dom.set_attribute(m, "optimum", "0.9").unwrap();
    assert_eq!(dom.node(m).meter_optimum(), Some(0.9));
}

#[test]
fn meter_accessors_return_none_on_wrong_tag() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).meter_value().is_none());
    assert!(dom.node(div).meter_min().is_none());
    assert!(dom.node(div).meter_max().is_none());
    assert!(dom.node(div).meter_low().is_none());
    assert!(dom.node(div).meter_high().is_none());
    assert!(dom.node(div).meter_optimum().is_none());
}

#[test]
fn invalid_numeric_value_falls_back_to_default() {
    // Per `parse_attr`-style helper: parse error → falls back.
    let (mut dom, p) = dom_with("progress");
    dom.set_attribute(p, "value", "not-a-number").unwrap();
    assert_eq!(dom.node(p).progress_value(), Some(0.0));
}

// ── Per-tag accessors: <form> (step 31) ──────────────────────

/// Build a `<form>` containing one input, one button, and a
/// nested div wrapping a select. Mirrors a realistic form
/// shape — the listed-elements walker descends into nested
/// children but only collects form controls.
fn dom_with_form() -> (TuiDom, NodeId, NodeId, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    let input = dom.create_element("input");
    let button = dom.create_element("button");
    // Nested under a non-control div — walker must still
    // find the <select> inside.
    let wrapper = dom.create_element("div");
    let select = dom.create_element("select");
    dom.append_child(wrapper, select).unwrap();
    dom.append_child(form, input).unwrap();
    dom.append_child(form, button).unwrap();
    dom.append_child(form, wrapper).unwrap();
    dom.append_child(root, form).unwrap();
    (dom, form, input, button, select)
}

#[test]
fn form_elements_lists_form_controls_in_document_order() {
    let (dom, form, input, button, select) = dom_with_form();
    let elts = dom.node(form).form_elements().unwrap();
    assert_eq!(elts, vec![input, button, select]);
}

#[test]
fn form_elements_excludes_the_form_itself() {
    // Sanity: <form> is a listed element in HTML but NOT in
    // `form.elements` (avoid the recursion).
    let (dom, form, _, _, _) = dom_with_form();
    let elts = dom.node(form).form_elements().unwrap();
    assert!(!elts.contains(&form));
}

#[test]
fn form_elements_some_empty_for_form_without_controls() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form = dom.create_element("form");
    dom.append_child(root, form).unwrap();
    assert_eq!(dom.node(form).form_elements(), Some(vec![]));
}

#[test]
fn form_elements_returns_none_on_wrong_tag() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).form_elements().is_none());
}

#[test]
fn form_length_matches_elements_len() {
    let (dom, form, _, _, _) = dom_with_form();
    assert_eq!(dom.node(form).form_length(), Some(3));
}

#[test]
fn form_length_returns_none_on_wrong_tag() {
    let (dom, div) = dom_with("div");
    assert!(dom.node(div).form_length().is_none());
}

#[test]
fn form_request_submit_fires_submit_with_submitter_detail() {
    use rdom_core::EventDetail;
    use std::cell::RefCell;
    use std::rc::Rc;
    let (mut dom, form, _, button, _) = dom_with_form();
    let captured: Rc<RefCell<Option<EventDetail>>> = Rc::new(RefCell::new(None));
    {
        let captured = captured.clone();
        dom.add_event_listener(
            form,
            "submit",
            rdom_core::ListenerOptions::default(),
            move |ctx| {
                *captured.borrow_mut() = Some(ctx.event.detail.clone());
            },
        )
        .unwrap();
    }
    let prevented = dom
        .node_mut(form)
        .form_request_submit(Some(button))
        .unwrap();
    assert!(!prevented);
    let detail = captured.borrow().clone().expect("submit listener fired");
    let submitter = match detail {
        EventDetail::Submit(s) => s.submitter,
        other => panic!("expected EventDetail::Submit, got {other:?}"),
    };
    assert_eq!(submitter, Some(button));
}

#[test]
fn form_request_submit_with_none_submitter() {
    use rdom_core::EventDetail;
    use std::cell::RefCell;
    use std::rc::Rc;
    let (mut dom, form, _, _, _) = dom_with_form();
    let captured: Rc<RefCell<Option<EventDetail>>> = Rc::new(RefCell::new(None));
    {
        let captured = captured.clone();
        dom.add_event_listener(
            form,
            "submit",
            rdom_core::ListenerOptions::default(),
            move |ctx| {
                *captured.borrow_mut() = Some(ctx.event.detail.clone());
            },
        )
        .unwrap();
    }
    dom.node_mut(form).form_request_submit(None).unwrap();
    let detail = captured.borrow().clone().unwrap();
    let submitter = match detail {
        EventDetail::Submit(s) => s.submitter,
        other => panic!("expected EventDetail::Submit, got {other:?}"),
    };
    assert_eq!(submitter, None);
}

#[test]
fn form_request_submit_returns_true_when_prevented() {
    let (mut dom, form, _, _, _) = dom_with_form();
    dom.add_event_listener(
        form,
        "submit",
        rdom_core::ListenerOptions::default(),
        |ctx| ctx.event.prevent_default(),
    )
    .unwrap();
    let prevented = dom.node_mut(form).form_request_submit(None).unwrap();
    assert!(prevented);
}

#[test]
fn form_request_submit_no_op_on_wrong_tag() {
    let (mut dom, div) = dom_with("div");
    let prevented = dom.node_mut(div).form_request_submit(None).unwrap();
    assert!(!prevented);
}

// ── set_value() ──────────────────────────────────────────────

#[test]
fn set_value_on_text_input_updates_attribute_and_text_child() {
    let (mut dom, input) = dom_with("input");
    dom.node_mut(input).set_value("hello").unwrap();
    assert_eq!(dom.node(input).get_attribute("value"), Some("hello"));
    // value() reads via the text child (input::value).
    assert_eq!(dom.node(input).value(), Some("hello".to_string()));
}

#[test]
fn set_value_on_password_input_uses_text_family_path() {
    let (mut dom, input) = dom_with("input");
    dom.set_attribute(input, "type", "password").unwrap();
    dom.node_mut(input).set_value("secret").unwrap();
    assert_eq!(dom.node(input).get_attribute("value"), Some("secret"));
    assert_eq!(dom.node(input).value(), Some("secret".to_string()));
}

#[test]
fn set_value_on_submit_button_only_writes_attribute() {
    // Non-text-family input: writes the `value` attribute but
    // does NOT install a text-node child (the UA ::before
    // provides the glyph for these tag/type combos).
    let (mut dom, input) = dom_with("input");
    dom.set_attribute(input, "type", "submit").unwrap();
    dom.node_mut(input).set_value("Send").unwrap();
    assert_eq!(dom.node(input).get_attribute("value"), Some("Send"));
    // No text child was added.
    assert_eq!(dom.node(input).child_nodes().count(), 0);
}

#[test]
fn set_value_on_textarea_replaces_children_with_text() {
    let (mut dom, ta) = dom_with("textarea");
    let preexisting = dom.create_text_node("old");
    dom.append_child(ta, preexisting).unwrap();
    dom.node_mut(ta).set_value("new content").unwrap();
    assert_eq!(dom.node(ta).text_content(), "new content");
    assert_eq!(dom.node(ta).value(), Some("new content".to_string()));
}

#[test]
fn set_value_on_select_selects_first_matching_option() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let sel = dom.create_element("select");
    let opt_a = dom.create_element("option");
    dom.set_attribute(opt_a, "value", "a").unwrap();
    let opt_b = dom.create_element("option");
    dom.set_attribute(opt_b, "value", "b").unwrap();
    let opt_c = dom.create_element("option");
    dom.set_attribute(opt_c, "value", "c").unwrap();
    dom.append_child(sel, opt_a).unwrap();
    dom.append_child(sel, opt_b).unwrap();
    dom.append_child(sel, opt_c).unwrap();
    dom.append_child(root, sel).unwrap();

    dom.node_mut(sel).set_value("b").unwrap();
    assert!(!dom.node(opt_a).has_attribute("selected"));
    assert!(dom.node(opt_b).has_attribute("selected"));
    assert!(!dom.node(opt_c).has_attribute("selected"));
    assert_eq!(dom.node(sel).value(), Some("b".to_string()));
}

#[test]
fn set_value_on_select_clears_selection_when_no_match() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let sel = dom.create_element("select");
    let opt = dom.create_element("option");
    dom.set_attribute(opt, "value", "a").unwrap();
    dom.set_attribute(opt, "selected", "").unwrap();
    dom.append_child(sel, opt).unwrap();
    dom.append_child(root, sel).unwrap();

    dom.node_mut(sel).set_value("missing").unwrap();
    assert!(!dom.node(opt).has_attribute("selected"));
}

#[test]
fn set_value_on_div_is_silent_no_op() {
    // M4 wrong-tag policy (§3.4.1): silent Ok(()), tree unchanged.
    let (mut dom, div) = dom_with("div");
    let pre_attrs: Vec<(String, String)> = dom
        .node(div)
        .attributes()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let pre_children = dom.node(div).child_nodes().count();
    let r = dom.node_mut(div).set_value("ignored");
    assert!(r.is_ok());
    let post_attrs: Vec<(String, String)> = dom
        .node(div)
        .attributes()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    assert_eq!(pre_attrs, post_attrs, "attributes unchanged");
    assert_eq!(
        dom.node(div).child_nodes().count(),
        pre_children,
        "no children added"
    );
}

#[test]
fn set_value_accepts_owned_string_too() {
    // The trait signature is `impl Into<String>`, so both &str
    // and String pass — same ergonomics as the browser IDL.
    let (mut dom, input) = dom_with("input");
    dom.node_mut(input)
        .set_value(String::from("owned"))
        .unwrap();
    assert_eq!(dom.node(input).value(), Some("owned".to_string()));
}

// ── set_checked() / set_indeterminate() ──────────────────────

#[test]
fn set_checked_true_writes_attribute() {
    let (mut dom, input) = dom_with("input");
    dom.node_mut(input).set_checked(true).unwrap();
    assert!(dom.node(input).has_attribute("checked"));
    assert!(dom.node(input).checked());
}

#[test]
fn set_checked_false_removes_attribute() {
    let (mut dom, input) = dom_with("input");
    dom.set_attribute(input, "checked", "").unwrap();
    dom.node_mut(input).set_checked(false).unwrap();
    assert!(!dom.node(input).has_attribute("checked"));
    assert!(!dom.node(input).checked());
}

#[test]
fn set_checked_on_div_is_no_op() {
    let (mut dom, div) = dom_with("div");
    let r = dom.node_mut(div).set_checked(true);
    assert!(r.is_ok());
    assert!(!dom.node(div).has_attribute("checked"));
}

#[test]
fn set_indeterminate_toggles_attribute() {
    let (mut dom, input) = dom_with("input");
    dom.node_mut(input).set_indeterminate(true).unwrap();
    assert!(dom.node(input).has_attribute("indeterminate"));
    dom.node_mut(input).set_indeterminate(false).unwrap();
    assert!(!dom.node(input).has_attribute("indeterminate"));
}

#[test]
fn set_indeterminate_on_div_is_no_op() {
    let (mut dom, div) = dom_with("div");
    let r = dom.node_mut(div).set_indeterminate(true);
    assert!(r.is_ok());
    assert!(!dom.node(div).has_attribute("indeterminate"));
}

// ── set_disabled() ───────────────────────────────────────────

#[test]
fn set_disabled_applies_to_owning_tags() {
    for tag in [
        "button", "input", "select", "textarea", "option", "optgroup", "fieldset",
    ] {
        let (mut dom, el) = dom_with(tag);
        dom.node_mut(el).set_disabled(true).unwrap();
        assert!(
            dom.node(el).has_attribute("disabled"),
            "<{tag}> should accept set_disabled"
        );
    }
}

#[test]
fn set_disabled_false_removes_attribute() {
    let (mut dom, input) = dom_with("input");
    dom.set_attribute(input, "disabled", "").unwrap();
    dom.node_mut(input).set_disabled(false).unwrap();
    assert!(!dom.node(input).has_attribute("disabled"));
}

#[test]
fn set_disabled_on_div_is_no_op() {
    let (mut dom, div) = dom_with("div");
    let r = dom.node_mut(div).set_disabled(true);
    assert!(r.is_ok());
    assert!(!dom.node(div).has_attribute("disabled"));
}

// ── set_read_only() ──────────────────────────────────────────

#[test]
fn set_read_only_applies_to_input_and_textarea() {
    for tag in ["input", "textarea"] {
        let (mut dom, el) = dom_with(tag);
        dom.node_mut(el).set_read_only(true).unwrap();
        assert!(
            dom.node(el).has_attribute("readonly"),
            "<{tag}> should accept set_read_only"
        );
        dom.node_mut(el).set_read_only(false).unwrap();
        assert!(!dom.node(el).has_attribute("readonly"));
    }
}

#[test]
fn set_read_only_on_div_is_no_op() {
    let (mut dom, div) = dom_with("div");
    let r = dom.node_mut(div).set_read_only(true);
    assert!(r.is_ok());
    assert!(!dom.node(div).has_attribute("readonly"));
}

// ── set_inert() ──────────────────────────────────────────────

#[test]
fn set_inert_applies_to_any_element() {
    // `inert` is an HTMLElement-level global attribute — works
    // on every element with no tag gate.
    for tag in ["div", "section", "button", "main"] {
        let (mut dom, el) = dom_with(tag);
        dom.node_mut(el).set_inert(true).unwrap();
        assert!(
            dom.node(el).has_attribute("inert"),
            "<{tag}> should accept set_inert"
        );
    }
}

#[test]
fn set_inert_false_removes_attribute() {
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "inert", "").unwrap();
    dom.node_mut(div).set_inert(false).unwrap();
    assert!(!dom.node(div).has_attribute("inert"));
}

// ── Read-then-mutate borrow pattern (spec step 21 criterion) ──

#[test]
fn read_then_mutate_compiles_in_single_block() {
    // `let v = el.value(); el.set_value("x")?;` must satisfy the
    // borrow checker in a single block. The read returns owned
    // String (drops the immutable borrow), so the write below
    // acquires the mut borrow without conflict.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.append_child(root, input).unwrap();
    dom.set_attribute(input, "value", "before").unwrap();
    crate::runtime::builtins::input::seed_all(&mut dom);

    let mut el = dom.node_mut(input);
    let prev = el.value();
    el.set_value("after").unwrap();
    let next = el.value();

    assert_eq!(prev, Some("before".to_string()));
    assert_eq!(next, Some("after".to_string()));
}

// ── focus() ──────────────────────────────────────────────────

#[test]
fn focus_on_focusable_input_sets_focused() {
    let (mut dom, input) = dom_with("input");
    assert_eq!(dom.focused(), None);
    dom.node_mut(input).focus();
    assert_eq!(dom.focused(), Some(input));
}

#[test]
fn focus_on_div_without_tabindex_is_no_op() {
    // Non-focusable element — browser-faithful: no focus change.
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div).focus();
    assert_eq!(dom.focused(), None);
}

#[test]
fn focus_on_div_with_tabindex_works() {
    let (mut dom, div) = dom_with("div");
    dom.set_attribute(div, "tabindex", "0").unwrap();
    dom.node_mut(div).focus();
    assert_eq!(dom.focused(), Some(div));
}

#[test]
fn focus_on_disabled_button_is_no_op() {
    // tabindex::tab_index returns None for disabled elements;
    // is_focusable wraps that. So focus() bails.
    let (mut dom, btn) = dom_with("button");
    dom.set_attribute(btn, "disabled", "").unwrap();
    dom.node_mut(btn).focus();
    assert_eq!(dom.focused(), None);
}

#[test]
fn focus_fires_focus_event_on_target() {
    use std::cell::Cell;
    use std::rc::Rc;
    let (mut dom, input) = dom_with("input");
    let fired = Rc::new(Cell::new(false));
    {
        let fired = fired.clone();
        dom.add_event_listener(
            input,
            "focus",
            rdom_core::ListenerOptions::default(),
            move |_ctx| fired.set(true),
        )
        .unwrap();
    }
    dom.node_mut(input).focus();
    assert!(fired.get(), "focus event should fire");
}

// ── blur() ───────────────────────────────────────────────────

#[test]
fn blur_clears_focus_when_self_is_focused() {
    let (mut dom, input) = dom_with("input");
    dom.set_focused(Some(input));
    dom.node_mut(input).blur();
    assert_eq!(dom.focused(), None);
}

#[test]
fn blur_is_no_op_when_self_not_focused() {
    // Per HTMLElement.blur — only acts when this element holds
    // focus. Other-element focus stays intact.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("input");
    let b = dom.create_element("input");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    dom.set_focused(Some(a));
    dom.node_mut(b).blur();
    assert_eq!(dom.focused(), Some(a));
}

// ── click() ──────────────────────────────────────────────────

#[test]
fn click_synthesizes_canonical_mouse_detail() {
    use rdom_core::{EventDetail, KeyboardModifiers, MouseButton};
    use std::cell::RefCell;
    use std::rc::Rc;
    let (mut dom, btn) = dom_with("button");
    let captured: Rc<RefCell<Option<EventDetail>>> = Rc::new(RefCell::new(None));
    {
        let captured = captured.clone();
        dom.add_event_listener(
            btn,
            "click",
            rdom_core::ListenerOptions::default(),
            move |ctx| {
                *captured.borrow_mut() = Some(ctx.event.detail.clone());
            },
        )
        .unwrap();
    }
    dom.node_mut(btn).click();

    let detail = captured.borrow().clone().expect("click listener fired");
    let m = match detail {
        EventDetail::Mouse(m) => m,
        other => panic!("expected EventDetail::Mouse, got {other:?}"),
    };
    assert_eq!(m.button, MouseButton::Left);
    assert_eq!(m.buttons, 0);
    assert_eq!(m.client_x, 0);
    assert_eq!(m.client_y, 0);
    assert_eq!(m.delta_x, 0);
    assert_eq!(m.delta_y, 0);
    assert_eq!(m.modifiers, KeyboardModifiers::default());
}

#[test]
fn click_marks_event_synthetic() {
    use std::cell::Cell;
    use std::rc::Rc;
    let (mut dom, btn) = dom_with("button");
    let synthetic = Rc::new(Cell::new(false));
    {
        let synthetic = synthetic.clone();
        dom.add_event_listener(
            btn,
            "click",
            rdom_core::ListenerOptions::default(),
            move |ctx| synthetic.set(ctx.event.is_synthetic()),
        )
        .unwrap();
    }
    dom.node_mut(btn).click();
    assert!(
        synthetic.get(),
        "click() should mark the synthesized event synthetic"
    );
}

// ── bounding_rect() / scroll_* read side ─────────────────────

#[test]
fn bounding_rect_returns_layout_rect_from_ext() {
    use crate::layout::LayoutRect;
    let (mut dom, div) = dom_with("div");
    // Poke the layout pass's output directly — we're testing
    // the accessor reads `ext.layout`, not the layout pipeline
    // itself (covered exhaustively in `render/layout_pass/`).
    {
        let mut nm = dom.node_mut(div);
        let ext = nm.ext_mut().unwrap();
        ext.layout = LayoutRect::new(3, 5, 20, 7);
    }
    assert_eq!(
        dom.node(div).bounding_rect(),
        Some(LayoutRect::new(3, 5, 20, 7))
    );
}

#[test]
fn bounding_rect_default_when_no_layout_run() {
    // Fresh element: layout is the default zero rect, but it
    // still returns Some — `None` is only for non-elements.
    use crate::layout::LayoutRect;
    let (dom, div) = dom_with("div");
    assert_eq!(dom.node(div).bounding_rect(), Some(LayoutRect::default()));
}

#[test]
fn bounding_rect_returns_shifted_rect_after_position_relative() {
    // D-M2-1 retirement: `position: relative` shifts the element's
    // `TuiExt.layout` rect at layout time. The shift is *visible*
    // to `bounding_rect()` — matches browser
    // `getBoundingClientRect()` semantics, which return the
    // *visual* (post-shift) rect, not the in-flow position.
    use crate::Rect;
    use crate::layout::{Direction, Flow, Length, Position as LayoutPosition, Size};
    use crate::render::layout_pass::LayoutExt;
    use crate::style::{CascadeExt, Stylesheet};
    use crate::{TuiDom, TuiStyle};

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("c");
    let rel = dom.create_element("rel");
    dom.append_child(c, rel).unwrap();
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "c",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked(
            "rel",
            TuiStyle::new()
                .position(LayoutPosition::Relative)
                .top(Length::Cells(2))
                .left(Length::Cells(3))
                .width(Size::Fixed(10))
                .height(Size::Fixed(2)),
        );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 10));

    // In-flow x/y would be (0, 0); after relative shift it's (3, 2).
    // bounding_rect() returns the visual (shifted) rect.
    let r = dom.node(rel).bounding_rect().expect("element has layout");
    assert_eq!(r.x, 3, "x shifted by left:3");
    assert_eq!(r.y, 2, "y shifted by top:2");
    assert_eq!(r.width, 10);
    assert_eq!(r.height, 2);
}

#[test]
fn scroll_top_left_default_to_zero() {
    let (dom, div) = dom_with("div");
    assert_eq!(dom.node(div).scroll_top(), Some(0));
    assert_eq!(dom.node(div).scroll_left(), Some(0));
}

#[test]
fn scroll_width_height_reflect_ext() {
    use crate::node::TuiNodeMutExt;
    let (mut dom, div) = dom_with("div");
    // The scrollbar runtime + paint write these from the layout
    // pass. We poke them directly here to assert the accessor
    // reads what's written.
    dom.node_mut(div).ext_mut().unwrap().scroll_content_width = 200;
    dom.node_mut(div).ext_mut().unwrap().scroll_content_height = 80;
    let _ = TuiNodeMutExt::set_scroll(&mut dom.node_mut(div), 0, 0);
    assert_eq!(dom.node(div).scroll_width(), Some(200));
    assert_eq!(dom.node(div).scroll_height(), Some(80));
}

// ── scroll write side ────────────────────────────────────────

fn dom_with_scrollable_div() -> (TuiDom, NodeId) {
    // Set up the ext so the clamping range allows scroll
    // changes: content > viewport, on both axes.
    let (mut dom, div) = dom_with("div");
    {
        let mut nm = dom.node_mut(div);
        let ext = nm.ext_mut().unwrap();
        // `write_scroll_clamped` reads `layout` (to derive the
        // padding-box viewport per CSS Overflow 3 §3); with no border
        // applied here, layout and content_layout coincide.
        ext.layout.width = 50;
        ext.layout.height = 20;
        ext.content_layout.width = 50;
        ext.content_layout.height = 20;
        ext.scroll_content_width = 200;
        ext.scroll_content_height = 100;
        ext.overflow = crate::layout::Overflow::Auto;
    }
    (dom, div)
}

#[test]
fn set_scroll_top_writes_and_clamps() {
    let (mut dom, div) = dom_with_scrollable_div();
    dom.node_mut(div).set_scroll_top(30).unwrap();
    assert_eq!(dom.node(div).scroll_top(), Some(30));
    // Clamp: max_y = 100 - 20 = 80.
    dom.node_mut(div).set_scroll_top(9999).unwrap();
    assert_eq!(dom.node(div).scroll_top(), Some(80));
    // Negative clamps to 0.
    dom.node_mut(div).set_scroll_top(-5).unwrap();
    assert_eq!(dom.node(div).scroll_top(), Some(0));
}

#[test]
fn set_scroll_left_writes_and_clamps() {
    let (mut dom, div) = dom_with_scrollable_div();
    dom.node_mut(div).set_scroll_left(40).unwrap();
    assert_eq!(dom.node(div).scroll_left(), Some(40));
    // max_x = 200 - 50 = 150.
    dom.node_mut(div).set_scroll_left(9999).unwrap();
    assert_eq!(dom.node(div).scroll_left(), Some(150));
}

#[test]
fn scroll_to_writes_both_axes() {
    let (mut dom, div) = dom_with_scrollable_div();
    dom.node_mut(div).scroll_to(10, 20).unwrap();
    assert_eq!(dom.node(div).scroll_left(), Some(10));
    assert_eq!(dom.node(div).scroll_top(), Some(20));
}

#[test]
fn scroll_by_adds_deltas() {
    let (mut dom, div) = dom_with_scrollable_div();
    dom.node_mut(div).scroll_to(10, 20).unwrap();
    dom.node_mut(div).scroll_by(5, -5).unwrap();
    assert_eq!(dom.node(div).scroll_left(), Some(15));
    assert_eq!(dom.node(div).scroll_top(), Some(15));
}

#[test]
fn set_scroll_top_on_non_scrollable_is_silent_no_op() {
    // Default ext: no scrollable content (content == viewport
    // == 0). Max scroll = 0, so any value clamps to 0.
    let (mut dom, div) = dom_with("div");
    dom.node_mut(div).set_scroll_top(50).unwrap();
    assert_eq!(dom.node(div).scroll_top(), Some(0));
}

#[test]
fn scroll_into_view_no_op_without_scrollable_ancestor() {
    // Default overflow on every ancestor: scroll_into_view bails.
    let (mut dom, div) = dom_with("div");
    let r = dom.node_mut(div).scroll_into_view();
    assert!(r.is_ok());
}

#[test]
fn scroll_into_view_scrolls_direct_parent() {
    use crate::layout::Overflow;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    dom.append_child(root, parent).unwrap();
    let child = dom.create_element("div");
    dom.append_child(parent, child).unwrap();

    // Configure parent as scrollable; pretend the layout pass
    // placed child at y=50 inside it (with parent currently at
    // scroll_y=0).
    {
        let mut nm = dom.node_mut(parent);
        let pe = nm.ext_mut().unwrap();
        pe.overflow = Overflow::Auto;
        pe.content_layout.width = 80;
        pe.content_layout.height = 20;
        pe.scroll_content_width = 80;
        pe.scroll_content_height = 200;
    }
    {
        let mut nm = dom.node_mut(child);
        let ce = nm.ext_mut().unwrap();
        ce.layout.x = 0;
        ce.layout.y = 50;
    }

    dom.node_mut(child).scroll_into_view().unwrap();
    assert_eq!(dom.node(parent).scroll_top(), Some(50));
    // Clamped — max_y = 200 - 20 = 180.
    assert!(dom.node(parent).scroll_top().unwrap() <= 180);
}

#[test]
fn click_toggles_checkbox_via_runtime_listener() {
    // The runtime's toggle module listens for `click` on root +
    // toggles the checkbox `[checked]` attribute. Verifying
    // `click()` runs the full activation chain end-to-end.
    let mut dom: TuiDom = TuiDom::new();
    crate::runtime::builtins::toggle::install(&mut dom);
    let root = dom.root();
    let cb = dom.create_element("input");
    dom.set_attribute(cb, "type", "checkbox").unwrap();
    dom.append_child(root, cb).unwrap();

    assert!(!dom.node(cb).has_attribute("checked"));
    dom.node_mut(cb).click();
    assert!(dom.node(cb).has_attribute("checked"));
    dom.node_mut(cb).click();
    assert!(!dom.node(cb).has_attribute("checked"));
}
