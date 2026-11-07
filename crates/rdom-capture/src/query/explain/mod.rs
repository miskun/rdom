//! `explain` (`specs/CAPTURE.md` §Paint trace, §Derivation chain): why a
//! cell shows what it shows, or why an element looks as it does — the
//! paint operations that made the cell, the element that decided it, and,
//! for the properties that part of its paint reads, the declaration each
//! took, the ones it overrode, and where they are written; for a property
//! with no declaration, the ancestor it inherits from.

use rdom_tui::capture::{DeclarationOrigin, Declared, OpKind, PaintOp, PaintPart, RuleRef};
use rdom_tui::core_api::NodeId;
use rdom_tui::render::TestBackend;
use rdom_tui::{App, Rect};
use serde::Serialize;

mod property;

use property::{Traces, property};

use super::Detail;
use crate::capture::{Attr, Boxes, ColorValue, NodeInfo, node_key, nodes};

/// What `explain` answers.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Explanation {
    pub target: Target,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell: Option<CellInfo>,
    /// The paint operations on the cell, in order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paint: Vec<OpInfo>,
    /// How the element's styles came to be, property by property.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub style: Vec<PropertyInfo>,
    /// The animations and transitions running on it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub animations: Vec<String>,
}

/// The element explained.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Target {
    pub node: String,
    #[serde(flatten)]
    pub info: NodeInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boxes: Option<Boxes>,
    /// The part of its paint the cell shows, for `at`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    /// The element whose layer the paint reached the frame through.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
}

/// A cell as it shows.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct CellInfo {
    pub x: u16,
    pub y: u16,
    pub glyph: String,
    pub fg: ColorValue,
    pub bg: ColorValue,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mods: Vec<Attr>,
}

/// One paint operation, for a person or an agent.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpInfo {
    /// Layer depth: 0 on the frame.
    pub depth: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub part: &'static str,
    /// `write`, `border`, `blend`, `join`.
    pub op: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f32>,
    pub glyph: String,
    pub fg: ColorValue,
    pub bg: ColorValue,
    /// For a join: the elements whose borders met.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub joined: Vec<String>,
}

