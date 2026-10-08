//! Cascade layers (CSS Cascade 5 §6.4): the layers one stylesheet
//! declares, and the document-wide [`LayerOrder`] the cascade sorts
//! author declarations by.
//!
//! A sheet records its layers in order of first declaration, each as
//! one name segment under a parent (`a.b` is `b` under `a`; an
//! anonymous layer has no name and is never the same as another).
//! [`LayerOrder::new`] merges the layers of a list of sheets — a
//! document's sheets are ordered together, CSSOM §6.2 — by name in
//! sheet order, and ranks them: sibling layers by first declaration, a
//! layer's sublayers before the layer's own rules (§6.4.3), and the
//! unlayered rules last.

use super::{ConditionId, Rule, Stylesheet};

/// A layer declared in one [`Stylesheet`]: an index into
/// [`Stylesheet::layers`]. Meaningful only for the sheet that issued
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayerId(u32);

impl LayerId {
    /// The index into [`Stylesheet::layers`].
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// One declared layer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Layer {
    /// The layer's own name segment (`b` for `a.b`); `None` for an
    /// anonymous layer (`@layer { … }`).
    pub name: Option<String>,
    /// The enclosing layer; `None` at the top level.
    pub parent: Option<LayerId>,
}

impl Stylesheet {
    /// The layers this sheet declares, in order of first declaration.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Declare the layer named by the dotted `path` (`["a", "b"]` for
    /// `a.b`) inside `parent` (`@layer a.b;`, or the prelude of
    /// `@layer a.b { … }`), and return it. Each segment is declared on
    /// its first mention, which fixes its place among its siblings
    /// (CSS Cascade 5 §6.4.2); a later mention finds the same layer.
    /// `None` for an empty path.
    pub fn declare_layer(&mut self, parent: Option<LayerId>, path: &[&str]) -> Option<LayerId> {
        let mut at = parent;
        for segment in path {
            at = Some(self.named_child(at, segment));
        }
        if path.is_empty() { None } else { at }
    }

    /// Declare a new anonymous layer inside `parent` (`@layer { … }`).
    /// It is distinct from every other layer.
    pub fn declare_anonymous_layer(&mut self, parent: Option<LayerId>) -> LayerId {
        self.push_layer(Layer { name: None, parent })
    }

    /// Append `other`'s layers, rules, `@keyframes` and root variables after this
    /// sheet's own, as if its text followed this sheet's: a named layer
    /// `other` declares merges with this sheet's layer of the same name
    /// (so the first declaration in either still fixes the order), an
    /// anonymous one stays apart, and every rule keeps its origin and
    /// layer and takes the next source index.
    pub fn append(&mut self, other: &Stylesheet) {
        // Every collection named, none behind `..`: a field added to
        // `Stylesheet` fails to compile here until `append` carries it (as
        // `@keyframes` once went missing, C13G-MISC).
        let Stylesheet {
            rules,
            index: _,           // rebuilt from the appended rules
            next_source_idx: _, // each appended rule takes the next of ours
            root_vars,
            layers,
            registrations,
            counter_styles,
            keyframes,
            imports,
            scopes: _,     // `append_scopes` maps them
            conditions: _, // `append_conditions` maps them
            media: _,      // `append_conditions` makes it a condition
            owner_node: _, // the receiver's own `<style>` stays its owner
            version: _,    // `touch` renews ours
        } = other;
        self.touch();
        let mut map: Vec<LayerId> = Vec::with_capacity(layers.len());
        for layer in layers {
            let parent = layer.parent.map(|p| map[p.index()]);
            let id = match &layer.name {
                Some(name) => self.named_child(parent, name),
                None => self.declare_anonymous_layer(parent),
            };
            map.push(id);
        }
        let scopes = self.append_scopes(other);
        let (conditions, root) = self.append_conditions(other);
        let condition = |c: Option<ConditionId>| c.map(|c| conditions[c.index()]).or(root);
        self.registrations.extend(registrations.iter().cloned());
        for def in counter_styles {
            let mut def = def.clone();
            def.layer = def.layer.map(|l| map[l.index()]);
            def.condition = condition(def.condition);
            self.counter_styles.push(def);
        }
        for import in imports {
            let mut import = import.clone();
            import.layer = import.layer.map(|l| map[l.index()]);
            self.imports.push(import);
        }
        for rule in keyframes {
            let mut rule = rule.clone();
            rule.layer = rule.layer.map(|l| map[l.index()]);
            rule.condition = condition(rule.condition);
            self.keyframes.push(rule);
        }
        let rules: Vec<Rule> = rules
            .iter()
            .map(|rule| {
                let mut rule = rule.clone();
                rule.layer = rule.layer.map(|l| map[l.index()]);
                rule.scope = rule.scope.map(|s| scopes[s.index()]);
                rule.condition = condition(rule.condition);
                rule.source_idx = self.next_source_idx;
                self.next_source_idx += 1;
                rule
            })
            .collect();
        self.push_rules(rules);
        for (name, value) in root_vars {
            self.root_vars.insert(name.clone(), value.clone());
        }
    }

