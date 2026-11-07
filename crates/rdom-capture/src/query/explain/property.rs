//! How one property's value came to be on an element (`explain`'s style
//! entries): its winning declaration, a CSS-wide keyword resolved, or the
//! value it inherits up the cascade's own parent chain — each ancestor's
//! style trace built once per `explain`.

use std::collections::HashMap;

use rdom_tui::App;
use rdom_tui::TuiNodeExt;
use rdom_tui::capture::{
    DeclarationOrigin, Declared, Inheritance, StyleTrace, css_wide_keyword, inheritance_parent,
};
use rdom_tui::core_api::NodeId;
use rdom_tui::render::TestBackend;

use super::super::Detail;
use super::{PropertyInfo, decl_info};
use crate::capture::node_key;

/// Style traces of the elements one `explain` reads, each built once.
pub(super) struct Traces<'a> {
    app: &'a App<TestBackend>,
    cache: HashMap<NodeId, StyleTrace>,
}

impl<'a> Traces<'a> {
    pub(super) fn new(app: &'a App<TestBackend>) -> Self {
        Traces {
            app,
            cache: HashMap::new(),
        }
    }

    pub(super) fn of(&mut self, id: NodeId) -> &StyleTrace {
        let app = self.app;
        self.cache
            .entry(id)
            .or_insert_with(|| app.explain_style(id).unwrap_or_default())
    }
}

fn info(property: &str, value: String, from: &'static str) -> PropertyInfo {
    PropertyInfo {
        property: property.to_string(),
        value,
        from,
        keyword: None,
        declaration: None,
        ancestor: None,
        note: None,
        computed: None,
        overridden: Vec::new(),
        more_overridden: 0,
    }
}

/// How `property` came to be on `id`: its winning declaration — a CSS-wide
/// keyword resolved to what it means — else its inherited value from the
/// element it inherits from, else `initial`.
pub(super) fn property(
    traces: &mut Traces<'_>,
    id: NodeId,
    property: &str,
    detail: Detail,
) -> PropertyInfo {
    let keep = match detail {
        Detail::Outline => 0,
        Detail::Summary => 3,
        _ => usize::MAX,
    };
    let app = traces.app;
    let inherits = app.property_inherits(property);
    let own = traces.of(id).clone();
    let mut out = match own.winner(property) {
        Some(w) => {
            let over = own.overridden(property);
            let mut out = info(property, w.value.clone(), "declared");
            out.declaration = Some(decl_info(w));
            out.overridden = over.iter().take(keep).map(|d| decl_info(d)).collect();
            out.more_overridden = over.len().saturating_sub(keep);
            match css_wide_keyword(&w.value) {
                None => out,
                Some(k) => resolve_keyword(traces, id, property, k, w, &over, inherits, out),
            }
        }
        None if inherits => inherited(traces, id, property),
        None => info(property, "initial".to_string(), "initial"),
    };
    // A custom property's computed value: its `var()` references resolved.
    if property.starts_with("--") {
        out.computed = app
            .dom()
            .node(id)
            .computed()
            .and_then(|c| {
                c.vars
                    .get(property.trim_start_matches("--"))
                    .or_else(|| c.vars.get(property))
            })
            .map(|v| v.as_str().trim().to_string());
    }
    out
}

/// A winning CSS-wide keyword (CSS Cascade 4 §7.3), resolved.
#[allow(clippy::too_many_arguments)]
fn resolve_keyword(
    traces: &mut Traces<'_>,
    id: NodeId,
    property: &str,
    keyword: &'static str,
    winner: &Declared,
    over: &[&Declared],
    inherits: bool,
    declared: PropertyInfo,
) -> PropertyInfo {
    let mut out = match keyword {
        "inherit" => inherited(traces, id, property),
        "unset" if inherits => inherited(traces, id, property),
        "initial" | "unset" => info(property, "initial".to_string(), "initial"),
        // An author, hint or inline `revert` rolls back to the UA's
        // declaration; a UA `revert` to none at all (§7.3.1).
        "revert" => {
            let ua = (winner.origin != DeclarationOrigin::UserAgent)
                .then(|| {
                    over.iter()
                        .find(|d| d.origin == DeclarationOrigin::UserAgent)
                })
                .flatten()
                .filter(|d| css_wide_keyword(&d.value).is_none());
            match ua {
                Some(d) => {
                    let mut o = info(property, d.value.clone(), "reverted");
                    o.declaration = Some(decl_info(d));
                    o
                }
                None if inherits => inherited(traces, id, property),
                None => info(property, "initial".to_string(), "initial"),
            }
        }
        // `revert-layer` rolls back to the cascade without the
        // declaration's layer and those above it; the trace does not say
        // which layer each declaration is in.
        _ => {
            let mut o = declared.clone();
            o.note = Some("rolled back to an earlier cascade layer; not resolved".to_string());
            o
        }
    };
    out.keyword = Some(keyword);
    if out.declaration.is_none() || out.from == "reverted" {
        out.overridden = declared.overridden;
        out.more_overridden = declared.more_overridden;
    }
    if out.from != "reverted" && keyword != "revert-layer" {
        // The keyword's own declaration explains why.
        out.note = Some(format!("{keyword} declared by {}", describe(winner)));
    }
    out
}

fn describe(d: &Declared) -> String {
    let i = decl_info(d);
    match (i.selector, i.source) {
        (Some(sel), Some(src)) => format!("{sel} ({src})"),
        (Some(sel), None) => sel,
        _ => i.origin.to_string(),
    }
}

/// `property`'s inherited value on `id`: up the elements it inherits from,
/// the first declaration that is not itself a keyword passing it on.
fn inherited(traces: &mut Traces<'_>, id: NodeId, property: &str) -> PropertyInfo {
    let dom = traces.app.dom();
    let mut cur = inheritance_parent(dom, id);
    while let Some(from) = cur {
        let a = match from {
            Inheritance::Element(a) => a,
            Inheritance::DetailsContent(d) => {
                let mut o = info(property, "unknown".to_string(), "inherited");
                o.ancestor = Some(node_key(d));
                o.note = Some("inherited from the <details> element's ::details-content slot, whose own declarations are not traced".to_string());
                return o;
            }
            _ => break,
        };
        if let Some(w) = traces.of(a).winner(property).cloned() {
            match css_wide_keyword(&w.value) {
                // Passes its parent's on: keep climbing.
                Some("inherit") | Some("unset") => {}
                Some(_) => {
                    let mut o = info(property, "initial".to_string(), "initial");
                    o.ancestor = Some(node_key(a));
                    o.note = Some(format!("{} on an ancestor", w.value.trim()));
                    return o;
                }
                None => {
                    let mut o = info(property, w.value.clone(), "inherited");
                    o.declaration = Some(decl_info(&w));
                    o.ancestor = Some(node_key(a));
                    return o;
                }
            }
        }
        cur = inheritance_parent(dom, a);
    }
    info(property, "initial".to_string(), "initial")
}
