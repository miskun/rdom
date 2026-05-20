//! `impl TuiAccessors for NodeMut` — read-side accessor methods on
//! `NodeMut<'a, TuiExt>`. Every method delegates to the matching
//! `NodeRef` impl through `as_ref()`, so the read-then-mutate
//! pattern compiles in a single block: the immutable borrow ends
//! when the method returns owned data.

use rdom_core::NodeId;

use super::{DomRect, TuiAccessors};
use crate::TuiExt;

impl<'a> TuiAccessors<'a> for rdom_core::NodeMut<'a, TuiExt> {
    fn value(&self) -> Option<String> {
        self.as_ref().value()
    }

    fn checked(&self) -> bool {
        self.as_ref().checked()
    }

    fn indeterminate(&self) -> bool {
        self.as_ref().indeterminate()
    }

    fn disabled(&self) -> bool {
        self.as_ref().disabled()
    }

    fn read_only(&self) -> bool {
        self.as_ref().read_only()
    }

    fn inert(&self) -> bool {
        self.as_ref().inert()
    }

    fn is_content_editable(&self) -> bool {
        self.as_ref().is_content_editable()
    }

    fn effective_tab_index(&self) -> Option<i32> {
        self.as_ref().effective_tab_index()
    }

    fn bounding_rect(&self) -> Option<DomRect> {
        self.as_ref().bounding_rect()
    }

    fn scroll_top(&self) -> Option<i32> {
        self.as_ref().scroll_top()
    }

    fn scroll_left(&self) -> Option<i32> {
        self.as_ref().scroll_left()
    }

    fn scroll_width(&self) -> Option<i32> {
        self.as_ref().scroll_width()
    }

    fn scroll_height(&self) -> Option<i32> {
        self.as_ref().scroll_height()
    }

    fn style(&self) -> Option<crate::cssom::StyleDeclaration> {
        // Reborrow through as_ref so the StyleDeclaration's
        // lifetime is tied to &self, not to the NodeMut's 'a.
        crate::cssom::declaration::from_node_ref(&self.as_ref())
    }

    fn input_value(&self) -> Option<String> {
        self.as_ref().input_value()
    }

    fn input_type(&self) -> Option<String> {
        self.as_ref().input_type()
    }

    fn input_name(&self) -> Option<String> {
        self.as_ref().input_name()
    }

    fn input_placeholder(&self) -> Option<String> {
        self.as_ref().input_placeholder()
    }

    fn input_form(&self) -> Option<NodeId> {
        self.as_ref().input_form()
    }

    fn textarea_value(&self) -> Option<String> {
        self.as_ref().textarea_value()
    }

    fn textarea_name(&self) -> Option<String> {
        self.as_ref().textarea_name()
    }

    fn textarea_form(&self) -> Option<NodeId> {
        self.as_ref().textarea_form()
    }

    fn select_value(&self) -> Option<String> {
        self.as_ref().select_value()
    }

    fn select_options(&self) -> Option<Vec<NodeId>> {
        self.as_ref().select_options()
    }

    fn select_selected_options(&self) -> Option<Vec<NodeId>> {
        self.as_ref().select_selected_options()
    }

    fn select_selected_index(&self) -> Option<i32> {
        self.as_ref().select_selected_index()
    }

    fn select_form(&self) -> Option<NodeId> {
        self.as_ref().select_form()
    }

    fn option_value(&self) -> Option<String> {
        self.as_ref().option_value()
    }

    fn option_label(&self) -> Option<String> {
        self.as_ref().option_label()
    }

    fn option_selected(&self) -> bool {
        self.as_ref().option_selected()
    }

    fn details_open(&self) -> bool {
        self.as_ref().details_open()
    }

    fn dialog_open(&self) -> bool {
        self.as_ref().dialog_open()
    }

    fn dialog_return_value(&self) -> Option<String> {
        self.as_ref().dialog_return_value()
    }

    fn button_form(&self) -> Option<NodeId> {
        self.as_ref().button_form()
    }

    fn label_html_for(&self) -> Option<String> {
        self.as_ref().label_html_for()
    }

    fn label_control(&self) -> Option<NodeId> {
        self.as_ref().label_control()
    }

    fn progress_value(&self) -> Option<f64> {
        self.as_ref().progress_value()
    }

    fn progress_max(&self) -> Option<f64> {
        self.as_ref().progress_max()
    }

    fn meter_value(&self) -> Option<f64> {
        self.as_ref().meter_value()
    }

    fn meter_min(&self) -> Option<f64> {
        self.as_ref().meter_min()
    }

    fn meter_max(&self) -> Option<f64> {
        self.as_ref().meter_max()
    }

    fn meter_low(&self) -> Option<f64> {
        self.as_ref().meter_low()
    }

    fn meter_high(&self) -> Option<f64> {
        self.as_ref().meter_high()
    }

    fn meter_optimum(&self) -> Option<f64> {
        self.as_ref().meter_optimum()
    }

    fn form_elements(&self) -> Option<Vec<NodeId>> {
        self.as_ref().form_elements()
    }

    fn form_length(&self) -> Option<usize> {
        self.as_ref().form_length()
    }
}
