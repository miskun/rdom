//! # rdom-core — arena-backed DOM for Rust
//!
//! A pure DOM tree with no rendering dependencies. Node / Element / Text /
//! Comment / Fragment types, full tree mutation, attributes, classes, and
//! (in later phases) query selectors, events, and O(1) index lookups.
//!
//! Designed to be paired with `rdom-tui` for terminal rendering, or used
//! standalone for headless tree manipulation, tests, templates, and
//! hypothetical alternative renderers.
//!
//! ## Quick tour
//!
//! ```
//! use rdom_core::{Dom, AdjacentPosition};
//!
//! let mut dom: Dom = Dom::new();
//! let root = dom.root();
//!
//! let hero = dom.create_element("div");
//! dom.node_mut(hero).set_id("hero").unwrap();
//! dom.node_mut(hero).add_class("active").unwrap();
//!
//! let title = dom.create_element("h1");
//! let text = dom.create_text_node("Welcome");
//! dom.node_mut(title).append_child(text).unwrap();
//! dom.node_mut(hero).append_child(title).unwrap();
//! dom.node_mut(root).append_child(hero).unwrap();
//!
//! assert_eq!(dom.node(root).child_element_count(), 1);
//! assert_eq!(dom.node(hero).first_element_child().unwrap().tag_name(), Some("h1"));
//! ```

mod abort;
mod accessor;
mod attrs;
mod clone;
mod constraint;
mod content_editable;
pub mod css_syntax;
mod directionality;
mod dispatch;
mod document_data;
mod dom;
mod dom_string_map;
mod error;
mod event;
mod event_detail;
mod form_assoc;
mod form_control;
mod highlight;
#[cfg(test)]
mod highlight_tests;
mod html_collection;
mod indexes;
mod input_type;
mod insert_adjacent;
mod interaction_state;
mod language;
mod markup;
mod node;
mod node_id;
mod node_list;
mod node_or_string;
mod observer;
mod position;
mod query;
mod query_selector;
mod radio_group;
mod range;
mod selection;
pub mod selectors;
mod text;
mod token_list;
mod traversal;
mod tree;
mod validate;

pub use abort::{AbortController, AbortSignal};
pub use accessor::{ChildIter, ElementChildIter, NodeMut, NodeRef};
pub use constraint::ValidityHook;
pub use content_editable::ContentEditableState;
pub use directionality::Directionality;
pub use dispatch::{ActivationHook, ActivationPhase, EventCtx, ListenerId, ListenerOptions};
pub use dom::Dom;
pub use dom_string_map::{DomStringMap, DomStringMapMut};
pub use error::{DomError, Result};
pub use event::{Event, EventPhase};
pub use event_detail::{
    EventDetail, FormEnctype, FormMethod, InputDetail, InputType, KeyboardDetail,
    KeyboardModifiers, MouseButton, MouseDetail, SubmitDetail, ToggleDetail, ToggleState,
    TransitionDetail,
};
pub use highlight::{Highlight, HighlightRegistry, HighlightType, HighlightsMut};
pub use html_collection::{FormControlsCollection, HtmlCollection};
pub use input_type::InputTypeState;
pub use markup::{VOID_ELEMENTS, is_void_element};
pub use node::{NodeData, NodeType};
pub use node_id::NodeId;
pub use node_list::NodeList;
pub use node_or_string::NodeOrString;
pub use observer::{InteractionKind, Mutation, MutationObserver, ObserverId};
pub use position::DocumentPosition;
pub use query_selector::{CacheWork, SelectorCaches};
pub use selection::{Position, Range, Selection, SelectionSerial};
pub use token_list::{DomTokenList, DomTokenListMut};
pub use traversal::Descendants;
pub use tree::AdjacentPosition;
pub use validate::InvariantViolation;
