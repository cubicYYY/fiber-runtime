//! A minimal I/O reactor built on [`mio`].
//!
//! The reactor runs a single background thread that monitors I/O sources
//! (sockets, pipes, etc.) for readiness events using the OS-native mechanism
//! (epoll on Linux, kqueue on macOS, IOCP on Windows).
//!
//! ## How it works
//!
//! ```text
//! Future               Reactor (bg thread)          Executor
//! ──────               ───────────────────          ────────
//! 1. try read()
//!    → WouldBlock
//! 2. store Waker    ──►  wakers[token] = waker
//!    return Pending
//!                        3. mio::Poll::poll()
//!                           blocks until OS event
//!                        4. event fires for token
//!                           waker.wake()          ──►  task re-enqueued
//!                                                      5. executor re-polls
//!                                                         future retries read()
//!                                                         → success!
//! ```

use mio::{Events, Interest, Poll, Token};
use std::{
    collections::HashMap,
    future::Future,
    io,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
    task::{Context, Poll as TaskPoll, Waker},
    thread,
};

static REACTOR: OnceLock<Reactor> = OnceLock::new();

pub struct Reactor {
    /// Handle to register/deregister I/O sources with the OS poller.
    /// Cloned from `mio::Poll` — the `Poll` itself lives on the bg thread.
    registry: mio::Registry,

    /// Maps each I/O source's token to the waker of the task waiting on it.
    /// The bg thread removes and calls the waker when the event fires.
    wakers: Arc<Mutex<HashMap<Token, Waker>>>,

    /// Monotonic counter for allocating unique tokens.
    next_token: AtomicUsize,
}

impl Reactor {
    /// Returns a reference to the global reactor, spawning its background
    /// thread on first call.
    pub fn get() -> &'static Reactor {
        REACTOR.get_or_init(|| {
            let poll = Poll::new().expect("failed to create mio::Poll");
            let registry = poll
                .registry()
                .try_clone()
                .expect("failed to clone mio registry");
            let wakers: Arc<Mutex<HashMap<Token, Waker>>> =
                Arc::new(Mutex::new(HashMap::new()));

            let wakers_clone = wakers.clone();
            thread::Builder::new()
                .name("reactor".into())
                .spawn(move || Self::event_loop(poll, wakers_clone))
                .expect("failed to spawn reactor thread");

            Reactor {
                registry,
                wakers,
                next_token: AtomicUsize::new(0),
            }
        })
    }

    /// Allocate a unique [`Token`] for a new I/O source.
    pub fn token(&self) -> Token {
        Token(self.next_token.fetch_add(1, Ordering::Relaxed))
    }

    /// Register a new I/O source with the OS poller.
    pub fn register(
        &self,
        source: &mut impl mio::event::Source,
        token: Token,
        interest: Interest,
    ) -> io::Result<()> {
        self.registry.register(source, token, interest)
    }

    /// Remove an I/O source from the OS poller.
    pub fn deregister(&self, source: &mut impl mio::event::Source) -> io::Result<()> {
        self.registry.deregister(source)
    }

    /// Store a waker to be called when the given token's event fires.
    pub fn set_waker(&self, token: Token, waker: Waker) {
        self.wakers.lock().unwrap().insert(token, waker);
    }

    /// Remove a stored waker (called on source drop).
    pub fn remove_waker(&self, token: Token) {
        self.wakers.lock().unwrap().remove(&token);
    }

    /// The event loop. Runs forever on a dedicated background thread.
    ///
    /// Blocks on `mio::Poll::poll()` (which calls epoll/kqueue/IOCP under the
    /// hood), then wakes every task whose I/O source reported readiness.
    fn event_loop(mut poll: Poll, wakers: Arc<Mutex<HashMap<Token, Waker>>>) {
        let mut events = Events::with_capacity(64);
        loop {
            poll.poll(&mut events, None)
                .expect("reactor: mio poll failed");

            let mut map = wakers.lock().unwrap();
            for event in events.iter() {
                if let Some(waker) = map.remove(&event.token()) {
                    waker.wake();
                }
            }
        }
    }
}

/// A future that yields once so the reactor can wake us when the
/// I/O source identified by `token` becomes ready.
///
/// Used internally by the async TCP wrappers.
pub(crate) struct WaitReady {
    token: Token,
    submitted: bool,
}

impl WaitReady {
    pub fn new(token: Token) -> Self {
        Self {
            token,
            submitted: false,
        }
    }
}

impl Future for WaitReady {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> TaskPoll<()> {
        if self.submitted {
            // The reactor (or a spurious wake) woke us. The caller will
            // retry the I/O operation in a loop.
            TaskPoll::Ready(())
        } else {
            // First poll: hand our waker to the reactor so it can wake
            // us when the OS reports readiness for this token.
            Reactor::get().set_waker(self.token, cx.waker().clone());
            self.submitted = true;
            TaskPoll::Pending
        }
    }
}
