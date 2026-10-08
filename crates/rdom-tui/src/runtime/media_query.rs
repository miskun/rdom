//! `matchMedia()` (CSSOM View §4.2): a [`MediaQueryList`] is a media query
//! list watched against the `App`'s media environment — the terminal's
//! size, the preferred color scheme and the reported preferences
//! (`App::match_media`). It says whether it matches now, and calls its
//! listeners with a [`MediaQueryListEvent`] each time that flips: the
//! App re-evaluates its lists at every frame ("evaluate media queries and
//! report changes", one of HTML's update-the-rendering steps, before the
//! frame's style and layout), in the order they were created.
//!
//! A list is a cheap handle (clone it freely); the App keeps only a weak
//! reference, so a list nobody holds stops being evaluated. A listener
//! gets the document through a [`TimerCtx`], as timer callbacks do — what
//! a script's `change` handler would reach through `document`.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use rdom_style::conditional::{MediaEnvironment, MediaList};

use crate::runtime::timers::TimerCtx;

/// A watched media query list (CSSOM View §4.2 `MediaQueryList`).
#[derive(Clone)]
pub struct MediaQueryList {
    watch: Rc<Watch>,
}

impl std::fmt::Debug for MediaQueryList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaQueryList")
            .field("media", &self.watch.media)
            .field("matches", &self.watch.matches.get())
            .finish()
    }
}

/// A listener's handle, for [`MediaQueryList::remove_listener`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaListenerId(u32);

/// The `change` event of a [`MediaQueryList`] (CSSOM View §4.2.1
/// `MediaQueryListEvent`): its serialized media and its new result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MediaQueryListEvent {
    /// The list's serialized media query list.
    pub media: String,
    /// Whether it matches now.
    pub matches: bool,
}

type Listener = Rc<RefCell<dyn FnMut(&mut TimerCtx<'_>, &MediaQueryListEvent)>>;

pub(crate) struct Watch {
    list: MediaList,
    media: String,
    matches: Cell<bool>,
    listeners: RefCell<Vec<(MediaListenerId, Listener)>>,
    next_listener: Cell<u32>,
}

impl MediaQueryList {
    /// The serialized media query list (CSSOM §4.2.2): `(MIN-WIDTH: 20)`
    /// reads `(width >= 20)`; a query that does not parse reads `not all`.
    pub fn media(&self) -> &str {
        &self.watch.media
    }

    /// Whether the list matches the environment as last evaluated — when
    /// it was created, then at each frame.
    pub fn matches(&self) -> bool {
        self.watch.matches.get()
    }

    /// Call `listener` each time the result flips (`addEventListener(
    /// "change", …)`), with the document and the event.
    pub fn add_listener(
        &self,
        listener: impl FnMut(&mut TimerCtx<'_>, &MediaQueryListEvent) + 'static,
    ) -> MediaListenerId {
        let id = MediaListenerId(self.watch.next_listener.get());
        self.watch.next_listener.set(id.0 + 1);
        self.watch
            .listeners
            .borrow_mut()
            .push((id, Rc::new(RefCell::new(listener))));
        id
    }

    /// Stop calling the listener `id`; `false` when it is not one of this
    /// list's.
    pub fn remove_listener(&self, id: MediaListenerId) -> bool {
        let mut listeners = self.watch.listeners.borrow_mut();
        let before = listeners.len();
        listeners.retain(|(l, _)| *l != id);
        listeners.len() != before
    }
}

/// The App's lists, weakly held, and the environment they were last
/// evaluated in.
#[derive(Default)]
pub(crate) struct MediaWatches {
    watches: Vec<Weak<Watch>>,
    last: Option<MediaEnvironment>,
}

impl MediaWatches {
    /// A new list for `query`, evaluated in `env`.
    pub(crate) fn watch(&mut self, query: &str, env: &MediaEnvironment) -> MediaQueryList {
        let list = MediaList::parse(query);
        let watch = Rc::new(Watch {
            media: list.to_string(),
            matches: Cell::new(list.matches(env)),
            list,
            listeners: RefCell::new(Vec::new()),
            next_listener: Cell::new(0),
        });
        self.watches.push(Rc::downgrade(&watch));
        MediaQueryList { watch }
    }

    /// Evaluate every live list in `env` (nothing to do when it is the
    /// environment of the last evaluation): each one whose result
    /// flipped, in creation order, with its event — the caller fires them.
    pub(crate) fn changes(
        &mut self,
        env: &MediaEnvironment,
    ) -> Vec<(Vec<Listener>, MediaQueryListEvent)> {
        if self.last.as_ref() == Some(env) {
            return Vec::new();
        }
        self.last = Some(*env);
        self.watches.retain(|w| w.strong_count() > 0);
        let mut out = Vec::new();
        for watch in self.watches.iter().filter_map(Weak::upgrade) {
            let matches = watch.list.matches(env);
            if matches == watch.matches.replace(matches) {
                continue;
            }
            // The listeners as they are now: one added by a listener runs
            // from the next change.
            let listeners = watch
                .listeners
                .borrow()
                .iter()
                .map(|(_, l)| l.clone())
                .collect();
            let event = MediaQueryListEvent {
                media: watch.media.clone(),
                matches,
            };
            out.push((listeners, event));
        }
        out
    }
}

/// Call `listeners` with `event`, the document through `cx`.
pub(crate) fn fire(cx: &mut TimerCtx<'_>, listeners: &[Listener], event: &MediaQueryListEvent) {
    for listener in listeners {
        (listener.borrow_mut())(cx, event);
    }
}
