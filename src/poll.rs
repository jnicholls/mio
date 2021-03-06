use {sys, Evented, Token};
use event::{EventSet, IoEvent, PollOpt};
use std::{fmt, io};

pub use sys::{Events};

pub struct Poll {
    selector: sys::Selector,
    events: sys::Events,
}

impl Poll {
    pub fn new() -> io::Result<Poll> {
        Ok(Poll {
            selector: try!(sys::Selector::new()),
            events: sys::Events::new(),
        })
    }

    pub fn register<E: ?Sized>(&mut self, io: &E, token: Token, interest: EventSet, opts: PollOpt) -> io::Result<()>
        where E: Evented
    {
        trace!("registering with poller");

        // Register interests for this socket
        try!(io.register(&mut self.selector, token, interest, opts));

        Ok(())
    }

    pub fn reregister<E: ?Sized>(&mut self, io: &E, token: Token, interest: EventSet, opts: PollOpt) -> io::Result<()>
        where E: Evented
    {
        trace!("registering with poller");

        // Register interests for this socket
        try!(io.reregister(&mut self.selector, token, interest, opts));

        Ok(())
    }

    pub fn deregister<E: ?Sized>(&mut self, io: &E) -> io::Result<()>
        where E: Evented
    {
        trace!("deregistering IO with poller");

        // Deregister interests for this socket
        try!(io.deregister(&mut self.selector));

        Ok(())
    }

    pub fn poll(&mut self, timeout_ms: usize) -> io::Result<usize> {
        try!(self.selector.select(&mut self.events, timeout_ms));
        Ok(self.events.len())
    }

    pub fn event(&self, idx: usize) -> IoEvent {
        self.events.get(idx)
    }
}

impl fmt::Debug for Poll {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Poll")
    }
}

