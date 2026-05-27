//! `rdom-showcase` binary entry point.
//!
//! Builds the shell, mounts a demo into the main view, pushes
//! per-demo stylesheets onto the App's sheet stack, runs the
//! event loop.
//!
//! ## CLI
//!
//! - `cargo run -p rdom-showcase` — opens to `DEMOS[0]` (Hello World).
//! - `cargo run -p rdom-showcase -- --demo <slug>` — opens directly
//!   to the named demo. The slug matches `Demo::slug()` (e.g.
//!   `layout/hello-world`, `events/counter-button`).
//! - `cargo run -p rdom-showcase -- --list` — print every registered
//!   demo's slug + title and exit.
//! - `cargo run -p rdom-showcase -- --help` — usage + slug list.

use std::cell::RefCell;
use std::process::ExitCode;
use std::rc::Rc;

use rdom_showcase::{
    DEMOS, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet,
    wire_mouse_position_indicator, wire_scroll_indicator, wire_sidebar_click, wire_sidebar_keys,
};
use rdom_tui::{App, TuiDom};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let initial_idx = match parse_args(&args) {
        Ok(CliAction::Run { demo_idx }) => demo_idx,
        Ok(CliAction::List) => {
            print_demo_list();
            return ExitCode::SUCCESS;
        }
        Ok(CliAction::Help) => {
            print_help();
            return ExitCode::SUCCESS;
        }
        Err(msg) => {
            eprintln!("rdom-showcase: {msg}");
            eprintln!();
            print_help();
            return ExitCode::from(2);
        }
    };
    match run(initial_idx) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("rdom-showcase: {err}");
            ExitCode::FAILURE
        }
    }
}

/// What the parsed CLI tells the binary to do.
#[derive(Debug)]
enum CliAction {
    Run { demo_idx: usize },
    List,
    Help,
}

/// Parse the argv tail (after the program name). Returns the
/// action to take or an error message describing the problem.
fn parse_args(args: &[String]) -> Result<CliAction, String> {
    let mut i = 0;
    let mut demo_idx: Option<usize> = None;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => return Ok(CliAction::Help),
            "--list" => return Ok(CliAction::List),
            "--demo" => {
                let slug = args
                    .get(i + 1)
                    .ok_or_else(|| "--demo requires a slug argument".to_string())?;
                let idx = DEMOS.iter().position(|d| d.slug() == slug).ok_or_else(|| {
                    format!("no demo with slug {slug:?} — use `--list` to see registered demos")
                })?;
                demo_idx = Some(idx);
                i += 2;
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }
    Ok(CliAction::Run {
        demo_idx: demo_idx.unwrap_or(0),
    })
}

fn print_help() {
    println!("rdom-showcase — browsable TUI demos for the rdom substrate");
    println!();
    println!("Usage:");
    println!("  rdom-showcase                    # open to the first demo");
    println!("  rdom-showcase --demo <slug>      # open directly to a named demo");
    println!("  rdom-showcase --list             # list every registered demo");
    println!("  rdom-showcase --help             # this message");
    println!();
    println!("Slugs:");
    for demo in DEMOS {
        println!("  {} — {}", demo.slug(), demo.title());
    }
}

fn print_demo_list() {
    for demo in DEMOS {
        println!("{}\t{}", demo.slug(), demo.title());
    }
}

