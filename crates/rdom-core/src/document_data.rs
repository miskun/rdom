//! Backend state attached to a document.
//!
//! A backend sometimes needs state that belongs to the document as a
//! whole rather than to one node — the size of the viewport the
//! document is presented in, which its style resolves `vw` / `vh`
//! against (CSS Values 4 §6.1.2), is rdom-tui's. Per-node state goes in
//! the `Ext` type; per-document state goes here, one value per Rust
//! type, so the substrate stays ignorant of what a backend stores
//! (the same shape as `http::Extensions`). The substrate itself never
//! reads it.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::dom::Dom;

/// The per-document values, keyed by type.
#[derive(Default)]
pub(crate) struct DocumentData(HashMap<TypeId, Box<dyn Any>>);

impl std::fmt::Debug for DocumentData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DocumentData({} values)", self.0.len())
    }
}

impl<Ext: 'static> Dom<Ext> {
    /// The document's value of type `T`, if one was set
    /// ([`set_document_data`](Self::set_document_data)).
    pub fn document_data<T: 'static>(&self) -> Option<&T> {
        self.document_data
            .0
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref())
    }

    /// The document's value of type `T`, mutably.
    pub fn document_data_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.document_data
            .0
            .get_mut(&TypeId::of::<T>())
            .and_then(|v| v.downcast_mut())
    }

    /// Set the document's value of type `T`, returning the previous one.
    pub fn set_document_data<T: 'static>(&mut self, value: T) -> Option<T> {
        self.document_data
            .0
            .insert(TypeId::of::<T>(), Box::new(value))
            .and_then(|old| old.downcast().ok())
            .map(|old| *old)
    }

    /// Remove the document's value of type `T`, returning it.
    pub fn remove_document_data<T: 'static>(&mut self) -> Option<T> {
        self.document_data
            .0
            .remove(&TypeId::of::<T>())
            .and_then(|old| old.downcast().ok())
            .map(|old| *old)
    }
}

#[cfg(test)]
mod tests {
    use crate::Dom;

    #[derive(Debug, PartialEq)]
    struct Size(u16, u16);

    #[derive(Debug, PartialEq)]
    struct Theme(&'static str);

    /// One value per type: setting replaces (and returns) the previous
    /// value of that type and leaves the others alone.
    #[test]
    fn values_are_keyed_by_type() {
        let mut dom: Dom = Dom::new();
        assert_eq!(dom.document_data::<Size>(), None);
        assert_eq!(dom.set_document_data(Size(80, 24)), None);
        assert_eq!(dom.set_document_data(Theme("dark")), None);
        assert_eq!(dom.set_document_data(Size(40, 10)), Some(Size(80, 24)));
        assert_eq!(dom.document_data::<Size>(), Some(&Size(40, 10)));
        assert_eq!(dom.document_data::<Theme>(), Some(&Theme("dark")));
        dom.document_data_mut::<Size>().unwrap().0 = 20;
        assert_eq!(dom.remove_document_data::<Size>(), Some(Size(20, 10)));
        assert_eq!(dom.document_data::<Size>(), None);
        assert_eq!(dom.document_data::<Theme>(), Some(&Theme("dark")));
    }

    /// The data belongs to its document: another `Dom` does not see it,
    /// and a tree mutation does not touch it.
    #[test]
    fn data_is_per_document() {
        let mut a: Dom = Dom::new();
        let b: Dom = Dom::new();
        a.set_document_data(Size(1, 2));
        let div = a.create_element("div");
        a.append_child(a.root(), div).unwrap();
        a.remove_child(a.root(), div).unwrap();
        assert_eq!(a.document_data::<Size>(), Some(&Size(1, 2)));
        assert_eq!(b.document_data::<Size>(), None);
    }
}
