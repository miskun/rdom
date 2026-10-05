# The sealed extension traits (C4G-SEALED)

rdom-tui's call-only extension traits are sealed (DESIGN "Which rdom-tui traits a consumer implements"): each block below implements every method of one of them for a type outside rdom-tui, and fails to compile only because the type is not `Sealed`. Before the seal each compiled. `CascadeExt` carries the same example on its own docs; `Backend`, `Clipboard` and `UrlOpener` stay implementable.

`TuiNodeExt`:

```compile_fail
use rdom_tui::core_api::NodeRef;
use rdom_tui::cssom::{StyleDeclaration, StyleDeclarationMut};
use rdom_tui::runtime::timers::{TimerCtx, TimerId, TuiTimers};
use rdom_tui::*;
struct Mine;
impl<'a> TuiNodeExt<'a> for Mine {
    fn tui_ext(&self) -> Option<&'a TuiExt> { todo!() }
    fn is_editable(&self) -> bool { todo!() }
}
```

`TuiNodeMutExt`:

```compile_fail
use rdom_tui::core_api::NodeRef;
use rdom_tui::cssom::{StyleDeclaration, StyleDeclarationMut};
use rdom_tui::runtime::timers::{TimerCtx, TimerId, TuiTimers};
use rdom_tui::*;
struct Mine;
impl<'a> TuiNodeMutExt<'a> for Mine {
    fn tui_ext_mut(&mut self) -> Option<&mut TuiExt> { todo!() }
}
```

`TuiAccessors`:

```compile_fail
use rdom_tui::core_api::NodeRef;
use rdom_tui::cssom::{StyleDeclaration, StyleDeclarationMut};
use rdom_tui::runtime::timers::{TimerCtx, TimerId, TuiTimers};
use rdom_tui::*;
struct Mine;
impl<'a> TuiAccessors<'a> for Mine {
    fn value(&self) -> Option<std::string::String> { todo!() }
    fn checked(&self) -> bool { todo!() }
    fn default_value(&self) -> Option<std::string::String> { todo!() }
    fn default_checked(&self) -> Option<bool> { todo!() }
    fn default_selected(&self) -> Option<bool> { todo!() }
    fn indeterminate(&self) -> bool { todo!() }
    fn disabled(&self) -> bool { todo!() }
    fn read_only(&self) -> bool { todo!() }
    fn inert(&self) -> bool { todo!() }
    fn is_content_editable(&self) -> bool { todo!() }
    fn effective_tab_index(&self) -> Option<i32> { todo!() }
    fn bounding_rect(&self) -> Option<LayoutRect> { todo!() }
    fn scroll_top(&self) -> Option<i32> { todo!() }
    fn scroll_left(&self) -> Option<i32> { todo!() }
    fn scroll_width(&self) -> Option<i32> { todo!() }
    fn scroll_height(&self) -> Option<i32> { todo!() }
    fn style(&self) -> Option<StyleDeclaration> { todo!() }
    fn input_value(&self) -> Option<std::string::String> { todo!() }
    fn input_type(&self) -> Option<std::string::String> { todo!() }
    fn input_name(&self) -> Option<std::string::String> { todo!() }
    fn input_placeholder(&self) -> Option<std::string::String> { todo!() }
    fn input_form(&self) -> Option<NodeId> { todo!() }
    fn textarea_value(&self) -> Option<std::string::String> { todo!() }
    fn textarea_name(&self) -> Option<std::string::String> { todo!() }
    fn textarea_form(&self) -> Option<NodeId> { todo!() }
    fn select_value(&self) -> Option<std::string::String> { todo!() }
    fn select_options(&self) -> Option<Vec<NodeId>> { todo!() }
    fn select_selected_options(&self) -> Option<Vec<NodeId>> { todo!() }
    fn select_selected_index(&self) -> Option<i32> { todo!() }
    fn select_form(&self) -> Option<NodeId> { todo!() }
    fn option_value(&self) -> Option<std::string::String> { todo!() }
    fn option_label(&self) -> Option<std::string::String> { todo!() }
    fn option_selected(&self) -> bool { todo!() }
    fn details_open(&self) -> bool { todo!() }
    fn dialog_open(&self) -> bool { todo!() }
    fn dialog_return_value(&self) -> Option<std::string::String> { todo!() }
    fn button_form(&self) -> Option<NodeId> { todo!() }
    fn label_html_for(&self) -> Option<std::string::String> { todo!() }
    fn label_control(&self) -> Option<NodeId> { todo!() }
    fn progress_value(&self) -> Option<f64> { todo!() }
    fn progress_max(&self) -> Option<f64> { todo!() }
    fn meter_value(&self) -> Option<f64> { todo!() }
    fn meter_min(&self) -> Option<f64> { todo!() }
    fn meter_max(&self) -> Option<f64> { todo!() }
    fn meter_low(&self) -> Option<f64> { todo!() }
    fn meter_high(&self) -> Option<f64> { todo!() }
    fn meter_optimum(&self) -> Option<f64> { todo!() }
    fn form_elements(&self) -> Option<Vec<NodeId>> { todo!() }
    fn form_length(&self) -> Option<usize> { todo!() }
    fn validity(&self) -> Option<ValidityState> { todo!() }
    fn will_validate(&self) -> bool { todo!() }
    fn validation_message(&self) -> Option<std::string::String> { todo!() }
}
```

