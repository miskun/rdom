//! Views of a capture (`specs/CAPTURE.md` §Views). Each is a pure
//! function of a [`Capture`](crate::Capture) or one of its layers.

pub(crate) mod base64;
pub(crate) mod html;
pub mod text;
