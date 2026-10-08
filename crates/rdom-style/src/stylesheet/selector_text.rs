//! Selector-text preprocessing before `rdom_core::selectors::parse`:
//! splitting a selector list on its top-level commas, and stripping the
//! pseudo-element suffix (`::before`, `::scrollbar-thumb:vertical`, …)
//! that rdom-core's structural grammar does not accept.

use std::borrow::Cow;

use super::{PseudoElementTarget, UserActionState};

// ── Pseudo-element suffix extraction ────────────────────────────────

/// The supported pseudo-element suffixes. Longer suffixes go first:
/// `::scrollbar` is a prefix of `::scrollbar-thumb`, which is a prefix
/// of the axis forms.
const SUFFIXES: [(&str, PseudoElementTarget); 13] = [
    ("::before", PseudoElementTarget::Before),
    ("::details-content", PseudoElementTarget::DetailsContent),
    ("::after", PseudoElementTarget::After),
    ("::backdrop", PseudoElementTarget::Backdrop),
    ("::placeholder", PseudoElementTarget::Placeholder),
    ("::selection", PseudoElementTarget::Selection),
    ("::first-line", PseudoElementTarget::FirstLine),
    ("::first-letter", PseudoElementTarget::FirstLetter),
    ("::marker", PseudoElementTarget::Marker),
    (
        "::scrollbar-thumb:vertical",
        PseudoElementTarget::ScrollbarThumbVertical,
    ),
    (
        "::scrollbar-thumb:horizontal",
        PseudoElementTarget::ScrollbarThumbHorizontal,
    ),
    ("::scrollbar-thumb", PseudoElementTarget::ScrollbarThumb),
    ("::scrollbar", PseudoElementTarget::Scrollbar),
];

/// The CSS 2.1 pseudo-elements Selectors 4 §15 still accepts with one
/// colon (`:before`, `:after`, `:first-line`, `:first-letter`).
const LEGACY_SUFFIXES: [(&str, PseudoElementTarget); 4] = [
    (":before", PseudoElementTarget::Before),
    (":after", PseudoElementTarget::After),
    (":first-line", PseudoElementTarget::FirstLine),
    (":first-letter", PseudoElementTarget::FirstLetter),
];

/// Strip a trailing pseudo-element (`::before`, `::scrollbar-thumb:vertical`,
/// the legacy `:before`, …) if present. Returns the core selector + pseudo
/// target. Errors if more than one pseudo-element is present (not allowed
/// in a single selector) or the pseudo-element is unsupported. Names are
/// ASCII case-insensitive (Selectors 4 §4.1).
///
/// The pseudo-element attaches to the last compound of the core. When
/// there is none — the core is empty, or ends in whitespace or a
/// combinator (`::before`, `div ::before`, `div > ::after`) — that
/// compound is the implicit universal selector (Selectors 4 §5.2: a
/// compound without a type selector has an implied `*`), so the core
/// returned is the source with `*` appended: `*`, `div *`, `div > *`.
pub(super) fn extract_pseudo_suffix(
    selector: &str,
) -> Result<(Cow<'_, str>, PseudoElementTarget), String> {
    let s = selector.trim_end();

    // Disallow multiple `::` pseudo-elements (`::before::after` is invalid).
    let pseudo_count = s.matches("::").count();
    if pseudo_count > 1 {
        return Err(format!(
            "at most one pseudo-element suffix allowed per selector, found {pseudo_count}"
        ));
    }

    for (suffix, target) in SUFFIXES {
        if let Some(core) = strip_suffix_ignore_case(s, suffix) {
            return Ok((with_compound(core), target));
        }
    }

    // CSS Custom Highlight API 1 §5.1: `::highlight(<custom-ident>)`.
    if let Some((core, name)) = highlight_suffix(s)? {
        return Ok((
            with_compound(core),
            PseudoElementTarget::Highlight(name.into()),
        ));
    }

    // A bare `::other` anywhere is rejected (unsupported pseudo-element).
    if pseudo_count == 1 {
        return Err(
            "unsupported pseudo-element; only ::before, ::after, ::backdrop, ::selection, ::placeholder, ::first-line, ::first-letter, ::marker, ::highlight(<name>), ::details-content, ::scrollbar, ::scrollbar-thumb (optionally :vertical / :horizontal) allowed"
                .to_string(),
        );
    }

    // Selectors 4 §15: the CSS 2.1 single-colon spellings. An escaped
    // colon (`.a\:before`) belongs to an identifier and is no pseudo.
    for (suffix, target) in LEGACY_SUFFIXES {
        if let Some(core) = strip_suffix_ignore_case(s, suffix)
            && core.chars().rev().take_while(|&c| c == '\\').count() % 2 == 0
        {
            return Ok((with_compound(core), target));
        }
    }

    Ok((Cow::Borrowed(s), PseudoElementTarget::None))
}

/// [`extract_pseudo_suffix`], with the user-action pseudo-classes that
/// may follow the pseudo-element (Selectors 4 §3.6.3: `::before:hover`).
///
/// The trailing `:hover` / `:active` / `:focus` / `:focus-visible` /
/// `:focus-within` are peeled off the end — each parsed by rdom-core's
/// selector parser, so names and escapes follow the element grammar —
/// and the rest must end in a pseudo-element that takes them
/// ([`takes_user_action`]). When the rest ends in none, they were the
/// element's own (`a:hover`) and the selector is left whole. Any other
/// pseudo-class after a pseudo-element leaves it unrecognized, an error.
pub(super) fn extract_pseudo_chain(
    selector: &str,
) -> Result<(Cow<'_, str>, PseudoElementTarget, UserActionState), String> {
    let s = selector.trim_end();
    let (head, state) = peel_user_action(s);
    if state.is_empty() {
        let (core, target) = extract_pseudo_suffix(s)?;
        return Ok((core, target, state));
    }
    let (core, target) = extract_pseudo_suffix(head)?;
    if target == PseudoElementTarget::None {
        // `a:hover`: the element's pseudo-classes, rdom-core's to match.
        return Ok((Cow::Borrowed(s), target, UserActionState::EMPTY));
    }
    if !takes_user_action(&target) {
        return Err(format!(
            "no pseudo-class may follow this pseudo-element ({target:?}); \
             only ::before, ::after, ::marker and ::first-letter take :hover / :active / :focus*"
        ));
    }
    Ok((core, target, state))
}