fn run(initial_idx: usize) -> std::io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    let state = Rc::new(RefCell::new(ShowcaseState::from_handles(&handles)));

    // Initial mount: deep-linked demo (default 0).
    mount_demo(&mut state.borrow_mut(), &mut dom, initial_idx);

    // Sidebar click handler — walks up from the click target to
    // find the `<li>` with `data-demo-slug`, then swaps demos.
    wire_sidebar_click(&mut dom, handles.sidebar, Rc::clone(&state));
    // Sidebar keyboard handler — Arrow keys to navigate between
    // demo `<li>`s, Enter / Space to activate the focused one.
    // Tab / Shift+Tab is handled by the runtime's built-in
    // focus traversal (the `<li>`s carry `tabindex="0"`).
    wire_sidebar_keys(&mut dom, handles.sidebar, Rc::clone(&state));
    // Scroll listener — writes scroll info into the LEFT slot of
    // the status bar (the hints slot) whenever any scrollable
    // descendant of `<main>` fires a scroll event.
    wire_scroll_indicator(&mut dom, handles.main, handles.status_bar_hints);
    // Live mouse-position gauge in the RIGHT slot of the status
    // bar — instantly tells the developer whether motion events
    // are flowing (numbers update) or stalled (numbers freeze).
    // Independent slot so it doesn't clobber the hints / scroll
    // content owned by other listeners.
    wire_mouse_position_indicator(&mut dom, handles.status_bar_mouse_pos);
    // Focus listener — refreshes keyboard hints in the LEFT slot
    // whenever focus moves. The bar is pre-seeded in `build_shell`
    // with the global default; this listener handles changes.
    rdom_showcase::wire_focus_hints(&mut dom, handles.status_bar_hints);

    // Construct the App with the shell's base stylesheet.
    let mut app = App::new(dom, base_stylesheet())?;

    // Pre-push every demo's stylesheet onto the App's sheet stack.
    // Each demo's CSS uses unique class-scoped selectors (e.g.
    // `.hello`, `.flex-row-demo`, `.hover-demo`), so the cascade
    // naturally applies only the mounted demo's rules — switching
    // demos is just a subtree swap, no per-demo sheet push/remove
    // required.
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }

    app.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_args_defaults_to_first_demo() {
        match parse_args(&[]).unwrap() {
            CliAction::Run { demo_idx } => assert_eq!(demo_idx, 0),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn help_flag_returns_help_action() {
        for flag in &["--help", "-h"] {
            match parse_args(&args(&[flag])).unwrap() {
                CliAction::Help => {}
                _ => panic!("expected Help"),
            }
        }
    }

    #[test]
    fn list_flag_returns_list_action() {
        match parse_args(&args(&["--list"])).unwrap() {
            CliAction::List => {}
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn demo_flag_resolves_known_slug() {
        match parse_args(&args(&["--demo", "events/counter-button"])).unwrap() {
            CliAction::Run { demo_idx } => {
                assert_eq!(DEMOS[demo_idx].slug(), "events/counter-button");
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn demo_flag_with_unknown_slug_errors() {
        let err = parse_args(&args(&["--demo", "no-such-demo"])).unwrap_err();
        assert!(err.contains("no-such-demo"), "{err}");
        assert!(err.contains("--list"), "{err}");
    }

    #[test]
    fn demo_flag_without_argument_errors() {
        let err = parse_args(&args(&["--demo"])).unwrap_err();
        assert!(err.contains("requires"), "{err}");
    }

    #[test]
    fn unknown_argument_errors() {
        let err = parse_args(&args(&["--bogus"])).unwrap_err();
        assert!(err.contains("--bogus"), "{err}");
    }

    #[test]
    fn multiple_demo_flags_last_one_wins() {
        // Repeated `--demo X --demo Y` — last wins (POSIX-ish
        // convention). Pin the behavior so a future refactor
        // doesn't silently swap to "first wins" or "error".
        let slug_a = DEMOS[0].slug();
        let slug_b = DEMOS[2].slug();
        match parse_args(&args(&["--demo", slug_a, "--demo", slug_b])).unwrap() {
            CliAction::Run { demo_idx } => assert_eq!(DEMOS[demo_idx].slug(), slug_b),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn list_short_circuits_after_demo() {
        // `--demo X --list` — list trumps; never enters TUI.
        // Useful for "show me the demo list, but I've been
        // running this command with --demo before."
        let slug = DEMOS[0].slug();
        match parse_args(&args(&["--demo", slug, "--list"])).unwrap() {
            CliAction::List => {}
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn help_short_circuits_after_demo() {
        let slug = DEMOS[0].slug();
        match parse_args(&args(&["--demo", slug, "--help"])).unwrap() {
            CliAction::Help => {}
            _ => panic!("expected Help"),
        }
    }

    #[test]
    fn list_flag_before_demo_short_circuits() {
        // `--list --demo X` — list trumps regardless of position
        // (it returns immediately on first sight).
        match parse_args(&args(&["--list", "--demo", "events/counter-button"])).unwrap() {
            CliAction::List => {}
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn unknown_flag_after_demo_errors() {
        // Error should still propagate even if a valid `--demo`
        // preceded the bogus arg.
        let slug = DEMOS[0].slug();
        let err = parse_args(&args(&["--demo", slug, "--bogus"])).unwrap_err();
        assert!(err.contains("--bogus"), "{err}");
    }
}