`TuiAccessorsMut`:

```compile_fail
use rdom_tui::core_api::NodeRef;
use rdom_tui::cssom::{StyleDeclaration, StyleDeclarationMut};
use rdom_tui::runtime::timers::{TimerCtx, TimerId, TuiTimers};
use rdom_tui::*;
struct Mine;
impl<'a> TuiAccessorsMut<'a> for Mine {
    fn set_value(&mut self, _: impl Into<String>) -> std::result::Result<(), DomError> { todo!() }
    fn set_checked(&mut self, _: bool) -> std::result::Result<(), DomError> { todo!() }
    fn set_default_value(&mut self, _: impl Into<String>) -> std::result::Result<(), DomError> { todo!() }
    fn set_default_checked(&mut self, _: bool) -> std::result::Result<(), DomError> { todo!() }
    fn set_default_selected(&mut self, _: bool) -> std::result::Result<(), DomError> { todo!() }
    fn set_indeterminate(&mut self, _: bool) -> std::result::Result<(), DomError> { todo!() }
    fn set_disabled(&mut self, _: bool) -> std::result::Result<(), DomError> { todo!() }
    fn set_read_only(&mut self, _: bool) -> std::result::Result<(), DomError> { todo!() }
    fn set_inert(&mut self, _: bool) -> std::result::Result<(), DomError> { todo!() }
    fn focus(&mut self) { todo!() }
    fn blur(&mut self) { todo!() }
    fn click(&mut self) { todo!() }
    fn set_scroll_top(&mut self, _: i32) -> std::result::Result<(), DomError> { todo!() }
    fn set_scroll_left(&mut self, _: i32) -> std::result::Result<(), DomError> { todo!() }
    fn scroll_to(&mut self, _: i32, _: i32) -> std::result::Result<(), DomError> { todo!() }
    fn scroll_by(&mut self, _: i32, _: i32) -> std::result::Result<(), DomError> { todo!() }
    fn scroll_with(&mut self, _: ScrollToOptions) -> std::result::Result<(), DomError> { todo!() }
    fn scroll_by_with(&mut self, _: ScrollToOptions) -> std::result::Result<(), DomError> { todo!() }
    fn scroll_into_view(&mut self) -> std::result::Result<(), DomError> { todo!() }
    fn scroll_into_view_with(&mut self, _: ScrollIntoViewOptions) -> std::result::Result<(), DomError> { todo!() }
    fn style_mut(&mut self) -> Option<StyleDeclarationMut<'_>> { todo!() }
    fn set_details_open(&mut self, _: bool) -> std::result::Result<(), DomError> { todo!() }
    fn set_dialog_return_value(&mut self, _: impl Into<String>) -> std::result::Result<(), DomError> { todo!() }
    fn form_request_submit(&mut self, _: Option<NodeId>) -> std::result::Result<SubmitOutcome, DomError> { todo!() }
    fn check_validity(&mut self) -> bool { todo!() }
    fn report_validity(&mut self) -> bool { todo!() }
    fn set_custom_validity(&mut self, _: &str) -> std::result::Result<(), DomError> { todo!() }
}
```

