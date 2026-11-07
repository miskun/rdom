//! CAPTURE phase 5 (`specs/CAPTURE.md` §Rasterizer): an acid page drawn by
//! rdom-capture's rasterizer is the same PNG, byte for byte, on every OS —
//! integer arithmetic, a bundled font, rdom's own encoder — under each
//! profile pinned here. A change to the rasterizer, the font, the encoder,
//! a profile or the page's paint changes the bytes: look at the new image
//! (written beside the golden as `*.actual.png`), then accept it with
//! `UPDATE_SNAPSHOTS=1`.

use std::path::PathBuf;

use rdom_capture::raster::{Raster, render};
use rdom_capture::{CaptureRequest, Environment, Layer, Profile, Session};
use rdom_showcase::capture::Showcase;
use rdom_showcase::demos::acid::{PAGE_HEIGHT, PAGE_WIDTH};

fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/integration/acid/images")
        .join(name)
}

/// Acid page `page` drawn under `profile`, as PNG bytes.
fn acid_png(page: usize, profile: &str) -> (Vec<u8>, Vec<char>) {
    let mut session = Session::new(Showcase);
    let env = Environment::default().with_viewport(PAGE_WIDTH, PAGE_HEIGHT);
    session.open(env, Some(&format!("acid:{page}"))).unwrap();
    session.advance(500).unwrap();
    let capture = session
        .capture(&CaptureRequest::with_layers([Layer::Grid]))
        .unwrap();
    let profile = Profile::named(profile).unwrap();
    let image = render(capture.grid.as_ref().unwrap(), Raster::new(&profile, 1));
    (image.png(), image.missing)
}

/// The glyphs unscii-16 lacks on acid page 1: its CJK sample, drawn as
/// code-point boxes (`TECH_DEBT` CAPTURE-FONT-1).
const PAGE_ONE_MISSING: [char; 2] = ['日', '本'];

fn check(page: usize, profile: &str) {
    let name = format!("page{page}-{profile}.png");
    let path = golden(&name);
    let (actual, missing) = acid_png(page, profile);
    assert_eq!(
        missing, PAGE_ONE_MISSING,
        "{name}: the glyphs the font lacks"
    );
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = std::fs::read(&path).unwrap_or_else(|e| {
        panic!("golden {name} not found ({e}). Run with UPDATE_SNAPSHOTS=1 to create it.")
    });
    if expected != actual {
        let out = path.with_extension("actual.png");
        std::fs::write(&out, &actual).unwrap();
        panic!(
            "{name} differs from its golden ({} bytes, now {}); the new image is {}. Run with UPDATE_SNAPSHOTS=1 to accept it.",
            expected.len(),
            actual.len(),
            out.display()
        );
    }
}

#[test]
fn acid_page_one_is_the_same_png_under_a_dark_profile() {
    check(1, "xterm-dark");
}

#[test]
fn acid_page_one_is_the_same_png_under_a_light_profile() {
    check(1, "solarized-light");
}