/// How one property's value came to be.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct PropertyInfo {
    pub property: String,
    /// The value it takes: the winning declaration's, an ancestor's, or
    /// `initial`.
    pub value: String,
    /// `declared`, `inherited`, `initial`, `reverted` (a `revert` rolled
    /// back to the UA's declaration).
    pub from: &'static str,
    /// The CSS-wide keyword the winning declaration was (`inherit`,
    /// `revert`, …); `value` is what it resolved to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<&'static str>,
    /// What the explanation could not resolve, or why.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// A custom property's computed value: its `var()` references
    /// resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration: Option<DeclInfo>,
    /// For an inherited value: the ancestor that declared it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ancestor: Option<String>,
    /// The declarations the winner overrode, latest first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub overridden: Vec<DeclInfo>,
    /// How many more were overridden than are listed.
    #[serde(skip_serializing_if = "is_zero")]
    pub more_overridden: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// One declaration and where it is written.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeclInfo {
    pub value: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub important: bool,
    /// `ua`, `author`, `hint`, `inline`.
    pub origin: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specificity: Option<[u16; 3]>,
    /// `file:line:column`, `rust:file:line:column`, `<style #12>:line`, …
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The rule as `edit` takes it: `[sheet, index]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<[usize; 2]>,
}

fn origin_name(o: DeclarationOrigin) -> &'static str {
    match o {
        DeclarationOrigin::UserAgent => "ua",
        DeclarationOrigin::Author => "author",
        DeclarationOrigin::Hint => "hint",
        DeclarationOrigin::Inline => "inline",
        _ => "other",
    }
}

/// Where `rule` is written, as one string.
fn source_of(rule: &RuleRef) -> Option<String> {
    let l = rule.location?;
    let at = format!("{}:{}", l.line, l.column);
    Some(match l.file {
        rdom_tui::SourceFile::Rust(file) => format!("rust:{file}:{at}"),
        rdom_tui::SourceFile::Css(_) => match (&rule.source_name, rule.owner_node) {
            (Some(name), _) => format!("{name}:{at}"),
            (None, Some(owner)) => format!("<style #{}>:{at}", node_key(owner)),
            (None, None) => format!("<css>:{at}"),
        },
        _ => return None,
    })
}

fn decl_info(d: &Declared) -> DeclInfo {
    let r = d.rule.as_ref();
    DeclInfo {
        value: d.value.clone(),
        important: d.important,
        origin: origin_name(d.origin),
        selector: r.map(|r| r.selector.clone()),
        specificity: r.map(|r| {
            let s = r.specificity;
            [s.id, s.class_attr_pseudo, s.type_pseudo_el]
        }),
        source: r.and_then(source_of),
        rule: r.map(|r| [r.sheet, r.index]),
    }
}

/// The properties a part of an element's paint reads.
pub(crate) fn relevant(part: Option<PaintPart>) -> &'static [&'static str] {
    const TEXT: &[&str] = &[
        "color",
        "background-color",
        "font-weight",
        "font-style",
        "text-decoration-line",
        "text-decoration-color",
        "text-decoration-style",
        "opacity",
        "visibility",
    ];
    const BG: &[&str] = &["background-color", "opacity", "visibility"];
    const BORDER: &[&str] = &[
        "border-top-style",
        "border-right-style",
        "border-bottom-style",
        "border-left-style",
        "border-top-color",
        "border-right-color",
        "border-bottom-color",
        "border-left-color",
        "border-top-width",
        "border-collapse",
        "corner-shape",
        "opacity",
    ];
    const OUTLINE: &[&str] = &[
        "outline-style",
        "outline-color",
        "outline-width",
        "outline-offset",
    ];
    const GROUP: &[&str] = &["opacity", "filter", "mix-blend-mode", "isolation"];
    const SCROLL: &[&str] = &[
        "scrollbar-color",
        "scrollbar-width",
        "overflow-x",
        "overflow-y",
    ];
    match part {
        Some(
            PaintPart::Background | PaintPart::Canvas | PaintPart::Backdrop | PaintPart::Shadow,
        ) => BG,
        Some(PaintPart::Border | PaintPart::BorderJoin | PaintPart::ColumnRule) => BORDER,
        Some(PaintPart::Outline) => OUTLINE,
        Some(PaintPart::Group) => GROUP,
        Some(PaintPart::Scrollbar) => SCROLL,
        _ => TEXT,
    }
}

fn op_info(app: &App<TestBackend>, op: &PaintOp) -> OpInfo {
    let dom = app.dom();
    let (op_name, alpha) = match op.kind {
        OpKind::Write => ("write", None),
        OpKind::Border => ("border", None),
        OpKind::Blend { alpha } => ("blend", Some(alpha)),
        OpKind::Join => ("join", None),
        _ => ("other", None),
    };
    OpInfo {
        depth: op.depth,
        node: op.node.map(node_key),
        path: op.node.map(|n| nodes::node_info(dom, n).path),
        part: op.part.name(),
        op: op_name,
        alpha,
        glyph: if op.cell.is_spacer() {
            String::new()
        } else {
            op.cell.symbol().to_string()
        },
        fg: op.cell.fg.into(),
        bg: op.cell.bg.into(),
        joined: op.joined.iter().map(|n| node_key(*n)).collect(),
    }
}

/// Explain element `id`: its node, boxes, animations, and the properties
/// `properties` (all its declared ones when empty).
pub(crate) fn element(
    app: &App<TestBackend>,
    id: NodeId,
    properties: &[String],
    part: Option<PaintPart>,
    detail: Detail,
) -> Explanation {
    let dom = app.dom();
    let mut traces = Traces::new(app);
    let own = traces.of(id).clone();
    let names: Vec<String> = if !properties.is_empty() {
        properties.to_vec()
    } else if part.is_some() {
        relevant(part).iter().map(|s| (*s).to_string()).collect()
    } else {
        own.properties().into_iter().map(str::to_string).collect()
    };
    let style = names
        .iter()
        .map(|p| property(&mut traces, id, p, detail))
        .collect();
    let animations = app
        .get_animations(id)
        .iter()
        .map(|a| format!("{:?}", a.kind()))
        .collect();
    Explanation {
        target: Target {
            node: node_key(id),
            info: nodes::node_info(dom, id),
            boxes: rdom_tui::capture::box_model(dom, id).map(Boxes::from),
            part: part.map(|p| p.name().to_string()),
            via: None,
        },
        cell: None,
        paint: Vec::new(),
        style,
        animations,
    }
}

/// Explain the cell at `(x, y)`: its paint operations and the element
/// whose paint it shows, with the properties that part reads (or
/// `properties`). `None` when the cell is off screen or nothing painted it.
pub(crate) fn cell(
    app: &App<TestBackend>,
    x: u16,
    y: u16,
    properties: &[String],
    detail: Detail,
) -> Option<Explanation> {
    let area = app.terminal().buffer().area;
    if !area.contains(x, y) {
        return None;
    }
    let trace = app.trace_paint(Rect::new(x, y, 1, 1));
    let painter = trace.painter_at(x, y)?;
    let via = trace
        .top_at(x, y)
        .filter(|top| painter.depth > 0 && top.node != painter.node)
        .and_then(|top| top.node)
        .map(node_key);
    // The joiner's glyph is the borders' that met there: explain the
    // first of them.
    let id = painter.node.or_else(|| painter.joined.first().copied())?;
    let mut out = element(app, id, properties, Some(painter.part), detail);
    out.target.via = via;
    let shown = app.terminal().buffer().cell(x, y)?;
    out.cell = Some(CellInfo {
        x,
        y,
        glyph: if shown.is_spacer() {
            String::new()
        } else {
            shown.symbol().to_string()
        },
        fg: shown.fg.into(),
        bg: shown.bg.into(),
        mods: Attr::of(shown.modifier),
    });
    out.paint = trace.ops_at(x, y).map(|op| op_info(app, op)).collect();
    if detail == Detail::Outline {
        out.paint.retain(|op| op.depth == 0 && op.op != "border");
    }
    Some(out)
}
