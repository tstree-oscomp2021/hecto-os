use super::{MEMORY_END, PPN, VA, VPN};
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;
use spin::Mutex;

pub struct FrameTracker {
    // TODO 去掉 pub
    pub ppn: PPN,
}

impl FrameTracker {
    pub fn new(ppn: PPN) -> FrameTracker {
        FrameTracker { ppn }
    }
    pub fn zero(&mut self) {
        VPN::from(self.ppn).get_array::<usize>().fill(0);
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        FRAME_ALLOCATOR.lock().dealloc(self);
    }
}

/// `FrameTracker` 可以 deref 得到对应的 `[u8; PAGE_SIZE]`
impl core::ops::Deref for FrameTracker {
    type Target = [usize];
    fn deref(&self) -> &Self::Target {
        VPN::from(self.ppn).get_array::<usize>()
    }
}

/// `FrameTracker` 可以 deref 得到对应的 `[u8; PAGE_SIZE]`
impl core::ops::DerefMut for FrameTracker {
    fn deref_mut(&mut self) -> &mut Self::Target {
        VPN::from(self.ppn).get_array::<usize>()
    }
}

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<FrameTracker>;
    fn dealloc(&mut self, ft: &FrameTracker);
}

pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PPN, r: PPN) {
        self.current = l.0;
        self.end = r.0;
        log::info!(
            "last {} Physical Frames: [{:#x}, {:#x}]",
            self.end - self.current,
            self.current,
            self.end
        );
    }
}
impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }
    fn alloc(&mut self) -> Option<FrameTracker> {
        if let Some(ppn) = self.recycled.pop() {
            Some(FrameTracker::new(ppn.into()))
        } else {
            if self.current == self.end {
                None
            } else {
                self.current += 1;
                Some(FrameTracker::new((self.current - 1).into()))
            }
        }
    }
    fn dealloc(&mut self, ft: &FrameTracker) {
        let ppn = ft.ppn.into();
        // validity check
        if ppn >= self.current || self.recycled.iter().find(|&v| *v == ppn).is_some() {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // recycle
        self.recycled.push(ppn);
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<FrameAllocatorImpl> =
        Mutex::new(FrameAllocatorImpl::new());
}

pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.lock().init(
        VA(ekernel as usize).ceil().into(),
        VA(MEMORY_END).floor().into(),
    );
    log::info!("frame allocator initialized");
}

pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR.lock().alloc()
}
