//! The stage-2 driver (`specs/ACID.md` Stage 2): a [`Session`] is one
//! acid page in a fresh 120 × 50 headless `App`, driven by scripted input
//! — keys, mouse, wheel, resize — on a controllable clock
//! (`App::advance`), with the screen read back the way a terminal shows
//! it after every action.
//!
//! ## Fresh pages, chained checkpoints
//!
//! Every [`Step`] opens its page anew — built, styled, its tiles' load
//! scripts run, two frames drawn, exactly as stage 1 paints it — so a step
//! starts from the state the static references describe, and no step
//! inherits another's hover, focus, scroll offset or clock. Acid3 chains
//! its tests through one document; here a failing step must not move the
//! next one's starting point, so isolation wins. Inside a step the
//! checkpoints do chain: a step is one user story (press, then release;
//! type, then Tab away), and each checkpoint reads the state the actions
//! before it left.
//!
//! ## Checkpoints
//!
//! [`Session::expect`] compares tiles of the page against per-step
//! [`Reference`]s — the stage-1 format, derived by hand from the spec —
//! and [`Session::check`] records a fact no cell shows (an event order, a
//! hit target). Failures are collected, not panicked, so one wrong
//! checkpoint never hides the next; [`Session::finish`] reports them all,
//! each naming the step, the checkpoint, the tile, the cells and the spec.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use rdom_showcase::demos::acid::{self, PAGE_HEIGHT, PAGE_WIDTH, TILES, Tile};
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::{NodeId, SgrCapabilities, TuiDom, VirtualScreen};

use super::super::compare::check_on;
use super::super::reference::Reference;

/// One stage-2 step: a scripted interaction on one acid page.
pub struct Step {
    /// `ACID.md`'s step number (`"I1"`).
    pub id: &'static str,
    pub title: &'static str,
    /// The page the step drives.
    pub page: u8,
    /// The spec sections the step's checks follow.
    pub spec: &'static [&'static str],
    /// The script: actions and checkpoints, in order.
    pub run: fn(&mut Session),
}

/// One acid page in a headless `App`, driven by a [`Step`]'s script.
pub struct Session {
    pub step: &'static Step,
    app: App<TestBackend>,
    screen: VirtualScreen,
    /// Checkpoints run, paint and fact checks together.
    checkpoints: usize,
    failures: Vec<String>,
}

fn tile(id: &str) -> &'static Tile {
    TILES
        .iter()
        .copied()
        .find(|t| t.id == id)
        .unwrap_or_else(|| panic!("no acid tile {id:?}"))
}

impl Session {
    /// Open `step`'s page fresh, as stage 1 paints it: both sheets, the
    /// first frame at the clock's 0, the tiles' load scripts, a second
    /// frame.
    pub fn open(step: &'static Step) -> Session {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let page_root = acid::build_page(&mut dom, step.page);
        dom.append_child(root, page_root).unwrap();
        let terminal = Terminal::new(TestBackend::new(PAGE_WIDTH, PAGE_HEIGHT)).unwrap();
        let mut app = App::with_backend(dom, acid::stylesheet(), terminal)
            .unwrap()
            .with_sgr_capabilities(SgrCapabilities::EXTENDED)
            .with_import_loader(acid::import_loader());
        app.push_stylesheet(acid::late_stylesheet());
        app.advance(0).unwrap();
        acid::run_scripts(app.dom_mut(), step.page);
        let mut session = Session {
            step,
            app,
            screen: VirtualScreen::new(PAGE_WIDTH, PAGE_HEIGHT),
            checkpoints: 0,
            failures: Vec::new(),
        };
        session.settle();
        session
    }

    /// Finish the loop iteration (`advance(0)`) and replay what it drew.
    fn settle(&mut self) {
        self.app.advance(0).unwrap();
        self.read_back();
    }

    /// Replay the terminal bytes written since the last read.
    fn read_back(&mut self) {
        let bytes = self.app.terminal_mut().backend_mut().take_bytes();
        self.screen.apply(&bytes);
    }

    // ── geometry ──

    /// The page cell at `(dx, dy)` of tile `id`'s box.
    pub fn at(&self, id: &str, dx: u16, dy: u16) -> (u16, u16) {
        let t = tile(id);
        assert!(
            t.page == self.step.page,
            "tile {id} is not on page {}",
            self.step.page
        );
        (t.x + dx, t.y + dy)
    }

    /// The element `selector` matches inside tile `id`'s box.
    pub fn find(&self, id: &str, selector: &str) -> NodeId {
        let t = tile(id);
        let dom = self.app.dom();
        let tile_box = dom
            .query_selector_in(dom.root(), &format!(".{}", t.class))
            .expect("a valid selector")
            .unwrap_or_else(|| panic!("tile {id} is on the page"));
        dom.query_selector_in(tile_box, selector)
            .expect("a valid selector")
            .unwrap_or_else(|| panic!("tile {id} holds {selector:?}"))
    }

    pub fn dom(&self) -> &TuiDom {
        self.app.dom()
    }

    pub fn app(&mut self) -> &mut App<TestBackend> {
        &mut self.app
    }

    pub fn screen(&self) -> &VirtualScreen {
        &self.screen
    }

    // ── input ──

