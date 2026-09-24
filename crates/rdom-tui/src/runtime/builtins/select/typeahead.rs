//! Type-ahead (C.7c + polish #6): the per-select keystroke buffer
//! kept on the node's `TuiExt::typeahead`, its inactivity timeout,
//! and the prefix / cycle match that moves the highlight (and, in
//! single-select, the selection) to the next option whose label
//! starts with what was typed.
//!
//! A single-keystroke match (C.7c original) is frustrating when
//! several options share a first letter. This upgrade accumulates
//! typed characters within an inactivity window and matches
//! whole-prefix, case-insensitive.
//!
//! Reset rules:
//! - Inactivity timeout of [`TYPEAHEAD_TIMEOUT`] since the last
//!   keystroke → next key starts a fresh buffer.
//! - Focus moves to a different `<select>` (or away entirely) →
//!   buffer reset on next match.
//! - Buffer is shared across all selects (thread-local); since the
//!   user can only interact with one focused select at a time,
//!   cross-contamination isn't possible.
//!
//! Stickiness: if the appended character extends the buffer to a
//! prefix no option matches, we still update the buffer (so the
//! user's next keystroke combines with it) and fall back to a
//! single-char search of the new key alone — matches browser
//! behavior where repeated typing "cycles" even past misses.

use std::time::{Duration, Instant};

use rdom_core::NodeId;

use super::model::{option_label, options};
use super::state::{fire_input_and_change, highlight, select_single, set_anchor, set_highlight};
use crate::TuiDom;

const TYPEAHEAD_TIMEOUT: Duration = Duration::from_millis(500);

/// Decode the DOM `KeyboardEvent.key` string into a single
/// printable character. Returns `None` for named keys (`"Enter"`,
/// `"ArrowUp"`, …), multi-char strings, and control characters.
/// Type-ahead, character-insertion, and printable-key heuristics
/// share this so the "is this a typed character?" check has one
/// home.
pub(super) fn single_printable_char(key: &str) -> Option<char> {
    let mut chars = key.chars();
    let first = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if first.is_control() {
        return None;
    }
    Some(first)
}

pub(super) fn typeahead_search(dom: &mut TuiDom, select: NodeId, ch: char, multi: bool) {
    let all = options(dom, select);
    if all.is_empty() {
        return;
    }

    // Update the shared buffer, returning (query, cycle_mode). In
    // cycle mode we advance the start index PAST the current
    // highlight so repeated same-letter taps cycle through matches;
    // in prefix mode (multi-char buffer) we start AT the current
    // highlight so extra letters refine the match.
    // The buffer lives on the select's own ext (per node, never shared
    // between two selects) and runs on the scheduler clock under an App.
    let now = crate::runtime::timers::current_now().unwrap_or_else(Instant::now);
    let (query_lower, cycle_mode): (String, bool) = {
        let mut node = dom.node_mut(select);
        let Some(ext) = node.ext_mut() else {
            return;
        };
        let st = ext.typeahead.get_or_insert_with(Default::default);
        let expired = st
            .last
            .is_none_or(|t| now.duration_since(t) > TYPEAHEAD_TIMEOUT);
        let lc = ch.to_ascii_lowercase();
        let mut cycle = false;

        if expired {
            st.buffer.clear();
            st.buffer.push(lc);
        } else if st.buffer.len() == 1 && st.buffer.starts_with(lc) {
            // Same single char repeated within the timeout →
            // cycle. Buffer stays as that one char.
            cycle = true;
        } else {
            st.buffer.push(lc);
        }
        st.last = Some(now);
        (st.buffer.clone(), cycle)
    };

    let start_idx = highlight(dom, select)
        .and_then(|h| all.iter().position(|&o| o == h))
        .map(|i| if cycle_mode { i + 1 } else { i })
        .unwrap_or(0);
    let len = all.len();

    // Two passes: current-and-after then from-start (wrap). For a
    // single-char buffer we advance PAST the current highlight so
    // repeated taps cycle; for a multi-char buffer we start AT the
    // current highlight so additional letters refine rather than
    // skip.
    let target = (0..len).find_map(|offset| {
        let i = (start_idx + offset) % len;
        let opt = all[i];
        if dom.node(opt).has_attribute("disabled") {
            return None;
        }
        let label = option_label(dom, opt).to_ascii_lowercase();
        label.starts_with(&query_lower).then_some(opt)
    });
    let Some(target) = target else { return };

    set_highlight(dom, select, Some(target));
    if !multi {
        select_single(dom, select, target);
    } else {
        // Multi-select: clear anchor so the next shift-action
        // starts from the new highlight (matches the C.7a arrow-
        // nav pattern).
        set_anchor(dom, select, None);
    }
    fire_input_and_change(dom, select);
}