    fn named_child(&mut self, parent: Option<LayerId>, name: &str) -> LayerId {
        let found = self
            .layers
            .iter()
            .position(|l| l.parent == parent && l.name.as_deref() == Some(name));
        match found {
            Some(i) => LayerId(i as u32),
            None => self.push_layer(Layer {
                name: Some(name.to_string()),
                parent,
            }),
        }
    }

    fn push_layer(&mut self, layer: Layer) -> LayerId {
        self.touch();
        self.layers.push(layer);
        LayerId(self.layers.len() as u32 - 1)
    }
}

/// The cascade-layer order of a list of sheets: for each sheet and
/// layer, its rank — lower ranks lose to higher ones for normal
/// declarations, and win for `!important` ones (CSS Cascade 5 §6.4).
#[derive(Debug, Clone, Default)]
pub struct LayerOrder {
    /// `ranks[sheet][layer]`.
    ranks: Vec<Vec<u32>>,
}

impl LayerOrder {
    /// The rank of unlayered declarations: above every layer.
    pub const UNLAYERED: u32 = u32::MAX;

    /// Merge and rank the layers of `sheets`, in the order the sheets
    /// cascade.
    pub fn new(sheets: &[&Stylesheet]) -> Self {
        // A tree of every distinct layer; node 0 is the unlayered root.
        let mut names: Vec<Option<String>> = vec![None];
        let mut children: Vec<Vec<usize>> = vec![Vec::new()];
        let mut nodes: Vec<Vec<usize>> = Vec::with_capacity(sheets.len());
        for sheet in sheets {
            let mut local: Vec<usize> = Vec::with_capacity(sheet.layers.len());
            for layer in &sheet.layers {
                let parent = layer.parent.map_or(0, |p| local[p.index()]);
                let existing = layer.name.as_ref().and_then(|name| {
                    children[parent]
                        .iter()
                        .copied()
                        .find(|&c| names[c].as_ref() == Some(name))
                });
                let node = existing.unwrap_or_else(|| {
                    names.push(layer.name.clone());
                    children.push(Vec::new());
                    let node = names.len() - 1;
                    children[parent].push(node);
                    node
                });
                local.push(node);
            }
            nodes.push(local);
        }
        // Post-order: a layer's sublayers rank below its own rules.
        let mut rank = vec![Self::UNLAYERED; names.len()];
        let mut next = 0u32;
        let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
        while let Some((node, child)) = stack.pop() {
            if let Some(&c) = children[node].get(child) {
                stack.push((node, child + 1));
                stack.push((c, 0));
            } else if node != 0 {
                rank[node] = next;
                next += 1;
            }
        }
        LayerOrder {
            ranks: nodes
                .into_iter()
                .map(|local| local.into_iter().map(|n| rank[n]).collect())
                .collect(),
        }
    }

