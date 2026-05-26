//! Chrome layout contract — asserts that the showcase shell's
//! layout matches the *intent* of its CSS, not just a pixel-
//! snapshot of what it rendered last time.
//!
//! Motivation (grumpy-architect review, 2026-05-26): the snapshot
//! tests under `crates/rdom-tui/tests/snapshots/` froze the
//! showcase's rendered output. When the chrome had latent layout
//! bugs (e.g. `view-content`'s `flex: 1` losing its slice of the
//! main-axis budget to a sibling with `display: none` children
//! inflating its intrinsic), the snapshots captured the broken
//! layout as expected. Regressing to the *correct* layout would
//! fail CI. This file pins design intent in assertions so the
//! snapshot review process has a contract-level check above it.
//!
//! Add a new test here whenever a chrome bug surfaces — encode
//! "here is what the CSS says, here is what the layout pass must
//! produce" so the next bug of the same shape fails loudly.

use rdom_showcase::shell::base_stylesheet;
use rdom_showcase::{DEMOS, ShowcaseState, build_shell, mount_demo};
use rdom_tui::prelude::*;
use rdom_tui::render::Buffer;
use rdom_tui::{Color, NodeId, NodeType, TuiDom};

/// Build the shell + mount the first demo + cascade + layout at
/// `width × height`. Returns the dom and the handles so tests can
/// inspect layout rects.
fn shell_at(width: u16, height: u16) -> (TuiDom, rdom_showcase::ShellHandles) {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);
    mount_demo(&mut state, &mut dom, 0); // Hello World

    let base = base_stylesheet();
    let mut sheets = vec![base];
    for d in DEMOS {
        sheets.push(d.stylesheet());
    }
    let refs: Vec<&_> = sheets.iter().collect();
    dom.cascade_all(&refs);
    dom.layout_dom(Rect::new(0, 0, width, height));
    (dom, handles)
}

fn find_by_class(dom: &TuiDom, id: NodeId, class: &str) -> Option<NodeId> {
    let n = dom.node(id);
    if n.node_type() == NodeType::Element
        && n.get_attribute("class")
            .map(|s| s.split_whitespace().any(|c| c == class))
            .unwrap_or(false)
    {
        return Some(id);
    }
    for c in n.child_nodes() {
        if let Some(f) = find_by_class(dom, c.id(), class) {
            return Some(f);
        }
    }
    None
}

#[test]
fn view_content_flex_grows_to_fill_main_with_source_at_bottom() {
    // Design intent: `view-content { flex: 1 }` should grab the
    // remaining main-axis space inside `<main>` (which is a flex
    // column). `source-disclosure` (Auto height, max-height: 16)
    // should sit at the bottom and shrink to its intrinsic when
    // closed.
    //
    // Bug this catches: when a sibling's intrinsic was inflated
    // by hidden children, `view-content`'s `flex: 1` was starved.
    //
    // Phase 1a removed the in-`<main>` scroll-indicator; the
    // status bar lives outside `.app` now, so view-content's flex
    // budget no longer competes with it.
    let (dom, _h) = shell_at(80, 24);
    let main = find_by_class(&dom, dom.root(), "main").unwrap();
    let vc = find_by_class(&dom, dom.root(), "view-content").unwrap();
    let src = find_by_class(&dom, dom.root(), "source-disclosure").unwrap();

    let main_rect = dom.node(main).ext().unwrap().layout;
    let vc_rect = dom.node(vc).ext().unwrap().layout;
    let src_rect = dom.node(src).ext().unwrap().layout;

    // source-disclosure is CLOSED. Its content is the summary
    // (one row of text) PLUS its 1-row `border-top: solid`. The
    // border-top longhand now applies (per-side `border-*`
    // longhand parser landed) so outer height is 2.
    assert_eq!(
        src_rect.height, 2,
        "closed source-disclosure = 1 row summary + 1 row border-top"
    );

    // view-content should fill the rest of <main>'s content area.
    // main's inner area = main.height - border (2). Source-
    // disclosure is the only other child now.
    let main_inner_h = main_rect.height - 2; // app's chrome border collapses with main's
    let expected_vc_h = main_inner_h - src_rect.height;
    assert!(
        vc_rect.height >= expected_vc_h - 1 && vc_rect.height <= expected_vc_h + 1,
        "view-content should stretch via flex:1 to ~{expected_vc_h} rows, got {} \
         (likely cause: an Auto-sized sibling's intrinsic is inflated by hidden children, \
         eating from the flex budget — see Phase 9 grumpy review's intrinsic_size fix)",
        vc_rect.height
    );

    // Source sits BELOW view-content (within ~1 row tolerance for
    // padding-and-border math).
    assert!(
        src_rect.y >= vc_rect.y + vc_rect.height as i32 - 1
            && src_rect.y <= vc_rect.y + vc_rect.height as i32 + 1,
        "source-disclosure should sit at view-content's bottom: vc.bottom={}, src.y={}",
        vc_rect.y + vc_rect.height as i32,
        src_rect.y
    );
}

