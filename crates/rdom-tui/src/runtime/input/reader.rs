//! [`InputReader`] — the terminal input source of `App::run`.
//!
//! Unix: `poll(2)` over the terminal (stdin, or `/dev/tty` when stdin is
//! not a terminal) and a self-pipe the `SIGWINCH` handler writes to
//! (`signal_hook::low_level::pipe`, crossterm's mechanism); each read
//! is fed to the [`Parser`], and a window-size change becomes
//! `Event::Resize` with the size `crossterm::terminal::size` reads. A
//! lone escape prefix waits [`ESC_GRACE`] for the rest of its
//! sequence.
//!
//! Other platforms: crossterm's own reader (`crossterm::event::poll` /
//! `read`); no replies or reports are read there.

use std::time::Duration;

use super::Input;
#[cfg(unix)]
use super::{ESC_GRACE, Parser};

/// The terminal's input, parsed.
pub(crate) struct InputReader {
    #[cfg(unix)]
    unix: unix::Source,
    #[cfg(unix)]
    parser: Parser,
    /// When the buffered escape prefix last grew (`Parser::awaits_prefix`).
    #[cfg(unix)]
    prefix_since: Option<std::time::Instant>,
    #[cfg(not(unix))]
    held: std::collections::VecDeque<Input>,
}

#[cfg(unix)]
impl InputReader {
    /// Read the controlling terminal: stdin when it is one, else
    /// `/dev/tty`; and listen for window-size changes.
    pub(crate) fn open() -> std::io::Result<Self> {
        Ok(Self::with_source(unix::Source::open()?))
    }

    /// Read `fd` (a test's socket); no window-size signal.
    #[cfg(test)]
    pub(crate) fn from_fd(fd: std::os::fd::OwnedFd) -> Self {
        Self::with_source(unix::Source::from_fd(fd))
    }

    fn with_source(unix: unix::Source) -> Self {
        Self {
            unix,
            parser: Parser::default(),
            prefix_since: None,
        }
    }

    /// Wait up to `timeout` for an input; `true` once one is ready for
    /// [`Self::next`]. A zero timeout reads what has arrived without
    /// waiting. An escape prefix that nothing has followed for
    /// [`ESC_GRACE`] becomes a key here — after the bytes already
    /// queued are read, so a caller that comes back late does not split
    /// a sequence that arrived in time (`C4G-ESC-GRACE`). Errors: the
    /// terminal closed (end of file, hang-up) or could not be read.
    pub(crate) fn poll(&mut self, timeout: Duration) -> std::io::Result<bool> {
        use std::time::Instant;
        let deadline = Instant::now() + timeout;
        let mut first = true;
        loop {
            let now = Instant::now();
            if let Some(since) = self.prefix_since
                && now >= since + ESC_GRACE
                && !self.read_queued()?
            {
                self.parser.flush_prefix();
                self.prefix_since = None;
            }
            if self.parser.has_ready() {
                return Ok(true);
            }
            if !first && now >= deadline {
                return Ok(false);
            }
            first = false;
            let mut wait = deadline.saturating_duration_since(now);
            if let Some(since) = self.prefix_since {
                wait = wait.min((since + ESC_GRACE).saturating_duration_since(now));
            }
            let ready = self.unix.wait(wait)?;
            if ready.input {
                self.read_input()?;
            }
            if ready.resize {
                self.unix.drain_resize();
                let (w, h) = crossterm::terminal::size()?;
                self.parser
                    .deliver(Input::Event(crossterm::event::Event::Resize(w, h)));
            }
        }
    }

    /// Read the bytes already queued, without waiting; `true` if there
    /// were any.
    fn read_queued(&mut self) -> std::io::Result<bool> {
        if !self.unix.wait(Duration::ZERO)?.input {
            return Ok(false);
        }
        self.read_input()
    }

    /// Read and parse what the terminal has sent (after `wait` said so);
    /// `true` if any byte came. A read that fills the buffer may have
    /// left more behind (crossterm's rule), so reading goes on — without
    /// waiting — until one does not. The escape grace restarts when the
    /// bytes leave a prefix.
    fn read_input(&mut self) -> std::io::Result<bool> {
        let mut any = false;
        loop {
            let (bytes, full) = self.unix.read()?;
            any |= !bytes.is_empty();
            self.parser.feed(bytes);
            if !full || !self.unix.wait(Duration::ZERO)?.input {
                break;
            }
        }
        if any {
            self.prefix_since = self.parser.awaits_prefix().then(std::time::Instant::now);
        }
        Ok(any)
    }

    /// The next ready input, if any.
    pub(crate) fn next(&mut self) -> Option<Input> {
        self.parser.next()
    }

    /// Put inputs back, ahead of anything not yet taken.
    pub(crate) fn unread(&mut self, inputs: Vec<Input>) {
        self.parser.unread(inputs);
    }

    /// True while an escape sequence is partly read (a reply arriving
    /// over a slow link, say).
    pub(crate) fn in_sequence(&self) -> bool {
        self.parser.in_sequence()
    }
}

#[cfg(not(unix))]
impl InputReader {
    /// crossterm's reader.
    pub(crate) fn open() -> std::io::Result<Self> {
        Ok(Self {
            held: Default::default(),
        })
    }

    /// `crossterm::event::poll`.
    pub(crate) fn poll(&mut self, timeout: Duration) -> std::io::Result<bool> {
        if !self.held.is_empty() {
            return Ok(true);
        }
        crossterm::event::poll(timeout)
    }