`TuiDocAccessors`:

```compile_fail
use rdom_tui::core_api::NodeRef;
use rdom_tui::cssom::{StyleDeclaration, StyleDeclarationMut};
use rdom_tui::runtime::timers::{TimerCtx, TimerId, TuiTimers};
use rdom_tui::*;
struct Mine;
impl TuiDocAccessors for Mine {
    fn element_from_point(&self, _: i32, _: i32) -> Option<NodeRef<'_, TuiExt>> { todo!() }
    fn elements_from_point(&self, _: i32, _: i32) -> Vec<NodeRef<'_, TuiExt>> { todo!() }
    fn caret_position_from_point(&self, _: i32, _: i32) -> Option<Position> { todo!() }
}
```

`LayoutExt`:

```compile_fail
use rdom_tui::core_api::NodeRef;
use rdom_tui::cssom::{StyleDeclaration, StyleDeclarationMut};
use rdom_tui::runtime::timers::{TimerCtx, TimerId, TuiTimers};
use rdom_tui::*;
struct Mine;
impl LayoutExt for Mine {
    fn layout_dom(&mut self, _: Rect) { todo!() }
}
```

`PaintExt`:

```compile_fail
use rdom_tui::core_api::NodeRef;
use rdom_tui::cssom::{StyleDeclaration, StyleDeclarationMut};
use rdom_tui::runtime::timers::{TimerCtx, TimerId, TuiTimers};
use rdom_tui::*;
struct Mine;
impl PaintExt for Mine {
    fn paint_dom(&self, _: &mut Buffer, _: Rect) { todo!() }
}
```

`HitTestExt`:

```compile_fail
use rdom_tui::core_api::NodeRef;
use rdom_tui::cssom::{StyleDeclaration, StyleDeclarationMut};
use rdom_tui::runtime::timers::{TimerCtx, TimerId, TuiTimers};
use rdom_tui::*;
struct Mine;
impl HitTestExt for Mine {
    fn hit_test(&self, _: u16, _: u16) -> Option<NodeId> { todo!() }
    fn hit_test_path(&self, _: u16, _: u16) -> Vec<NodeId> { todo!() }
    fn position_at(&self, _: u16, _: u16) -> Option<Position> { todo!() }
    fn nearest_selectable_position(&self, _: u16, _: u16) -> Option<Position> { todo!() }
}
```

`TuiDispatchExt`:

```compile_fail
use rdom_tui::core_api::NodeRef;
use rdom_tui::cssom::{StyleDeclaration, StyleDeclarationMut};
use rdom_tui::runtime::timers::{TimerCtx, TimerId, TuiTimers};
use rdom_tui::*;
struct Mine;
impl TuiDispatchExt for Mine {
    fn dispatch_tui_event(&mut self, _: NodeId, _: &mut TuiEvent) -> std::result::Result<(), DomError> { todo!() }
}
```

`TuiTimers`:

```compile_fail
use rdom_tui::core_api::NodeRef;
use rdom_tui::cssom::{StyleDeclaration, StyleDeclarationMut};
use rdom_tui::runtime::timers::{TimerCtx, TimerId, TuiTimers};
use rdom_tui::*;
struct Mine;
impl TuiTimers for Mine {
    fn set_timeout(&mut self, _: impl FnOnce(&mut TimerCtx<'_>) + 'static, _: u32) -> TimerId { todo!() }
    fn clear_timeout(&mut self, _: TimerId) { todo!() }
    fn set_interval(&mut self, _: impl FnMut(&mut TimerCtx<'_>) -> bool + 'static, _: u32) -> TimerId { todo!() }
    fn clear_interval(&mut self, _: TimerId) { todo!() }
    fn request_animation_frame(&mut self, _: impl FnOnce(&mut TimerCtx<'_>, f64) + 'static) -> TimerId { todo!() }
    fn cancel_animation_frame(&mut self, _: TimerId) { todo!() }
    fn queue_microtask(&mut self, _: impl FnOnce(&mut TimerCtx<'_>) + 'static) { todo!() }
}
```