#[test]
fn source_disclosure_when_open_has_fixed_height_12() {
    // Phase 2 design intent: when the source disclosure is OPEN,
    // it occupies a fixed 12 outer rows (border-top + 11-row
    // content area). `overflow: auto` lets longer source scroll
    // inside that fixed-size panel rather than pushing chrome
    // around. Predictable demo-vs-source split — opening source
    // for any demo always takes the same 12 rows.
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);
    mount_demo(&mut state, &mut dom, 0); // Hello World

    // Open the disclosure (substrate's UA cascade gates content
    // visibility on `[open]`).
    dom.set_attribute(handles.source_disclosure, "open", "")
        .unwrap();

    let base = base_stylesheet();
    let mut sheets = vec![base];
    for d in DEMOS {
        sheets.push(d.stylesheet());
    }
    let refs: Vec<&_> = sheets.iter().collect();
    dom.cascade_all(&refs);
    dom.layout_dom(Rect::new(0, 0, 80, 30));

    let src_rect = dom.node(handles.source_disclosure).ext().unwrap().layout;
    assert_eq!(
        src_rect.height, 12,
        "open source-disclosure should take fixed 12 outer rows (got {})",
        src_rect.height
    );

    // Content area: under `border-collapse: collapse` on `.app`,
    // M5.5b says a child's content area expands to include its
    // border-ring cells (the border shares with the previous
    // sibling's bottom edge). So content = full outer = 12.
    let src_content = dom
        .node(handles.source_disclosure)
        .ext()
        .unwrap()
        .content_layout;
    assert_eq!(
        src_content.height, 12,
        "border-collapse: collapse → content area includes the border-ring cell"
    );

    // `overflow: auto` means a long source scrolls inside this
    // 11-row content area. Check both axes are scrollable
    // (CSS `overflow: auto` shorthand sets both x and y).
    let (ox, oy) = dom
        .node(handles.source_disclosure)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| (c.overflow_x, c.overflow_y))
        .unwrap();
    let scroll = |o: rdom_tui::layout::Overflow| {
        matches!(
            o,
            rdom_tui::layout::Overflow::Auto | rdom_tui::layout::Overflow::Scroll
        )
    };
    assert!(
        scroll(ox) && scroll(oy),
        "open source-disclosure should be scrollable on both axes (got x={ox:?}, y={oy:?})"
    );
}

#[test]
fn source_disclosure_when_closed_shows_only_summary() {
    // Design intent: the UA `details:not([open]) > *:not(summary)`
    // rule hides every child of `<details>` except `<summary>` when
    // the disclosure is closed. Their layout rects must not exist
    // (or be zero-sized), and they must NOT contribute to the
    // disclosure's height.
    let (dom, _h) = shell_at(80, 24);
    let src = find_by_class(&dom, dom.root(), "source-disclosure").unwrap();
    let src_rect = dom.node(src).ext().unwrap().layout;
    assert_eq!(
        src_rect.height, 2,
        "closed disclosure: 1 row summary + 1 row border-top"
    );

    // Check that hidden children have zero-area layout rects.
    let n = dom.node(src);
    for child in n.child_nodes() {
        if child.tag_name() == Some("summary") {
            continue;
        }
        let r = child.ext().map(|e| e.layout).unwrap_or_default();
        assert!(
            r.width == 0 || r.height == 0,
            "hidden child {:?} should have zero area, got {r:?}",
            child.tag_name()
        );
    }
}

