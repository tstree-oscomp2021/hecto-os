use alloc::{sync::Arc, vec::Vec};
use core::fmt::{self, Debug, Formatter};

use lazy_static::*;
use spin::Mutex;

use super::{PPN, VA, VPN};
use crate::board::{interface::Config, ConfigImpl};

pub struct Frame {
    // TODO 去掉 pub
    pub ppn: PPN,
}

impl Frame {
    pub fn new(ppn: PPN) -> Frame {
        Frame { ppn }
    }

    pub fn zero(&mut self) {
        VPN::from(self.ppn).get_array::<usize>().fill(0);
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Frame:PPN={:#x}", self.ppn.0))
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        FRAME_ALLOCATOR.lock().dealloc(self);
    }
}

pub type FrameTracker = Arc<Frame>;

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<FrameTracker>;
    fn dealloc(&mut self, ft: &Frame);
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
        info!(
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
            Some(Arc::new(Frame::new(ppn.into())))
        } else {
            if self.current == self.end {
                None
            } else {
                self.current += 1;
                Some(Arc::new(Frame::new((self.current - 1).into())))
            }
        }
    }

    fn dealloc(&mut self, ft: &Frame) {
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
        VA(ConfigImpl::MEMORY_END).floor().into(),
    );
    info!("frame allocator initialized");
}

pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR.lock().alloc()
}
