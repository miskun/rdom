//! Transitions and animations of `translate` and `transform` (CSS
//! Transforms 2 §6.1, §11 "Interpolation of transforms", Web Animations 1
//! §5.4.5): the running value moves the box frame by frame in whole cells.

use rdom_tui::prelude::*;

/// A `w` × 3 app over `markup`'s one `#t` styled by `css`.
fn app(css: &str, w: u16) -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let body = dom.create_element("body");
    dom.append_child(root, body).unwrap();
    let t = dom.create_element("div");
    dom.set_attribute(t, "id", "t").unwrap();
    let text = dom.create_text_node("t");
    dom.append_child(t, text).unwrap();
    dom.append_child(body, t).unwrap();
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    let terminal = Terminal::new(TestBackend::new(w, 3)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    (app, t)
}

fn x(app: &App<TestBackend>, t: NodeId) -> i32 {
    app.dom().node(t).layout_rect().unwrap().x
}

/// Transforms 2 §11: `translate` interpolates component-wise (`none` as
/// zero); the running value moves the box every frame its cell changes.
#[test]
fn a_translate_transition_slides_the_box() {
    let (mut app, t) = app(
        "#t { width: 2; height: 1; translate: 0; transition: translate 100ms linear }
         #t.in { translate: 10 }",
        20,
    );
    assert_eq!(x(&app, t), 0);
    app.dom_mut().set_attribute(t, "class", "in").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    assert_eq!(x(&app, t), 5);
    app.advance(30).unwrap();
    assert_eq!(x(&app, t), 8);
    app.advance(40).unwrap();
    assert_eq!(x(&app, t), 10);
}

/// Transforms 2 §11 / §12: two translate-only `transform` lists of the
/// same functions interpolate function by function; a `@keyframes`
/// slide-in from off-screen runs its course.
#[test]
fn a_transform_slide_in_animation_runs() {
    let (mut app, t) = app(
        "@keyframes slide { from { transform: translateX(-10) } to { transform: translateX(0) } }
         #t { width: 2; height: 1; animation: slide 100ms linear both }",
        20,
    );
    assert_eq!(x(&app, t), -10);
    app.advance(50).unwrap();
    assert_eq!(x(&app, t), -5);
    app.advance(60).unwrap();
    assert_eq!(x(&app, t), 0);
}