/// The pseudo-elements a user-action pseudo-class may follow here: the
/// boxes whose pointer state the backend tracks (CSS Pseudo 4 §2–§3).
/// `::first-line` (a fragment of a line, no box of its own to hover) and
/// the highlight, scrollbar, `::placeholder`, `::backdrop` and
/// `::details-content` pseudo-elements take none (DIVERGENCES §3).
fn takes_user_action(target: &PseudoElementTarget) -> bool {
    matches!(
        target,
        PseudoElementTarget::Before
            | PseudoElementTarget::After
            | PseudoElementTarget::Marker
            | PseudoElementTarget::FirstLetter
    )
}

/// `s` with its trailing user-action pseudo-classes removed, and those
/// pseudo-classes. Stops at the first segment from the end that is not
/// one (a pseudo-element's `::`, an escaped colon, any other selector).
fn peel_user_action(s: &str) -> (&str, UserActionState) {
    use rdom_core::selectors::{PseudoClass, SimpleSelector};
    let mut head = s;
    let mut state = UserActionState::EMPTY;
    while let Some(at) = head.rfind(':') {
        let before = &head[..at];
        // `::name` is a pseudo-element; `\:` belongs to an identifier;
        // white space before the colon is a descendant combinator.
        let escaped = before.chars().rev().take_while(|&c| c == '\\').count() % 2 == 1;
        if before.ends_with(':') || before.ends_with(char::is_whitespace) || escaped {
            break;
        }
        let Ok(list) = rdom_core::selectors::parse(&format!("*{}", &head[at..])) else {
            break;
        };
        let simples = match list.0.as_slice() {
            [one] if one.ancestors.is_empty() => &one.subject.simples,
            _ => break,
        };
        let class = match simples.as_slice() {
            [SimpleSelector::Universal, SimpleSelector::Pseudo(class)] => *class,
            _ => break,
        };
        let one = match class {
            PseudoClass::Hover => UserActionState::HOVER,
            PseudoClass::Active => UserActionState::ACTIVE,
            PseudoClass::Focus => UserActionState::FOCUS,
            PseudoClass::FocusVisible => UserActionState::FOCUS_VISIBLE,
            PseudoClass::FocusWithin => UserActionState::FOCUS_WITHIN,
            _ => break,
        };
        state = state.with(one);
        head = before;
    }
    (head, state)
}

/// `s` ending in `::highlight(<name>)` (the pseudo-element name ASCII
/// case-insensitive, white space allowed around the argument): the text
/// before it and the name. `Ok(None)` when it does not end in one; an
/// error for an argument that is no single identifier (a
/// `<custom-ident>`, CSS Values 4 §4.2).
fn highlight_suffix(s: &str) -> Result<Option<(&str, String)>, String> {
    const OPEN: &str = "::highlight(";
    let Some(at) = s.rfind("::") else {
        return Ok(None);
    };
    let tail = &s[at..];
    let opens = tail
        .get(..OPEN.len())
        .is_some_and(|t| t.eq_ignore_ascii_case(OPEN));
    if !opens || !tail.ends_with(')') {
        return Ok(None);
    }
    let arg = tail[OPEN.len()..tail.len() - 1].trim();
    if !rdom_core::css_syntax::would_start_ident(arg) {
        return Err(format!("::highlight() takes a name, found {arg:?}"));
    }
    let (name, used) = rdom_core::css_syntax::consume_ident(arg);
    if used != arg.len() {
        return Err(format!("::highlight() takes one name, found {arg:?}"));
    }
    Ok(Some((&s[..at], name)))
}

/// `s` without `suffix`, compared ASCII case-insensitively.
fn strip_suffix_ignore_case<'a>(s: &'a str, suffix: &str) -> Option<&'a str> {
    let split = s.len().checked_sub(suffix.len())?;
    let tail = s.get(split..)?;
    tail.eq_ignore_ascii_case(suffix).then(|| &s[..split])
}

/// `core` (the text before a pseudo-element) with a compound for the
/// pseudo-element to attach to: unchanged when it ends in one, else
/// with the implicit `*` (Selectors 4 §5.2) appended.
fn with_compound(core: &str) -> Cow<'_, str> {
    let ends_in_compound = core.chars().next_back().is_some_and(|last| {
        (!last.is_whitespace() && !matches!(last, '>' | '+' | '~')) || escaped_at_end(core)
    });
    if ends_in_compound {
        Cow::Borrowed(core)
    } else {
        Cow::Owned(format!("{core}*"))
    }
}

/// True when `core`'s last character is escaped (`a\ ` — an escaped
/// space or combinator is part of an identifier, CSS Syntax 3 §4.3.7):
/// an odd run of backslashes precedes it.
fn escaped_at_end(core: &str) -> bool {
    let mut chars = core.chars();
    chars.next_back();
    chars.rev().take_while(|&c| c == '\\').count() % 2 == 1
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

    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate() {
        // `\,` is an escaped comma inside an identifier (CSS Syntax 3
        // §4.3.7), not a list separator.
        if escaped {
            escaped = false;
            continue;
        }
        match b {
            b'\\' => escaped = true,
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