    /// The rank of `layer` of sheet `sheet` ([`Self::UNLAYERED`] for
    /// `None`).
    pub fn rank(&self, sheet: usize, layer: Option<LayerId>) -> u32 {
        match layer {
            None => Self::UNLAYERED,
            Some(l) => self.ranks[sheet][l.index()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TuiStyle;

    fn ranks(sheets: &[&Stylesheet]) -> Vec<Vec<u32>> {
        LayerOrder::new(sheets).ranks
    }

    /// §6.4.2: layers rank by first declaration; a later mention of a
    /// name is the same layer.
    #[test]
    fn layers_rank_by_first_declaration() {
        let mut s = Stylesheet::bare();
        let b = s.declare_layer(None, &["b"]).unwrap();
        let a = s.declare_layer(None, &["a"]).unwrap();
        assert_eq!(s.declare_layer(None, &["b"]), Some(b));
        let order = LayerOrder::new(&[&s]);
        assert!(order.rank(0, Some(b)) < order.rank(0, Some(a)));
        assert!(order.rank(0, Some(a)) < order.rank(0, None));
    }

    /// §6.4.3: `a.b` is `b` nested in `a`, and a layer's sublayers rank
    /// below the layer's own rules but above earlier siblings of `a`.
    #[test]
    fn sublayers_rank_below_their_parent() {
        let mut s = Stylesheet::bare();
        let x = s.declare_layer(None, &["x"]).unwrap();
        let ab = s.declare_layer(None, &["a", "b"]).unwrap();
        let a = s.declare_layer(None, &["a"]).unwrap();
        let ac = s.declare_layer(Some(a), &["c"]).unwrap();
        assert_eq!(s.layers()[ab.index()].parent, Some(a));
        assert_eq!(ranks(&[&s]), vec![vec![0, 3, 1, 2]]);
        let _ = (x, ac);
    }

    /// Anonymous layers are distinct; named layers merge across sheets
    /// in sheet order.
    #[test]
    fn sheets_share_one_layer_order() {
        let mut first = Stylesheet::bare();
        first.declare_layer(None, &["b"]);
        first.declare_anonymous_layer(None);
        let mut second = Stylesheet::bare();
        second.declare_layer(None, &["a"]);
        second.declare_layer(None, &["b"]);
        second.declare_anonymous_layer(None);
        // b, anon1, a, anon2.
        assert_eq!(ranks(&[&first, &second]), vec![vec![0, 1], vec![2, 0, 3]]);
    }

    /// `append` keeps layers and merges named ones.
    #[test]
    fn append_maps_layers() {
        let mut base = Stylesheet::bare();
        let b = base.declare_layer(None, &["b"]);
        let mut other = Stylesheet::bare();
        let ob = other.declare_layer(None, &["b"]);
        let oa = other.declare_layer(None, &["a"]);
        let in_layer = |layer| crate::RuleContext::default().in_layer(layer);
        let p = crate::StyleSelector::parse("p").unwrap();
        let q = crate::StyleSelector::parse("q").unwrap();
        other.add_style_rule(&p, TuiStyle::new(), in_layer(oa));
        other.add_style_rule(&q, TuiStyle::new(), in_layer(ob));
        base.append(&other);
        assert_eq!(base.layers().len(), 2);
        assert_eq!(base.rules()[1].layer, b);
        assert_eq!(
            base.layers()[base.rules()[0].layer.unwrap().index()]
                .name
                .as_deref(),
            Some("a")
        );
    }

    /// C12G-README-ANIM: `append` carries `other`'s `@keyframes` rules
    /// (CSS Animations 1 §3), each in its layer as mapped — it dropped
    /// them, so a sheet built by `rdom_css::from_css_strict` (which
    /// appends the parse) had no keyframes and its animations never ran.
    #[test]
    fn append_carries_keyframes() {
        let mut base = Stylesheet::bare();
        let b = base.declare_layer(None, &["b"]);
        let mut other = Stylesheet::bare();
        let ob = other.declare_layer(None, &["b"]);
        other.define_keyframes(crate::keyframes::KeyframesRule::new("spin").in_layer(ob));
        base.append(&other);
        assert_eq!(base.keyframes().len(), 1);
        assert_eq!(&*base.keyframes()[0].name, "spin");
        assert_eq!(base.keyframes()[0].layer, b);
    }
}