#[test]
fn source_disclosure_block_children_share_content_box_left() {
    // Bug: <h3>Markup</h3> labels added by `rebuild_source_disclosure`
    // rendered flush against the right panel's left border, while the
    // sibling <pre> code body was indented. Caused by per-child
    // padding rules on <summary> and <pre> with nothing on <h3>
    // (UA-default <h3> has no padding).
    //
    // Contract: the source-disclosure container provides horizontal
    // padding for its block children, so <summary>, <h3>, and <pre>
    // all share the same content-box left edge. This is the web-
    // idiom layout — container-padding inherits to every block child
    // without each one needing its own rule, so new block children
    // (a future "Show diff" button, an explanation <p>, etc.) auto-
    // align without a per-element rule.
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);
    mount_demo(&mut state, &mut dom, 0); // Hello World

    // Open the disclosure so h3/pre are laid out.
    dom.set_attribute(handles.source_disclosure, "open", "")
        .unwrap();

    let base = base_stylesheet();
    let mut sheets = vec![base];
    for d in DEMOS {
        sheets.push(d.stylesheet());
    }
    let refs: Vec<&_> = sheets.iter().collect();
    dom.cascade_all(&refs);
    dom.layout_dom(Rect::new(0, 0, 123, 26));

    // Find the first <summary>, <h3>, and <pre> children of the
    // source-disclosure.
    let n = dom.node(handles.source_disclosure);
    let mut summary: Option<NodeId> = None;
    let mut h3: Option<NodeId> = None;
    let mut pre: Option<NodeId> = None;
    for child in n.child_nodes() {
        match child.tag_name() {
            Some("summary") if summary.is_none() => summary = Some(child.id()),
            Some("h3") if h3.is_none() => h3 = Some(child.id()),
            Some("pre") if pre.is_none() => pre = Some(child.id()),
            _ => {}
        }
    }
    let summary = summary.expect("<summary> child");
    let h3 = h3.expect("<h3> label child");
    let pre = pre.expect("<pre> code child");

    let summary_x = dom.node(summary).ext().unwrap().content_layout.x;
    let h3_x = dom.node(h3).ext().unwrap().content_layout.x;
    let pre_x = dom.node(pre).ext().unwrap().content_layout.x;

    assert_eq!(
        h3_x, pre_x,
        "<h3> label and <pre> code must share content-box left edge \
         (h3.x={h3_x}, pre.x={pre_x}) — move padding onto the \
         .source-disclosure container instead of per-child rules"
    );
    assert_eq!(
        h3_x, summary_x,
        "<h3> label and <summary> disclosure must share content-box \
         left edge (h3.x={h3_x}, summary.x={summary_x})"
    );
}

#[test]
fn first_demos_subtree_attaches_under_view_content_not_main() {
    // Design intent: mount_demo appends the demo's subtree to the
    // `.view-content` <div>, not directly to <main>. The demo's
    // CSS is class-scoped (every selector references the demo's
    // root class), so it'd be inert if mounted at the wrong level.
    let (dom, handles) = shell_at(80, 24);
    let vc = find_by_class(&dom, dom.root(), "view-content").unwrap();
    assert_eq!(
        vc, handles.main,
        "ShellHandles.main is the view-content mount point"
    );
    // Demo's root has class="hello" — sits directly under view-content.
    let hello = find_by_class(&dom, dom.root(), "hello").unwrap();
    let hello_parent = dom.node(hello).parent_node().unwrap();
    assert_eq!(hello_parent.id(), vc);
}

#[test]
fn shell_paints_no_overlap_between_demo_and_source_at_default_viewport() {
    // Design intent at 80x24: demo content occupies the top of
    // view-content, source-disclosure sits at the bottom of <main>,
    // and there's NO row at which the demo and source overlap.
    let (dom, _h) = shell_at(80, 24);
    let hello = find_by_class(&dom, dom.root(), "hello").unwrap();
    let src = find_by_class(&dom, dom.root(), "source-disclosure").unwrap();
    let hello_rect = dom.node(hello).ext().unwrap().layout;
    let src_rect = dom.node(src).ext().unwrap().layout;
    let hello_bottom = hello_rect.y + hello_rect.height as i32;
    assert!(
        hello_bottom <= src_rect.y,
        "demo content (`.hello`) bottom ({hello_bottom}) overlaps source-disclosure top ({}) \
         — either view-content is shrink-wrapped instead of flex-stretched, or the demo's intrinsic \
         is wrong",
        src_rect.y
    );
}

