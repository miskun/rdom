//! `impl TuiAccessors for NodeRef` — read-side accessor methods on
//! `NodeRef<'a, TuiExt>`. Trait declaration lives in `super` (mod.rs);
//! private helpers live in `super::helpers`.

use rdom_core::NodeId;

use super::helpers::{effective_content_editable, nearest_form_ancestor, parse_numeric_attribute};
use super::{DomRect, TuiAccessors};
use crate::TuiExt;

impl<'a> TuiAccessors<'a> for rdom_core::NodeRef<'a, TuiExt> {
    fn value(&self) -> Option<String> {
        match self.tag_name()? {
            "input" => Some(crate::runtime::builtins::input::value(
                self.dom(),
                self.id(),
            )),
            "textarea" => Some(self.text_content()),
            "select" => Some(crate::runtime::builtins::select::value(
                self.dom(),
                self.id(),
            )),
            _ => None,
        }
    }

    fn checked(&self) -> bool {
        self.has_attribute("checked")
    }

    fn indeterminate(&self) -> bool {
        self.has_attribute("indeterminate")
    }

    fn disabled(&self) -> bool {
        self.has_attribute("disabled")
    }

    fn read_only(&self) -> bool {
        self.has_attribute("readonly")
    }

    fn inert(&self) -> bool {
        self.has_attribute("inert")
    }

    fn is_content_editable(&self) -> bool {
        effective_content_editable(self.dom(), self.id())
    }

    fn effective_tab_index(&self) -> Option<i32> {
        crate::runtime::focus::tabindex::tab_index(self.dom(), self.id())
    }

    fn bounding_rect(&self) -> Option<DomRect> {
        use crate::node::TuiNodeExt;
        Some(self.tui_ext()?.layout)
    }

    fn scroll_top(&self) -> Option<i32> {
        use crate::node::TuiNodeExt;
        Some(self.tui_ext()?.scroll_y as i32)
    }

    fn scroll_left(&self) -> Option<i32> {
        use crate::node::TuiNodeExt;
        Some(self.tui_ext()?.scroll_x as i32)
    }

    fn scroll_width(&self) -> Option<i32> {
        use crate::node::TuiNodeExt;
        Some(self.tui_ext()?.scroll_content_width as i32)
    }

    fn scroll_height(&self) -> Option<i32> {
        use crate::node::TuiNodeExt;
        Some(self.tui_ext()?.scroll_content_height as i32)
    }

    fn style(&self) -> Option<crate::cssom::StyleDeclaration> {
        crate::cssom::declaration::from_node_ref(self)
    }

    fn input_value(&self) -> Option<String> {
        match self.tag_name()? {
            "input" => Some(crate::runtime::builtins::input::value(
                self.dom(),
                self.id(),
            )),
            _ => None,
        }
    }

    fn input_type(&self) -> Option<String> {
        match self.tag_name()? {
            "input" => Some(self.get_attribute("type").unwrap_or("text").to_string()),
            _ => None,
        }
    }

    fn input_name(&self) -> Option<String> {
        match self.tag_name()? {
            "input" => self.get_attribute("name").map(str::to_string),
            _ => None,
        }
    }

    fn input_placeholder(&self) -> Option<String> {
        match self.tag_name()? {
            "input" => self.get_attribute("placeholder").map(str::to_string),
            _ => None,
        }
    }

    fn input_form(&self) -> Option<NodeId> {
        match self.tag_name()? {
            "input" => nearest_form_ancestor(self.dom(), self.id()),
            _ => None,
        }
    }

    fn textarea_value(&self) -> Option<String> {
        match self.tag_name()? {
            "textarea" => Some(self.text_content()),
            _ => None,
        }
    }

    fn textarea_name(&self) -> Option<String> {
        match self.tag_name()? {
            "textarea" => self.get_attribute("name").map(str::to_string),
            _ => None,
        }
    }

    fn textarea_form(&self) -> Option<NodeId> {
        match self.tag_name()? {
            "textarea" => nearest_form_ancestor(self.dom(), self.id()),
            _ => None,
        }
    }

    fn select_value(&self) -> Option<String> {
        match self.tag_name()? {
            "select" => Some(crate::runtime::builtins::select::value(
                self.dom(),
                self.id(),
            )),
            _ => None,
        }
    }

    fn select_options(&self) -> Option<Vec<NodeId>> {
        match self.tag_name()? {
            "select" => Some(crate::runtime::builtins::select::options(
                self.dom(),
                self.id(),
            )),
            _ => None,
        }
    }

    fn select_selected_options(&self) -> Option<Vec<NodeId>> {
        match self.tag_name()? {
            "select" => Some(crate::runtime::builtins::select::selected_options(
                self.dom(),
                self.id(),
            )),
            _ => None,
        }
    }

    fn select_selected_index(&self) -> Option<i32> {
        match self.tag_name()? {
            "select" => {
                let opts = crate::runtime::builtins::select::options(self.dom(), self.id());
                let dom = self.dom();
                let idx = opts
                    .iter()
                    .position(|&id| dom.node(id).has_attribute("selected"))
                    .map_or(-1, |i| i as i32);
                Some(idx)
            }
            _ => None,
        }
    }

    fn select_form(&self) -> Option<NodeId> {
        match self.tag_name()? {
            "select" => nearest_form_ancestor(self.dom(), self.id()),
            _ => None,
        }
    }

    fn option_value(&self) -> Option<String> {
        match self.tag_name()? {
            "option" => Some(crate::runtime::builtins::select::option_value(
                self.dom(),
                self.id(),
            )),
            _ => None,
        }
    }

    fn option_label(&self) -> Option<String> {
        match self.tag_name()? {
            "option" => Some(crate::runtime::builtins::select::option_label(
                self.dom(),
                self.id(),
            )),
            _ => None,
        }
    }

    fn option_selected(&self) -> bool {
        self.tag_name() == Some("option") && self.has_attribute("selected")
    }

    fn details_open(&self) -> bool {
        self.tag_name() == Some("details") && self.has_attribute("open")
    }

    fn dialog_open(&self) -> bool {
        self.tag_name() == Some("dialog") && self.has_attribute("open")
    }

    fn dialog_return_value(&self) -> Option<String> {
        match self.tag_name()? {
            "dialog" => Some(crate::runtime::builtins::dialog::return_value(
                self.dom(),
                self.id(),
            )),
            _ => None,
        }
    }

    fn button_form(&self) -> Option<NodeId> {
        match self.tag_name()? {
            "button" => nearest_form_ancestor(self.dom(), self.id()),
            _ => None,
        }
    }

    fn label_html_for(&self) -> Option<String> {
        match self.tag_name()? {
            "label" => self.get_attribute("for").map(str::to_string),
            _ => None,
        }
    }

    fn label_control(&self) -> Option<NodeId> {
        match self.tag_name()? {
            "label" => crate::runtime::builtins::label::associated_control(self.dom(), self.id()),
            _ => None,
        }
    }

    fn progress_value(&self) -> Option<f64> {
        match self.tag_name()? {
            "progress" => Some(parse_numeric_attribute(self, "value").unwrap_or(0.0)),
            _ => None,
        }
    }

    fn progress_max(&self) -> Option<f64> {
        match self.tag_name()? {
            "progress" => Some(parse_numeric_attribute(self, "max").unwrap_or(1.0)),
            _ => None,
        }
    }

    fn meter_value(&self) -> Option<f64> {
        match self.tag_name()? {
            "meter" => Some(parse_numeric_attribute(self, "value").unwrap_or(0.0)),
            _ => None,
        }
    }

    fn meter_min(&self) -> Option<f64> {
        match self.tag_name()? {
            "meter" => Some(parse_numeric_attribute(self, "min").unwrap_or(0.0)),
            _ => None,
        }
    }

    fn meter_max(&self) -> Option<f64> {
        match self.tag_name()? {
            "meter" => Some(parse_numeric_attribute(self, "max").unwrap_or(1.0)),
            _ => None,
        }
    }

    fn meter_low(&self) -> Option<f64> {
        match self.tag_name()? {
            "meter" => Some(
                parse_numeric_attribute(self, "low")
                    .unwrap_or_else(|| self.meter_min().unwrap_or(0.0)),
            ),
            _ => None,
        }
    }

    fn meter_high(&self) -> Option<f64> {
        match self.tag_name()? {
            "meter" => Some(
                parse_numeric_attribute(self, "high")
                    .unwrap_or_else(|| self.meter_max().unwrap_or(1.0)),
            ),
            _ => None,
        }
    }

    fn meter_optimum(&self) -> Option<f64> {
        match self.tag_name()? {
            "meter" => Some(parse_numeric_attribute(self, "optimum").unwrap_or_else(|| {
                let min = self.meter_min().unwrap_or(0.0);
                let max = self.meter_max().unwrap_or(1.0);
                (min + max) / 2.0
            })),
            _ => None,
        }
    }

    fn form_elements(&self) -> Option<Vec<NodeId>> {
        match self.tag_name()? {
            "form" => Some(crate::runtime::builtins::form::elements(
                self.dom(),
                self.id(),
            )),
            _ => None,
        }
    }

    fn form_length(&self) -> Option<usize> {
        match self.tag_name()? {
            "form" => Some(crate::runtime::builtins::form::elements(self.dom(), self.id()).len()),
            _ => None,
        }
    }
}
