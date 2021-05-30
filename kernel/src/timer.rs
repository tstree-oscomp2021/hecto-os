use alloc::{boxed::Box, collections::BTreeMap};
use core::time::Duration;

use crate::sync::SpinLock;

pub static TIMER: SpinLock<Timer> = SpinLock::new(Timer::new());

pub struct Timer {
    events: BTreeMap<Duration, Callback>,
}

type Callback = Box<dyn Fn() + Send>;

impl Timer {
    pub const fn new() -> Timer {
        Timer {
            events: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, mut deadline: Duration, callback: Callback) {
        while self.events.contains_key(&deadline) {
            deadline += Duration::from_nanos(1);
        }
        self.events.insert(deadline, Box::new(callback));
    }

    pub fn expire(&mut self, now: Duration) {
        while let Some(entry) = self.events.first_entry() {
            if *entry.key() > now {
                return;
            }
            entry.remove_entry().1();
        }
    }
}