/// Paint the shell at `width × height` and return both the buffer
/// and the handles for rect lookups. Mirrors `shell_at` but adds the
/// paint step so cell-level assertions can run.
fn paint_shell(width: u16, height: u16) -> (TuiDom, rdom_showcase::ShellHandles, Buffer) {
    let (dom, handles) = shell_at(width, height);
    let viewport = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);
    (dom, handles, buf)
}

/// Extract a row of cell symbols within `[x_start, x_end)` at row
/// `y`, joining spacer cells the same way `buffer_to_snapshot` does
/// (skip them so wide glyphs read as a single character).
fn row_text(buf: &Buffer, y: u16, x_start: u16, x_end: u16) -> String {
    let mut s = String::new();
    for x in x_start..x_end {
        if let Some(c) = buf.cell(x, y) {
            if c.is_spacer() {
                continue;
            }
            s.push_str(c.symbol());
        }
    }
    s
}

#[test]
fn hello_world_demo_renders_per_css_contract() {
    // Cell-level contract for the Hello World demo as it sits inside
    // the showcase chrome. Pins the cascade + layout + paint pipeline
    // against the demo's literal CSS:
    //
    //   <div class="hello">
    //     <h1>Hello, rdom!</h1>
    //     <p>If you can read this in a terminal, the showcase shell is mounted.</p>
    //   </div>
    //
    //   .hello { padding: 1 }
    //   .hello h1 { color: rgb(180, 220, 255); font-weight: bold }
    //
    // UA defaults: <h1> and <p> are block + (h1 is) bold. No UA
    // margins on either (rdom doesn't ship browser-style heading
    // margins — see DIVERGENCES.md if/when added). So h1 and p
    // should appear on adjacent rows with no gap.
    let (dom, _h, buf) = paint_shell(80, 24);

    // ── Layout contract ─────────────────────────────────────────────
    let hello = find_by_class(&dom, dom.root(), "hello").expect("`.hello` mounts");
    let hello_rect = dom.node(hello).ext().unwrap().layout;
    let hello_content = dom.node(hello).ext().unwrap().content_layout;

    // `.hello { padding: 1 }` → content area is inset 1 cell on all
    // sides relative to the outer layout rect.
    assert_eq!(
        hello_content.x,
        hello_rect.x + 1,
        "padding: 1 → content x is outer.x + 1"
    );
    assert_eq!(
        hello_content.y,
        hello_rect.y + 1,
        "padding: 1 → content y is outer.y + 1"
    );
    assert_eq!(
        hello_content.width + 2,
        hello_rect.width,
        "padding: 1 each side → content width is outer.width - 2"
    );

    // Children layout: h1 sits at .hello's content top (no UA top
    // margin in rdom); p sits immediately below h1 (no UA margins
    // between siblings).
    let h1 = dom
        .node(hello)
        .child_nodes()
        .find(|n| n.tag_name() == Some("h1"))
        .expect("h1 child")
        .id();
    let p = dom
        .node(hello)
        .child_nodes()
        .find(|n| n.tag_name() == Some("p"))
        .expect("p child")
        .id();
    let h1_rect = dom.node(h1).ext().unwrap().layout;
    let p_rect = dom.node(p).ext().unwrap().layout;

    assert_eq!(
        h1_rect.y, hello_content.y,
        "h1 sits at .hello content top (no UA top margin)"
    );
    assert_eq!(
        p_rect.y,
        h1_rect.y + h1_rect.height as i32,
        "p sits immediately below h1 (no UA margin between siblings)"
    );

    // ── Cell-level contract ─────────────────────────────────────────
    // h1's "Hello, rdom!" should land on its first row at the h1's
    // x origin. (h1 inherits its parent's content area, so it starts
    // at the same x as `.hello`'s content.)
    let h1_y = h1_rect.y as u16;
    let h1_x = h1_rect.x as u16;
    let line1 = row_text(&buf, h1_y, h1_x, h1_x + 12);
    assert_eq!(line1, "Hello, rdom!", "h1 paints its text at its origin");

    // h1's color: bold + rgb(180, 220, 255) on every glyph cell.
    let expected_fg = Color::Rgb(180, 220, 255);
    for x in h1_x..h1_x + 12 {
        let cell = buf.cell(x, h1_y).expect("cell in h1 row exists");
        assert_eq!(
            cell.fg, expected_fg,
            "h1 cell at x={x} should be rgb(180, 220, 255); got {:?}",
            cell.fg
        );
        assert!(
            cell.modifier.contains(rdom_tui::Modifier::BOLD),
            "h1 cell at x={x} should be bold; modifier={:?}",
            cell.modifier
        );
    }

    // p's text: "If you can read this in a terminal, the showcase
    // shell is mounted." (67 chars). At the chrome's narrow view-
    // content width, this wraps. Read line 1 from the p's origin
    // out to the right edge of .hello's content area, and verify
    // the prefix matches what the wrap point allows.
    let p_y = p_rect.y as u16;
    let p_x = p_rect.x as u16;
    let p_w = hello_content.width;
    let p_line1 = row_text(&buf, p_y, p_x, p_x + p_w);

    let full = "If you can read this in a terminal, the showcase shell is mounted.";
    assert!(
        full.starts_with(p_line1.trim_end()),
        "p first line ({p_line1:?}) should be a prefix of {full:?}"
    );

    // p is NOT bold and uses the inherited fg (not the h1 blue).
    if !p_line1.trim().is_empty() {
        let probe = buf.cell(p_x, p_y).expect("p first cell exists");
        assert_ne!(
            probe.fg, expected_fg,
            "p should not inherit h1's color (selector is .hello h1)"
        );
        assert!(
            !probe.modifier.contains(rdom_tui::Modifier::BOLD),
            "p should not be bold; modifier={:?}",
            probe.modifier
        );
    }

    // Padding-top contract: the row immediately ABOVE h1's first
    // row, INSIDE .hello's outer rect, must be blank — that's the
    // 1-cell padding-top. Check the x range of .hello's content
    // area on the row just above h1.
    if hello_rect.y < h1_rect.y {
        let pad_y = (hello_rect.y) as u16;
        let pad_text = row_text(
            &buf,
            pad_y,
            hello_content.x as u16,
            (hello_content.x + hello_content.width as i32) as u16,
        );
        assert!(
            pad_text.trim().is_empty(),
            "padding-top row at y={pad_y} should be blank in .hello's x range, got {pad_text:?}"
        );
    }
}

