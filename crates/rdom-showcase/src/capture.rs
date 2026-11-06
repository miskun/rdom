//! The showcase as a capture app (`specs/CAPTURE.md`): every demo, in the
//! shell the binary shows, and every acid page, as the acid tests paint
//! it — each a fixture a capture session opens by name.
//!
//! - `demo:<slug>` — the showcase with that demo mounted (`--list` names
//!   the slugs); no fixture opens the first demo.
//! - `acid:<page>` — acid page `<page>` alone, with both acid sheets and
//!   its load scripts run after the first frame, at the 120 × 50 the
//!   references are derived at.
//!
//! The `rdom-showcase-capture` binary serves it over stdio.

use std::error::Error;

use rdom_capture::{CaptureApp, Environment, FixtureInfo};
use rdom_tui::App;
use rdom_tui::TuiDom;
use rdom_tui::render::TestBackend;

use crate::demos::acid::{self, PAGE_HEIGHT, PAGE_WIDTH};
use crate::shell::base_stylesheet;
use crate::{DEMOS, assemble};

/// The showcase's capture app.
pub struct Showcase;

/// What a fixture name opens.
enum Fixture {
    Demo(usize),
    Acid(u8),
}

fn parse(fixture: Option<&str>) -> Result<Fixture, String> {
    let Some(name) = fixture else {
        return Ok(Fixture::Demo(0));
    };
    if let Some(slug) = name.strip_prefix("demo:") {
        return DEMOS
            .iter()
            .position(|d| d.slug() == slug)
            .map(Fixture::Demo)
            .ok_or_else(|| format!("no demo {slug:?}"));
    }
    if let Some(page) = name.strip_prefix("acid:") {
        return page
            .parse::<u8>()
            .ok()
            .filter(|p| (1..=acid::page_count()).contains(p))
            .map(Fixture::Acid)
            .ok_or_else(|| format!("no acid page {page:?} (1–{})", acid::page_count()));
    }
    Err(format!("no fixture {name:?}"))
}

impl CaptureApp for Showcase {
    fn build(
        &self,
        env: &Environment,
        fixture: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        match parse(fixture)? {
            Fixture::Demo(idx) => {
                let mut dom = TuiDom::new();
                let state = assemble(&mut dom, idx);
                let mut app = env.app(dom, base_stylesheet())?;
                state.borrow_mut().attach_sheet(&mut app);
                Ok(app)
            }
            Fixture::Acid(page) => {
                let mut dom = TuiDom::new();
                let root = dom.root();
                let page_root = acid::build_page(&mut dom, page);
                dom.append_child(root, page_root)?;
                let mut app = env
                    .app(dom, acid::stylesheet())?
                    .with_import_loader(acid::import_loader());
                app.push_stylesheet(acid::late_stylesheet());
                Ok(app)
            }
        }
    }

    fn after_first_frame(&self, app: &mut App<TestBackend>, fixture: Option<&str>) {
        if let Ok(Fixture::Acid(page)) = parse(fixture) {
            acid::run_scripts(app.dom_mut(), page);
        }
    }

    fn fixtures(&self) -> Vec<FixtureInfo> {
        let demos = DEMOS
            .iter()
            .map(|d| FixtureInfo::new(format!("demo:{}", d.slug()), d.title()));
        let pages = (1..=acid::page_count()).map(|p| {
            FixtureInfo::new(format!("acid:{p}"), format!("acid test page {p}"))
                .with_viewport(PAGE_WIDTH, PAGE_HEIGHT)
        });
        demos.chain(pages).collect()
    }
}
