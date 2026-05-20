//! `impl TuiAccessorsMut for NodeMut` — write-side accessor methods
//! on `NodeMut<'a, TuiExt>`. Trait declaration lives in `super`
//! (mod.rs); private helpers live in `super::helpers`.

use rdom_core::NodeId;

use super::TuiAccessorsMut;
use super::helpers::{
    is_text_family_input, nearest_scrollable_ancestor, pre_scroll_offset_within, read_scroll_x,
    read_scroll_y, set_select_value, write_boolean_attribute, write_scroll_clamped,
};
use crate::node::install_text_content;
use crate::{Result, TuiExt};

impl<'a> TuiAccessorsMut<'a> for rdom_core::NodeMut<'a, TuiExt> {
    fn set_value(&mut self, value: impl Into<String>) -> Result<()> {
        let tag = match self.as_ref().tag_name() {
            Some(t) => t.to_string(),
            None => return Ok(()),
        };
        let new_value = value.into();
        let id = self.id();
        let dom = self.dom_mut();
        match tag.as_str() {
            "input" => {
                if is_text_family_input(dom, id) {
                    crate::runtime::builtins::input::set_value(dom, id, &new_value);
                } else {
                    dom.set_attribute(id, "value", &new_value)?;
                }
                Ok(())
            }
            "textarea" => install_text_content(dom, id, &new_value),
            "select" => set_select_value(dom, id, &new_value),
            _ => Ok(()),
        }
    }

    fn set_checked(&mut self, value: bool) -> Result<()> {
        if self.as_ref().tag_name() != Some("input") {
            return Ok(());
        }
        write_boolean_attribute(self, "checked", value)
    }

    fn set_indeterminate(&mut self, value: bool) -> Result<()> {
        if self.as_ref().tag_name() != Some("input") {
            return Ok(());
        }
        write_boolean_attribute(self, "indeterminate", value)
    }

    fn set_disabled(&mut self, value: bool) -> Result<()> {
        if !matches!(
            self.as_ref().tag_name(),
            Some("button" | "input" | "select" | "textarea" | "option" | "optgroup" | "fieldset")
        ) {
            return Ok(());
        }
        write_boolean_attribute(self, "disabled", value)
    }

    fn set_read_only(&mut self, value: bool) -> Result<()> {
        if !matches!(self.as_ref().tag_name(), Some("input" | "textarea")) {
            return Ok(());
        }
        write_boolean_attribute(self, "readonly", value)
    }

    fn set_inert(&mut self, value: bool) -> Result<()> {
        // `inert` is an HTMLElement-level attribute — applies to
        // every element with no tag gate.
        if self.as_ref().tag_name().is_none() {
            return Ok(());
        }
        write_boolean_attribute(self, "inert", value)
    }

    fn focus(&mut self) {
        let id = self.id();
        let dom = self.dom_mut();
        if !crate::runtime::focus::tabindex::is_focusable(dom, id) {
            return;
        }
        crate::runtime::focus::focus_node(dom, Some(id));
    }

    fn blur(&mut self) {
        let id = self.id();
        let dom = self.dom_mut();
        if dom.focused() != Some(id) {
            return;
        }
        crate::runtime::focus::focus_node(dom, None);
    }

    fn click(&mut self) {
        use rdom_core::{EventDetail, KeyboardModifiers, MouseButton, MouseDetail};
        let id = self.id();
        let dom = self.dom_mut();
        let mut event = rdom_core::Event::new("click").with_synthetic(true);
        event.detail = EventDetail::Mouse(MouseDetail {
            button: MouseButton::Left,
            buttons: 0,
            client_x: 0,
            client_y: 0,
            delta_x: 0,
            delta_y: 0,
            modifiers: KeyboardModifiers::default(),
        });
        let mut tui = crate::TuiEvent { event };
        let _ = crate::TuiDispatchExt::dispatch_tui_event(dom, id, &mut tui);
    }

    fn set_scroll_top(&mut self, value: i32) -> Result<()> {
        let id = self.id();
        let dom = self.dom_mut();
        let cur_x = read_scroll_x(dom, id);
        write_scroll_clamped(dom, id, cur_x, value);
        Ok(())
    }

    fn set_scroll_left(&mut self, value: i32) -> Result<()> {
        let id = self.id();
        let dom = self.dom_mut();
        let cur_y = read_scroll_y(dom, id);
        write_scroll_clamped(dom, id, value, cur_y);
        Ok(())
    }

    fn scroll_to(&mut self, x: i32, y: i32) -> Result<()> {
        let id = self.id();
        let dom = self.dom_mut();
        write_scroll_clamped(dom, id, x, y);
        Ok(())
    }

    fn scroll_by(&mut self, dx: i32, dy: i32) -> Result<()> {
        let id = self.id();
        let dom = self.dom_mut();
        let new_x = read_scroll_x(dom, id).saturating_add(dx);
        let new_y = read_scroll_y(dom, id).saturating_add(dy);
        write_scroll_clamped(dom, id, new_x, new_y);
        Ok(())
    }

    fn scroll_into_view(&mut self) -> Result<()> {
        let id = self.id();
        let dom = self.dom_mut();
        let Some(ancestor) = nearest_scrollable_ancestor(dom, id) else {
            return Ok(());
        };
        let (rel_x, rel_y) = pre_scroll_offset_within(dom, id, ancestor);
        write_scroll_clamped(dom, ancestor, rel_x, rel_y);
        Ok(())
    }

    fn style_mut(&mut self) -> Option<crate::cssom::StyleDeclarationMut<'_>> {
        // Non-element nodes have no inline style — no-op.
        self.as_ref().tag_name()?;
        let id = self.id();
        let dom = self.dom_mut();
        Some(crate::cssom::StyleDeclarationMut::new(dom.node_mut(id)))
    }

    fn set_details_open(&mut self, value: bool) -> Result<()> {
        if self.as_ref().tag_name() != Some("details") {
            return Ok(());
        }
        write_boolean_attribute(self, "open", value)
    }

    fn set_dialog_return_value(&mut self, value: impl Into<String>) -> Result<()> {
        if self.as_ref().tag_name() != Some("dialog") {
            return Ok(());
        }
        let id = self.id();
        let dom = self.dom_mut();
        let s = value.into();
        crate::runtime::builtins::dialog::set_return_value(dom, id, &s);
        Ok(())
    }

    fn form_request_submit(&mut self, submitter: Option<NodeId>) -> Result<bool> {
        if self.as_ref().tag_name() != Some("form") {
            return Ok(false);
        }
        let id = self.id();
        let dom = self.dom_mut();
        Ok(crate::runtime::builtins::form::fire_submit(
            dom, id, submitter,
        ))
    }
}