    /// `crossterm::event::read` (after a `poll` that said ready). A read
    /// error ends the drain, as it did before rdom read input itself.
    pub(crate) fn next(&mut self) -> Option<Input> {
        if let Some(input) = self.held.pop_front() {
            return Some(input);
        }
        match crossterm::event::read() {
            Ok(event) => Some(Input::Event(event)),
            Err(e) => {
                crate::rdom_trace!("crossterm::event::read() -> Err({e:?})");
                None
            }
        }
    }

    /// Put inputs back, ahead of anything not yet taken.
    pub(crate) fn unread(&mut self, inputs: Vec<Input>) {
        for input in inputs.into_iter().rev() {
            self.held.push_front(input);
        }
    }

    /// Always false: no sequence is read here.
    pub(crate) fn in_sequence(&self) -> bool {
        false
    }
}

#[cfg(unix)]
mod unix {
    //! The file descriptors: the terminal, and the `SIGWINCH` pipe.

    use std::io::{self, IsTerminal, Read};
    use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use rustix::event::{PollFd, PollFlags, poll};
    use rustix::io::Errno;

    /// Bytes read per `read(2)` (crossterm reads 1 KiB too).
    const BUF_SIZE: usize = 1024;

    enum Tty {
        Stdin(io::Stdin),
        Owned(OwnedFd),
    }

    impl Tty {
        fn fd(&self) -> BorrowedFd<'_> {
            match self {
                Tty::Stdin(s) => s.as_fd(),
                Tty::Owned(f) => f.as_fd(),
            }
        }
    }

    /// The `SIGWINCH` self-pipe; unregistered on drop.
    struct Resize {
        rx: UnixStream,
        id: signal_hook::SigId,
    }

    impl Drop for Resize {
        fn drop(&mut self) {
            signal_hook::low_level::unregister(self.id);
        }
    }

    pub(super) struct Source {
        tty: Tty,
        resize: Option<Resize>,
        buf: Box<[u8; BUF_SIZE]>,
    }

    /// What `wait` found readable.
    pub(super) struct Ready {
        pub(super) input: bool,
        pub(super) resize: bool,
    }

    impl Source {
        pub(super) fn open() -> io::Result<Self> {
            let stdin = io::stdin();
            let tty = if stdin.is_terminal() {
                Tty::Stdin(stdin)
            } else {
                Tty::Owned(
                    std::fs::OpenOptions::new()
                        .read(true)
                        .open("/dev/tty")?
                        .into(),
                )
            };
            let (rx, tx) = UnixStream::pair()?;
            rx.set_nonblocking(true)?;
            tx.set_nonblocking(true)?;
            let id = signal_hook::low_level::pipe::register(signal_hook::consts::SIGWINCH, tx)?;
            Ok(Self {
                tty,
                resize: Some(Resize { rx, id }),
                buf: Box::new([0; BUF_SIZE]),
            })
        }

        #[cfg(test)]
        pub(super) fn from_fd(fd: OwnedFd) -> Self {
            Self {
                tty: Tty::Owned(fd),
                resize: None,
                buf: Box::new([0; BUF_SIZE]),
            }
        }

        /// Wait up to `timeout` (rounded up to a millisecond) for input
        /// or a size change.
        pub(super) fn wait(&self, timeout: Duration) -> io::Result<Ready> {
            let ms = timeout.as_micros().div_ceil(1000);
            let ms = i32::try_from(ms).unwrap_or(i32::MAX);
            let tty = self.tty.fd();
            let mut fds = [
                PollFd::new(&tty, PollFlags::IN),
                PollFd::new(&tty, PollFlags::IN),
            ];
            let n = match &self.resize {
                Some(r) => {
                    fds[1] = PollFd::new(&r.rx, PollFlags::IN);
                    2
                }
                None => 1,
            };
            let fds = &mut fds[..n];
            match poll(fds, ms) {
                Ok(_) => {}
                // A signal (the size change itself) interrupted the wait:
                // report nothing; the caller polls again.
                Err(Errno::INTR) => {
                    return Ok(Ready {
                        input: false,
                        resize: false,
                    });
                }
                Err(e) => return Err(e.into()),
            }
            let ready = |f: &PollFd<'_>| !f.revents().is_empty();
            Ok(Ready {
                input: ready(&fds[0]),
                resize: fds.get(1).is_some_and(ready),
            })
        }

        /// Read what the terminal has sent (after `wait` said so), and
        /// whether the read filled the buffer (more may be queued).
        pub(super) fn read(&mut self) -> io::Result<(&[u8], bool)> {
            loop {
                match rustix::io::read(self.tty.fd(), &mut self.buf[..]) {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "the terminal's input closed",
                        ));
                    }
                    Ok(n) => return Ok((&self.buf[..n], n == BUF_SIZE)),
                    Err(Errno::INTR) => continue,
                    Err(Errno::AGAIN) => return Ok((&[], false)),
                    Err(e) => return Err(e.into()),
                }
            }
        }

        /// Empty the size-change pipe (several signals, one resize).
        pub(super) fn drain_resize(&mut self) {
            if let Some(r) = &mut self.resize {
                let mut sink = [0u8; 64];
                while matches!(r.rx.read(&mut sink), Ok(n) if n > 0) {}
            }
        }
    }
}

#[cfg(all(test, unix))]
#[path = "reader_tests.rs"]
mod tests;
