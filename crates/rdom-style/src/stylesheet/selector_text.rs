//! Selector-text preprocessing before `rdom_core::selectors::parse`:
//! splitting a selector list on its top-level commas, and stripping the
//! pseudo-element suffix (`::before`, `::scrollbar-thumb:vertical`, …)
//! that rdom-core's structural grammar does not accept.

use super::PseudoElementTarget;

// ── Pseudo-element suffix extraction ────────────────────────────────

/// Strip a trailing `::before` / `::after` if present. Returns the
/// core selector + pseudo target. Errors if multiple pseudo-element
/// suffixes are present (not allowed in a single selector).
pub(super) fn extract_pseudo_suffix(selector: &str) -> Result<(&str, PseudoElementTarget), String> {
    // The rule: `::before` / `::after` must appear at the END of the
    // selector, directly attached to the last compound (no whitespace
    // between). CSS `element::before` and `element::after` are the only
    // forms we accept.
    let s = selector.trim_end();

    // Disallow multiple `::` pseudo-elements (`::before::after` is invalid).
    let pseudo_count = s.matches("::").count();
    if pseudo_count > 1 {
        return Err(format!(
            "at most one pseudo-element suffix allowed per selector, found {pseudo_count}"
        ));
    }

    if let Some(core) = s.strip_suffix("::before") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::before` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::Before));
    }
    if let Some(core) = s.strip_suffix("::after") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::after` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::After));
    }
    if let Some(core) = s.strip_suffix("::backdrop") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::backdrop` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::Backdrop));
    }
    if let Some(core) = s.strip_suffix("::selection") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::selection` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::Selection));
    }
    // Note: the longer suffixes go first — `::scrollbar` is a prefix of
    // `::scrollbar-thumb`, which is a prefix of the axis forms.
    for (suffix, target) in [
        (
            "::scrollbar-thumb:vertical",
            PseudoElementTarget::ScrollbarThumbVertical,
        ),
        (
            "::scrollbar-thumb:horizontal",
            PseudoElementTarget::ScrollbarThumbHorizontal,
        ),
    ] {
        if let Some(core) = s.strip_suffix(suffix) {
            let core = core.trim_end();
            if core.is_empty() {
                return Err(format!("`{suffix}` requires a host selector"));
            }
            return Ok((core, target));
        }
    }
    if let Some(core) = s.strip_suffix("::scrollbar-thumb") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::scrollbar-thumb` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::ScrollbarThumb));
    }
    if let Some(core) = s.strip_suffix("::scrollbar") {
        let core = core.trim_end();
        if core.is_empty() {
            return Err("`::scrollbar` requires a host selector".to_string());
        }
        return Ok((core, PseudoElementTarget::Scrollbar));
    }

    // A bare `::other` anywhere is rejected (unsupported pseudo-element).
    if pseudo_count == 1 {
        return Err(
            "unsupported pseudo-element; only ::before, ::after, ::backdrop, ::selection, ::scrollbar, ::scrollbar-thumb (optionally :vertical / :horizontal) allowed"
                .to_string(),
        );
    }

    Ok((s, PseudoElementTarget::None))
}

// ── Top-level comma splitting ───────────────────────────────────────

/// Split on commas that are NOT inside `(...)` or `[...]`. Returns
/// borrowed slices into `input`. Empty input yields empty vec.
///
/// `:not(a, b)` has one internal comma; we preserve it as part of the
/// single selector item because it sits inside parens.
pub(super) fn split_top_level_commas(input: &str) -> Vec<&str> {
    if input.trim().is_empty() {
        return Vec::new();
    }
    let bytes = input.as_bytes();
    let mut depth_paren: i32 = 0;
    let mut depth_bracket: i32 = 0;
    let mut start = 0;
    let mut out = Vec::new();

    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth_paren += 1,
            b')' => depth_paren = (depth_paren - 1).max(0),
            b'[' => depth_bracket += 1,
            b']' => depth_bracket = (depth_bracket - 1).max(0),
            b',' if depth_paren == 0 && depth_bracket == 0 => {
                out.push(&input[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&input[start..]);
    out
}