/// Concatenate every text-node descendant of `id` in document
/// order — like the DOM `textContent` accessor.
fn text_content(dom: &TuiDom, id: NodeId) -> String {
    let mut s = String::new();
    fn walk(dom: &TuiDom, id: NodeId, out: &mut String) {
        let n = dom.node(id);
        if n.node_type() == NodeType::Text {
            out.push_str(&n.text_content());
            return;
        }
        for c in n.child_nodes() {
            walk(dom, c.id(), out);
        }
    }
    walk(dom, id, &mut s);
    s
}

#[test]
fn status_bar_shows_default_keyboard_hints_after_build() {
    // Phase 1b design intent: on boot, the status bar shows the
    // global keyboard hints (sidebar nav is autofocused, so the
    // hints applicable there double as the global default). The
    // bar is populated by `seed_status_bar_hints` during build,
    // BEFORE any focus event fires.
    let (dom, handles) = shell_at(80, 24);
    let txt = text_content(&dom, handles.status_bar);
    // The exact prose may evolve; pin the substantive shortcuts.
    assert!(
        txt.contains("↑↓"),
        "status bar should advertise the up/down keys; got {txt:?}"
    );
    assert!(
        txt.contains("navigate"),
        "status bar should label the up/down shortcut; got {txt:?}"
    );
    assert!(
        txt.contains("Enter"),
        "status bar should advertise the Enter key; got {txt:?}"
    );
    assert!(
        txt.contains("select"),
        "status bar should label the Enter shortcut; got {txt:?}"
    );
}

