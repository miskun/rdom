//! The state of an `<input>`'s `type` attribute (HTML §4.10.5).
//!
//! `type` is an *enumerated attribute* (HTML §2.3.3): its keywords match
//! ASCII case-insensitively, and both the missing value default and the
//! invalid value default are the Text state. Every read of an input's
//! type goes through [`Dom::input_type_state`] so no call site compares
//! the raw attribute string.

use crate::dom::Dom;
use crate::node_id::NodeId;

/// The state of an `<input>`'s `type` attribute (HTML §4.10.5, the
/// table of keywords and states). rdom renders only some of them; the
/// rest are still recognized so an unshipped type such as `date` is not
/// mistaken for Text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputTypeState {
    Hidden,
    Text,
    Search,
    Tel,
    Url,
    Email,
    Password,
    Date,
    Month,
    Week,
    Time,
    DateTimeLocal,
    Number,
    Range,
    Color,
    Checkbox,
    Radio,
    File,
    Submit,
    Image,
    Reset,
    Button,
}

impl InputTypeState {
    /// Every state, in the order of the HTML keyword table.
    const ALL: [Self; 22] = [
        Self::Hidden,
        Self::Text,
        Self::Search,
        Self::Tel,
        Self::Url,
        Self::Email,
        Self::Password,
        Self::Date,
        Self::Month,
        Self::Week,
        Self::Time,
        Self::DateTimeLocal,
        Self::Number,
        Self::Range,
        Self::Color,
        Self::Checkbox,
        Self::Radio,
        Self::File,
        Self::Submit,
        Self::Image,
        Self::Reset,
        Self::Button,
    ];

    /// The state a `type` attribute value maps to: its keyword matched
    /// ASCII case-insensitively, Text when the attribute is missing or
    /// names no state (HTML §2.3.3).
    pub fn from_attribute(value: Option<&str>) -> Self {
        let Some(value) = value else {
            return Self::Text;
        };
        Self::ALL
            .into_iter()
            .find(|s| s.as_str().eq_ignore_ascii_case(value))
            .unwrap_or(Self::Text)
    }

    /// The state's canonical keyword — the `input.type` IDL value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Text => "text",
            Self::Search => "search",
            Self::Tel => "tel",
            Self::Url => "url",
            Self::Email => "email",
            Self::Password => "password",
            Self::Date => "date",
            Self::Month => "month",
            Self::Week => "week",
            Self::Time => "time",
            Self::DateTimeLocal => "datetime-local",
            Self::Number => "number",
            Self::Range => "range",
            Self::Color => "color",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::File => "file",
            Self::Submit => "submit",
            Self::Image => "image",
            Self::Reset => "reset",
            Self::Button => "button",
        }
    }
}

impl<Ext> Dom<Ext> {
    /// The [`InputTypeState`] of `id` when it is an `<input>`, `None`
    /// for any other node.
    pub fn input_type_state(&self, id: NodeId) -> Option<InputTypeState> {
        if !self.tag_is(id, "input") {
            return None;
        }
        Some(InputTypeState::from_attribute(
            self.get_attribute(id, "type"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::InputTypeState;
    use crate::Dom;

    #[test]
    fn keywords_match_ascii_case_insensitively() {
        assert_eq!(
            InputTypeState::from_attribute(Some("CheckBox")),
            InputTypeState::Checkbox
        );
        assert_eq!(
            InputTypeState::from_attribute(Some("DATETIME-local")),
            InputTypeState::DateTimeLocal
        );
        assert_eq!(
            InputTypeState::from_attribute(Some("submit")),
            InputTypeState::Submit
        );
    }

    /// HTML §4.10.5: the missing and the invalid value default are both
    /// Text; non-ASCII case folding does not apply.
    #[test]
    fn missing_and_invalid_values_are_text() {
        assert_eq!(InputTypeState::from_attribute(None), InputTypeState::Text);
        assert_eq!(
            InputTypeState::from_attribute(Some("bogus")),
            InputTypeState::Text
        );
        assert_eq!(
            InputTypeState::from_attribute(Some("")),
            InputTypeState::Text
        );
        // Only ASCII letters fold: a dotless ı is not an `i`.
        assert_eq!(
            InputTypeState::from_attribute(Some("h\u{131}dden")),
            InputTypeState::Text
        );
    }

    #[test]
    fn every_state_round_trips_through_its_keyword() {
        for s in InputTypeState::ALL {
            assert_eq!(InputTypeState::from_attribute(Some(s.as_str())), s);
        }
    }

    #[test]
    fn input_type_state_is_none_for_non_inputs() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let input = dom.create_element("input");
        let button = dom.create_element("button");
        dom.set_attribute(input, "type", "Radio").unwrap();
        dom.set_attribute(button, "type", "radio").unwrap();
        dom.append_child(root, input).unwrap();
        dom.append_child(root, button).unwrap();
        assert_eq!(dom.input_type_state(input), Some(InputTypeState::Radio));
        assert_eq!(dom.input_type_state(button), None);
    }
}
