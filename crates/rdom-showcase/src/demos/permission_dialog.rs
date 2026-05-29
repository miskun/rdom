//! Permission dialog — a system-notification-style modal that
//! asks the user to allow or deny an action with a "once" or
//! "always" duration.
//!
//! Composes three rdom-tui built-ins into a recognizable pattern:
//!
//! - `<dialog open>` provides the modal chrome (UA: rounded
//!   accent-color border + 1×2 padding).
//! - `<input type="radio" name="action">` forms a single-tab-stop
//!   radio group with arrow-key navigation, supplied by the
//!   `runtime::builtins::toggle` default actions.
//! - `<button>` for the Apply / Dismiss row, with `margin-left:
//!   auto` on Apply to push both buttons against the dialog's
//!   right edge (rdom doesn't ship `justify-content` yet — auto
//!   margins are the canonical CSS workaround).
//!
//! Apply reads the checked radio's `value` and writes a "result"
//! line below the dialog. Dismiss removes the `open` attribute,
//! collapsing the dialog out of layout (UA rule:
//! `dialog:not([open]) { display: none }`).
//!
//! ## Substrate workarounds documented in TECH_DEBT.md
//!
//! Building this demo surfaced two flex-intrinsic-sizing gaps.
//! The CSS below works around each one cleanly:
//!
//! - `FLEX-BLOCK-MAIN-INTRINSIC-1` — `<input type=radio>` is UA
//!   `display: Block`, and Block items in a flex row oversize past
//!   their intrinsic content width. Worked around by setting
//!   `.choice input[type=radio] { width: 4 }` — the radio's
//!   `( ) ` / `(•) ` pseudo content is exactly 4 cells, so the
//!   explicit width matches reality.
//! - `FLEX-ITEM-MARGIN-MAIN-INTRINSIC-1` — `margin-top` /
//!   `margin-bottom` on a flex item doesn't count toward the flex
//!   container's intrinsic main-axis size, so a dialog full of
//!   children with `margin-bottom` would shrink and clip the last
//!   child into its bottom border. Worked around by putting the
//!   inter-section spacing on the dialog itself via `gap: 1` and
//!   wrapping the speaker + URL in a `.message` container so the
//!   gap rule doesn't insert a row between *them* (they're block
//!   siblings inside `.message`, not flex children of the dialog).

use std::io;