#[test]
fn status_bar_keys_render_with_distinct_style_from_labels() {
    // Phase 1b design intent: status bar uses `<span class="key">`
    // and `<span class="label">` so the cascade can style keys
    // (bold + brighter) differently from their action labels
    // (dimmer, regular weight). Probe specific glyph cells:
    // - "↑" — first cell of the key span ("↑↓")
    // - "n" — first cell of the label span ("navigate")
    let (dom, handles, buf) = paint_shell(80, 24);

    // First confirm the structure exists (helps fail loudly if the
    // markup drifts).
    find_by_class(&dom, handles.status_bar, "key")
        .expect("status bar should contain at least one .key span");
    find_by_class(&dom, handles.status_bar, "label")
        .expect("status bar should contain at least one .label span");

    // Find the painted row of the status bar via its layout rect.
    let status_rect = dom.node(handles.status_bar).ext().unwrap().layout;
    let y = status_rect.y as u16;

    // Scan the row left-to-right and pick the first cell that
    // paints "↑" (key glyph) and the first cell that paints "n"
    // (label glyph — `navigate`'s first letter).
    let mut key_cell = None;
    let mut label_cell = None;
    for x in 0..80 {
        let Some(cell) = buf.cell(x, y) else { continue };
        let sym = cell.symbol();
        if key_cell.is_none() && sym == "↑" {
            key_cell = Some(cell);
        }
        if label_cell.is_none() && sym == "n" {
            label_cell = Some(cell);
        }
    }
    let key_cell = key_cell.expect("`↑` glyph should paint somewhere in the status row");
    let label_cell =
        label_cell.expect("`n` of `navigate` should paint somewhere in the status row");

    assert!(
        key_cell.modifier.contains(rdom_tui::Modifier::BOLD),
        "key cells should be bold; got modifier {:?}",
        key_cell.modifier
    );
    assert!(
        !label_cell.modifier.contains(rdom_tui::Modifier::BOLD),
        "label cells should NOT be bold; got modifier {:?}",
        label_cell.modifier
    );
    assert_ne!(
        key_cell.fg, label_cell.fg,
        "key and label spans should resolve to different fg colors"
    );
}

#[test]
fn status_bar_is_sibling_of_app_and_sits_below_panel() {
    // Phase 1a design intent: the status bar is a separate concern
    // from the bordered `.app` panel. It lives OUTSIDE the panel
    // border (a sibling of `.app`, not a descendant), at the very
    // bottom of the viewport. This:
    //
    // 1. Removes the border-collapse trick that previously hid the
    //    scroll-indicator's content row under main's bottom border.
    // 2. Gives the status bar a real 1-row of its own to display
    //    keyboard shortcuts (Phase 1b).
    // 3. Mirrors the conventional terminal/IDE status-line pattern
    //    (vim, htop, VS Code's bottom bar): chrome above, status
    //    line below, both peer-level.
    let (dom, _h) = shell_at(80, 24);

    let app = find_by_class(&dom, dom.root(), "app").expect("`.app` mounts");
    let status = find_by_class(&dom, dom.root(), "status-bar")
        .expect("`.status-bar` mounts under root, not under `.app`");

    // Sibling check: both share the same parent (`.app-shell` or
    // the root fragment — the contract is "they're at the same
    // level," not "their parent has any particular class").
    let app_parent = dom.node(app).parent_node().map(|p| p.id());
    let status_parent = dom.node(status).parent_node().map(|p| p.id());
    assert_eq!(
        app_parent, status_parent,
        "status-bar must be a sibling of .app, not a descendant"
    );

    // Layout check: status-bar sits strictly below the panel.
    let app_rect = dom.node(app).ext().unwrap().layout;
    let status_rect = dom.node(status).ext().unwrap().layout;
    let app_bottom = app_rect.y + app_rect.height as i32;
    assert!(
        status_rect.y >= app_bottom,
        "status-bar (y={}) must sit at or below the panel bottom (={app_bottom})",
        status_rect.y
    );

    // Height: 1 row per chrome spec.
    assert_eq!(status_rect.height, 1, "status-bar should be 1 row tall");
}

#[test]
fn source_disclosure_has_border_top() {
    // Chrome CSS: `.main .source-disclosure { border-top: solid }`.
    // Per-side `border-*` longhand parser landed
    // (`BORDER-PER-SIDE-LONGHAND-1`); this asserts the cascaded
    // computed style has the top side enabled.
    let (dom, _h) = shell_at(80, 24);
    let src = find_by_class(&dom, dom.root(), "source-disclosure").unwrap();
    let border = dom
        .node(src)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| c.border)
        .unwrap_or_default();
    assert!(
        border.top,
        "border-top: solid should set border.top = true (got {border:?})"
    );
    assert!(!border.right, "border-top doesn't enable other sides");
    assert!(!border.bottom);
    assert!(!border.left);
}