    fn mouse(&mut self, kind: MouseEventKind, (column, row): (u16, u16)) {
        self.app.handle_event(CtEvent::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::empty(),
        }));
        self.settle();
    }

    /// Move the pointer to `(dx, dy)` of tile `id`.
    pub fn hover(&mut self, id: &str, dx: u16, dy: u16) {
        self.mouse(MouseEventKind::Moved, self.at(id, dx, dy));
    }

    /// Move the pointer to a page cell.
    pub fn hover_page(&mut self, x: u16, y: u16) {
        self.mouse(MouseEventKind::Moved, (x, y));
    }

    /// Press the primary button at `(dx, dy)` of tile `id`.
    pub fn press(&mut self, id: &str, dx: u16, dy: u16) {
        self.mouse(MouseEventKind::Down(MouseButton::Left), self.at(id, dx, dy));
    }

    /// Release the primary button at `(dx, dy)` of tile `id`.
    pub fn release(&mut self, id: &str, dx: u16, dy: u16) {
        self.mouse(MouseEventKind::Up(MouseButton::Left), self.at(id, dx, dy));
    }

    /// Release the primary button at a page cell.
    pub fn release_page(&mut self, x: u16, y: u16) {
        self.mouse(MouseEventKind::Up(MouseButton::Left), (x, y));
    }

    /// Move there, press and release: a click.
    pub fn click(&mut self, id: &str, dx: u16, dy: u16) {
        self.hover(id, dx, dy);
        self.press(id, dx, dy);
        self.release(id, dx, dy);
    }

    /// A click at a page cell.
    pub fn click_page(&mut self, x: u16, y: u16) {
        self.hover_page(x, y);
        self.mouse(MouseEventKind::Down(MouseButton::Left), (x, y));
        self.release_page(x, y);
    }

    /// One wheel tick at `(dx, dy)` of tile `id`, down (`true`) or up.
    pub fn wheel(&mut self, id: &str, dx: u16, dy: u16, down: bool) {
        let kind = if down {
            MouseEventKind::ScrollDown
        } else {
            MouseEventKind::ScrollUp
        };
        self.mouse(kind, self.at(id, dx, dy));
    }

    /// One key press with `modifiers`.
    pub fn key_with(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.app
            .handle_event(CtEvent::Key(KeyEvent::new(code, modifiers)));
        self.settle();
    }

    /// One key press.
    pub fn key(&mut self, code: KeyCode) {
        self.key_with(code, KeyModifiers::empty());
    }

    /// Type `text`, one key per character.
    pub fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.key(KeyCode::Char(c));
        }
    }

    /// Resize the terminal to `width` × `height`: the backend's size, the
    /// screen's, then the resize event and the frame after it (a full
    /// repaint over a cleared terminal).
    pub fn resize(&mut self, width: u16, height: u16) {
        self.app.terminal_mut().backend_mut().resize(width, height);
        self.screen.resize(width, height);
        self.app.handle_event(CtEvent::Resize(width, height));
        self.settle();
    }

    /// Move the clock `ms` forward and draw what came due.
    pub fn advance(&mut self, ms: u64) {
        self.app.advance(ms).unwrap();
        self.read_back();
    }

    /// Run `f` on the document — a page script — then the frame after it.
    pub fn script(&mut self, f: impl FnOnce(&mut TuiDom)) {
        f(self.app.dom_mut());
        self.settle();
    }

    // ── checkpoints ──

    /// Compare each of `refs` (tiles of this page) against the screen:
    /// the checkpoint `at`, in the step's own words.
    pub fn expect(&mut self, at: &str, refs: &[&'static Reference]) {
        self.checkpoints += 1;
        for reference in refs {
            assert!(
                tile(reference.tile).page == self.step.page,
                "step {}: tile {} is not on page {}",
                self.step.id,
                reference.tile,
                self.step.page
            );
            let report = check_on(&self.screen, reference);
            if !report.passed() {
                self.failures.push(format!(
                    "{} — checkpoint {} \"{at}\":\n{}",
                    self.header(),
                    self.checkpoints,
                    report.failure()
                ));
            }
        }
    }

    /// Record a fact no cell shows: `holds`, or a failure naming the
    /// checkpoint, what was wanted, what was seen and the spec.
    pub fn check(&mut self, at: &str, holds: bool, seen: impl std::fmt::Display, spec: &str) {
        self.checkpoints += 1;
        if !holds {
            self.failures.push(format!(
                "{} — checkpoint {} \"{at}\" — spec: {spec}\n  seen: {seen}\n",
                self.header(),
                self.checkpoints,
            ));
        }
    }

    /// [`Self::check`] of an equality.
    pub fn check_eq<T: PartialEq + std::fmt::Debug>(
        &mut self,
        at: &str,
        seen: T,
        want: T,
        spec: &str,
    ) {
        let holds = seen == want;
        self.check(at, holds, format!("{seen:?}, want {want:?}"), spec);
    }

    fn header(&self) -> String {
        format!(
            "step {} \"{}\" (page {}; {})",
            self.step.id,
            self.step.title,
            self.step.page,
            self.step.spec.join("; ")
        )
    }

    /// The step's verdict.
    pub fn finish(self) -> StepResult {
        StepResult {
            step: self.step,
            checkpoints: self.checkpoints,
            failures: self.failures,
        }
    }
}

/// What a step's run found.
pub struct StepResult {
    pub step: &'static Step,
    pub checkpoints: usize,
    pub failures: Vec<String>,
}

impl StepResult {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }

    /// Every failure's report.
    pub fn failure(&self) -> String {
        self.failures.join("\n")
    }
}

/// Run `step` from a fresh page.
pub fn run(step: &'static Step) -> StepResult {
    let mut session = Session::open(step);
    (step.run)(&mut session);
    session.finish()
}
