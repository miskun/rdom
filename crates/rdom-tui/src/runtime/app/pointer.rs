//! The terminal pointer's shape (CSS UI 4 §4.1 `cursor`,
//! `runtime::pointer_shape`): the `App` keeps where the pointer is and
//! which shape it last sent, and sends the shape under the pointer after
//! each pointer event and each frame when it changed.

use super::App;
use crate::render::backend::Backend;
use crate::runtime::pointer_shape::{self, PointerShapes};

/// The pointer state of an [`App`].
#[derive(Debug, Default)]
pub(super) struct Pointer {
    /// The protocol the terminal takes.
    shapes: PointerShapes,
    /// Where the pointer was at the last mouse event.
    at: Option<(u16, u16)>,
    /// The shape last sent.
    shown: Option<&'static str>,
}

impl<B: Backend> App<B> {
    /// Send the pointer shape `cursor` asks for through `shapes` —
    /// overriding [`App::new`]'s guess from the environment
    /// ([`PointerShapes::from_env`]), a test backend's
    /// [`None`](PointerShapes::None). With `None` nothing is sent.
    pub fn with_pointer_shapes(mut self, shapes: PointerShapes) -> Self {
        self.pointer.shapes = shapes;
        self.pointer.shown = None;
        self
    }

    /// The pointer-shape protocol the app sends with.
    pub fn pointer_shapes(&self) -> PointerShapes {
        self.pointer.shapes
    }

    /// A mouse event at `(x, y)`: the pointer is there.
    pub(super) fn note_pointer(&mut self, x: u16, y: u16) {
        self.pointer.at = Some((x, y));
    }

    /// Send the shape under the pointer when it differs from the one
    /// sent last. A write error is the terminal's and leaves the shape
    /// to be sent again next time.
    pub(super) fn update_pointer_shape(&mut self) {
        if self.pointer.shapes != PointerShapes::Osc22 {
            return;
        }
        let Some((x, y)) = self.pointer.at else {
            return;
        };
        let shape = pointer_shape::shape_at(&self.dom, x, y);
        if self.pointer.shown == Some(shape) {
            return;
        }
        let backend = self.terminal.backend_mut();
        let sent = backend
            .write_all(&pointer_shape::osc22(shape))
            .and_then(|()| backend.flush());
        if sent.is_ok() {
            pointer_shape::note_shape_sent();
            self.pointer.shown = Some(shape);
        }
    }
}