use rdom_tui::{App, ListenerOptions, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="permission-dialog-demo">
  <dialog open>
    <div class="message">
      <p class="speaker">hermes wants to:</p>
      <p class="url"><code>POST https://api.linear.app/graphql</code></p>
    </div>
    <div class="choices">
      <div class="choice">
        <input type="radio" id="allow-once" name="action" value="allow-once" checked autofocus>
        <label for="allow-once">Allow once</label>
      </div>
      <div class="choice">
        <input type="radio" id="allow-always" name="action" value="allow-always">
        <label for="allow-always">Allow always</label>
      </div>
      <div class="choice">
        <input type="radio" id="deny-once" name="action" value="deny-once">
        <label for="deny-once">Deny once</label>
      </div>
      <div class="choice">
        <input type="radio" id="deny-always" name="action" value="deny-always">
        <label for="deny-always">Deny always</label>
      </div>
    </div>
    <div class="actions">
      <button class="apply">Apply</button>
      <button class="dismiss">Dismiss</button>
    </div>
  </dialog>
  <p class="result">(no choice yet)</p>
</div>"#;

pub const CSS: &str = r#"
.permission-dialog-demo {
  padding: 0 1;
  display: flex;
  flex-direction: column;
  gap: 1;
}
.permission-dialog-demo dialog {
  display: flex;
  flex-direction: column;
  /* Override the UA's `Padding(1, 2, 1, 2)` on `<dialog>` —
     drop the vertical pad so the speaker line sits flush
     against the top border. Horizontal stays at 1 so text
     doesn't crowd the side borders.

     No `gap` — a uniform gap would put a row between
     `.choices` and `.actions` too, which we don't want. The
     single inter-section row above the radios is supplied by
     `.choices { padding-top: 1 }` below; `.actions` butts up
     against `.choices` with no space between. */
  padding: 0 1;
  /* Subtle near-black surface that distinguishes the dialog
     from the terminal default. The Apply button's half-block
     border has `background-clip: padding-box`-equivalent
     behavior (see DIVERGENCES.md) — the dialog bg shows
     through the empty half of each half-block glyph, joining
     the pill silhouette into a continuous accent shape. */
  background-color: #1f2123;
  /* Override the UA's accent border-color with a neutral gray
     one shade lighter than the dialog bg — frames the dialog
     without screaming for attention. */
  border-color: #2d2f31;
}
.permission-dialog-demo .speaker {
  font-weight: bold;
}
/* Suppress the UA's `<code>` field-tint bg (`FIELD_BG` =
   `#1f2123`) — it now matches the dialog bg by coincidence, but
   the override keeps the URL line readable even if the dialog
   palette shifts later. */
.permission-dialog-demo .url code {
  background-color: transparent;
}
.permission-dialog-demo .choices {
  display: flex;
  flex-direction: column;
  /* Single row of breathing space between the message block
     above and the radios. Pads the .choices box itself rather
     than relying on the dialog's `gap`, because we want NO
     space between .choices and .actions. */
  padding-top: 1;
}
.permission-dialog-demo .choice {
  display: flex;
  flex-direction: row;
  height: 1;
  /* Anchor for the absolutely-positioned `::before` / `::after`
     half-block extensions below — they pin to `left: -1` /
     `right: -1` relative to this `.choice` rect to spill into
     the dialog's 1-cell horizontal padding when the row is
     focused. */
  position: relative;
}
.permission-dialog-demo .choice input[type=radio] {
  width: 2;
}
/* `○` (U+25CB WHITE CIRCLE) + `◉` (U+25C9 FISHEYE) — both share
   the same outer ring silhouette, so the unchecked/checked pair
   reads as "same shape, filled in" (the classic radio metaphor).
   The rdom UA defaults to `( ) ` / `(•) ` — pleasant in pure-ASCII
   environments but visually heavier and breaks the
   matched-silhouette read. Author rules override at higher
   specificity (1 class + 1 class + 1 type + 1 attribute +
   pseudo-element) against the UA's bare
   `input[type=radio]::before` and `:checked::before`. The checked
   glyph also picks up the dialog's accent color (#3d90ce) so
   the selected option pops against the unchecked siblings. */
.permission-dialog-demo .choice input[type=radio]::before {
  content: "○ ";
}
.permission-dialog-demo .choice input[type=radio]:checked::before {
  content: "◉ ";
  color: #3d90ce;
}
/* Suppress the UA `:focus` bg on the radio itself — by default it
   paints a dark gray block over the 2-cell input box, which reads
   as a colored blob behind the glyph instead of a focus indicator.
   The whole-row indicator below (via `:focus-within` on the label)
   takes its place. */
.permission-dialog-demo .choice input[type=radio]:focus {
  background-color: transparent !important;
}
/* `:focus-within` matches the .choice label whenever its wrapped
   radio is focused — so the focus highlight lands on the entire
   row, not just the input's glyph cells. */
.permission-dialog-demo .choice:focus-within {
  background-color: rgb(45, 47, 49);
}
/* Half-block extensions: when a `.choice` is focused, paint a
   right-half-block (`▐`) one cell to the LEFT of the row and a
   left-half-block (`▌`) one cell to the RIGHT — those cells sit
   inside the dialog's 1-cell horizontal padding, so the focus
   highlight visually spills into the padding zone with a soft
   pill edge instead of stopping abruptly at the .choice rect.
   Each half-block paints its inner half in the focus color and
   leaves the outer half showing the dialog bg.

   Anchored via `position: absolute` (the `.choice` rule above
   sets `position: relative` so these pseudos use `.choice` as
   their containing block). Negative `left` / `right` offsets put
   them just outside the row's normal layout rect. */
.permission-dialog-demo .choice:focus-within::before {
  content: "▐";
  position: absolute;
  left: -1;
  color: rgb(45, 47, 49);
}
.permission-dialog-demo .choice:focus-within::after {
  content: "▌";
  position: absolute;
  right: -1;
  color: rgb(45, 47, 49);
}
.permission-dialog-demo .actions {
  display: flex;
  flex-direction: row;
  /* Shift the row 1 cell to the right so the buttons land
     flush against the dialog's right padding/border edge
     instead of leaving a visible gap. `position: relative` +
     `left: 1` is the cleanest way — `margin-right: -1` doesn't
     compose with the default `align-items: stretch` on the
     dialog's cross-axis. */
  position: relative;
  left: 1;
}
.permission-dialog-demo .actions .apply {
  margin-left: auto;
}
/* Box-bordered buttons in place of the UA's bracketed-glyph
   `[ Label ]` chrome. Suppress the UA `::before` / `::after`
   brackets (override their `content` to the empty string) and
   ship a `border: rounded` ring — solid lines with rounded
   `╭ ╮ ╰ ╯` corners matching the dialog's own border style.
   With no gap between actions, adjacent buttons render as
   `╮╭` (two cells, no collapse — `border-collapse` would merge
   them into `┬`/`├` glyphs which isn't the look we want). */
.permission-dialog-demo .actions button {
  border: rounded;
  padding: 0 1;
}
.permission-dialog-demo .actions button::before,
.permission-dialog-demo .actions button::after {
  content: "";
}
/* Apply is the primary CTA. Filled rectangle visually outweighs
   the outlined Dismiss next to it when every cell is colored —
   so the button uses `border: half-block`, a rdom-specific
   border style that paints `▄ ▀ ▌ ▐ ▗ ▖ ▝ ▘` glyphs (U+258x /
   U+259x) sized to half a cell each. The colored ring spans
   ~2 cells of visible vertical color across the 3-row layout,
   so the button reads as a 2-cell-tall pill — same visual
   weight as Dismiss's thin outline, without giving up the
   filled-CTA look. `border-color` AND `background-color` both
   set to #3d90ce so the ring and the interior share the
   accent color; the join phase clears bg on the border cells
   themselves so the half-block silhouette stays visible against
   the surrounding parent bg (see border_join's HalfBlock branch
   + the DIVERGENCES.md entry — closest analog to CSS's
   `background-clip: padding-box`). `color: white` makes the
   label legible against the accent-colored interior. */
.permission-dialog-demo .actions .apply {
  border: half-block;
  border-color: #3d90ce;
  background-color: #3d90ce;
  color: white;
  padding: 0 1;
}
/* Dismiss mirrors Apply's filled-pill shape (`border: half-block`
   ring + filled interior + white text) but in a near-black
   `#2d2f31` instead of the accent. The low-contrast surface
   reads as "present but secondary" against the `#181a1c`
   dialog bg — visually distinguishable without competing with
   Apply for the eye. Same half-block bg-clip behavior applies:
   the dialog bg shows through the empty half of each
   half-block glyph, joining the pill silhouette continuously. */
.permission-dialog-demo .actions .dismiss {
  border: half-block;
  border-color: #2d2f31;
  background-color: #2d2f31;
  color: white;
}
.permission-dialog-demo .result {
  color: rgb(140, 150, 170);
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "permission-dialog-demo")
        .unwrap();

    let dialog = dom.create_element("dialog");
    dom.set_attribute(dialog, "open", "").unwrap();
    dom.append_child(root, dialog).unwrap();

    // Message block — speaker + URL stacked tight (Block siblings,
    // no gap). Wrapped in its own container so the dialog's
    // `gap: 1` doesn't insert a row between speaker and URL.
    let message = dom.create_element("div");
    dom.set_attribute(message, "class", "message").unwrap();
    dom.append_child(dialog, message).unwrap();
    append_p(dom, message, "speaker", "hermes wants to:");
    let url_p = dom.create_element("p");
    dom.set_attribute(url_p, "class", "url").unwrap();
    let url_code = dom.create_element("code");
    let url_text = dom.create_text_node("POST https://api.linear.app/graphql");
    dom.append_child(url_code, url_text).unwrap();
    dom.append_child(url_p, url_code).unwrap();
    dom.append_child(message, url_p).unwrap();

    // Radio group — four choices, `Allow once` checked initially.
    // Explicit `for`/`id` association (input + label as siblings)
    // rather than wrapping; matches the canonical web-developer
    // idiom and lets `.choice` be a plain `<div>` flex container,
    // independent of the label element.
    let choices = dom.create_element("div");
    dom.set_attribute(choices, "class", "choices").unwrap();
    let radios: Vec<NodeId> = [
        ("allow-once", "Allow once", true),
        ("allow-always", "Allow always", false),
        ("deny-once", "Deny once", false),
        ("deny-always", "Deny always", false),
    ]
    .into_iter()
    .enumerate()
    .map(|(idx, (value, label_text, checked))| {
        let row = dom.create_element("div");
        dom.set_attribute(row, "class", "choice").unwrap();

        let input = dom.create_element("input");
        dom.set_attribute(input, "type", "radio").unwrap();
        dom.set_attribute(input, "id", value).unwrap();
        dom.set_attribute(input, "name", "action").unwrap();
        dom.set_attribute(input, "value", value).unwrap();
        if checked {
            dom.set_attribute(input, "checked", "").unwrap();
        }
        // First radio carries `autofocus` so the showcase's
        // `mount_demo` (which runs `focus_within` on the demo
        // subtree, see crates/rdom-showcase/src/nav.rs) drops
        // initial focus inside the dialog instead of leaving
        // it on the sidebar `<li>`. Tab from here moves to the
        // Apply button (the radio group is a single tab stop
        // per HTML — arrow keys navigate within the group).
        if idx == 0 {
            dom.set_attribute(input, "autofocus", "").unwrap();
        }
        dom.append_child(row, input).unwrap();

        let label = dom.create_element("label");
        dom.set_attribute(label, "for", value).unwrap();
        let t = dom.create_text_node(label_text);
        dom.append_child(label, t).unwrap();
        dom.append_child(row, label).unwrap();

        dom.append_child(choices, row).unwrap();
        input
    })
    .collect();
    dom.append_child(dialog, choices).unwrap();

    // Action row.
    let actions = dom.create_element("div");
    dom.set_attribute(actions, "class", "actions").unwrap();

    let apply_btn = dom.create_element("button");
    dom.set_attribute(apply_btn, "class", "apply").unwrap();
    let apply_text = dom.create_text_node("Apply");
    dom.append_child(apply_btn, apply_text).unwrap();
    dom.append_child(actions, apply_btn).unwrap();

    let dismiss_btn = dom.create_element("button");
    dom.set_attribute(dismiss_btn, "class", "dismiss").unwrap();
    let dismiss_text = dom.create_text_node("Dismiss");
    dom.append_child(dismiss_btn, dismiss_text).unwrap();
    dom.append_child(actions, dismiss_btn).unwrap();

    dom.append_child(dialog, actions).unwrap();

    // Result line — sibling of the dialog so it stays visible after
    // Dismiss collapses the dialog.
    let result = dom.create_element("p");
    dom.set_attribute(result, "class", "result").unwrap();
    let result_text = dom.create_text_node("(no choice yet)");
    dom.append_child(result, result_text).unwrap();
    dom.append_child(root, result).unwrap();

    // Apply: find the checked radio and write its value to the
    // result line.
    let radios_for_apply = radios.clone();
    dom.add_event_listener(apply_btn, "click", ListenerOptions::default(), move |ctx| {
        let chosen = radios_for_apply
            .iter()
            .find(|&&r| ctx.dom.node(r).has_attribute("checked"))
            .and_then(|&r| {
                ctx.dom
                    .node(r)
                    .get_attribute("value")
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "(none)".to_string());
        let _ = ctx
            .dom
            .node_mut(result_text)
            .set_node_value(&format!("applied: {chosen}"));
    })
    .unwrap();

    // Dismiss: collapse the dialog (UA rule:
    // `dialog:not([open]) { display: none }`) and note the dismiss.
    dom.add_event_listener(
        dismiss_btn,
        "click",
        ListenerOptions::default(),
        move |ctx| {
            let _ = ctx.dom.remove_attribute(dialog, "open");
            let _ = ctx.dom.node_mut(result_text).set_node_value("dismissed");
        },
    )
    .unwrap();

    root
}

pub fn stylesheet() -> Stylesheet {
    rdom_css::from_css(CSS)
}

pub fn run_standalone() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = build(&mut dom);
    dom.append_child(root, demo_root).unwrap();
    App::new(dom, stylesheet())?.run()
}

pub struct PermissionDialog;

impl Demo for PermissionDialog {
    fn slug(&self) -> &'static str {
        "forms/permission-dialog"
    }

    fn title(&self) -> &'static str {
        "Permission dialog"
    }

    fn category(&self) -> Category {
        Category::Forms
    }

    fn build(&self, dom: &mut TuiDom) -> NodeId {
        build(dom)
    }

    fn stylesheet(&self) -> Stylesheet {
        stylesheet()
    }

    fn source(&self) -> Source {
        Source {
            markup: MARKUP,
            css: CSS,
        }
    }
}

fn append_p(dom: &mut TuiDom, parent: NodeId, class: &str, text: &str) {
    let p = dom.create_element("p");
    dom.set_attribute(p, "class", class).unwrap();
    let t = dom.create_text_node(text);
    dom.append_child(p, t).unwrap();
    dom.append_child(parent, p).unwrap();
}
