//! Unit tests for [`VirtualScreen`](super::VirtualScreen): `parser`
//! drives the emulator with hand-written ANSI streams; `terminal`
//! feeds it real `Terminal` + backend output, including the resize
//! regressions it exists to catch.

mod parser;
mod terminal;
