//! C15G-README — the README's property list is generated from
//! [`property_names`](super::property_names) and stays complete: the block
//! between its `property-names` markers names every property the
//! dispatch table knows, and nothing else.

use super::property_names;

const README: &str = include_str!("../../README.md");

/// The backticked names between the README's markers.
fn listed() -> Vec<String> {
    let begin = "<!-- property-names:begin -->";
    let end = "<!-- property-names:end -->";
    let start = README.find(begin).expect("the begin marker") + begin.len();
    let stop = README[start..].find(end).expect("the end marker") + start;
    README[start..stop]
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// What the block should hold: every name, sorted, one backticked list.
fn expected() -> String {
    let mut names: Vec<_> = property_names().to_vec();
    names.sort_unstable();
    names
        .iter()
        .map(|n| format!("`{n}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn the_readme_lists_every_property() {
    let mut want: Vec<_> = property_names().iter().map(|s| s.to_string()).collect();
    want.sort_unstable();
    assert_eq!(
        listed(),
        want,
        "regenerate the README block between the property-names markers:\n{}",
        expected()
    );
}
