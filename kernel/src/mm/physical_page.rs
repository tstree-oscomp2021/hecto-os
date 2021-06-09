use core::{alloc::Layout, mem::size_of, ptr::null_mut};

use super::{ForwardList, PPN, VA, VPN};
use crate::{
    board::{ekernel, interface::Config, ConfigImpl},
    sync::SpinLock,
};

pub struct FreePhyPage;
/// 空闲页面链表
type FreePhyPageList = ForwardList<FreePhyPage>;

/// 页面使用状况
pub struct PhyPageUsage {
    /// 计数
    ref_count: usize,
}

pub static PHYPAGE_ALLOCATOR: SpinLock<PhyPageAllocator> = SpinLock::new(PhyPageAllocator {
    free_list: FreePhyPageList::new(),
    page_usage: null_mut(),
    start_ppn: PPN(1),
});

pub struct FrameTracker {
    pub ppn: PPN,
}
impl FrameTracker {
    pub fn get_ref_count(&self) -> usize {
        unsafe { PHYPAGE_ALLOCATOR.lock().get_ref_count(self) }
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        unsafe { PHYPAGE_ALLOCATOR.lock().dealloc(self) }
    }
}
impl Clone for FrameTracker {
    fn clone(&self) -> Self {
        unsafe { PHYPAGE_ALLOCATOR.lock().clone(self) };
        Self { ppn: self.ppn }
    }
}

pub struct PhyPageAllocator {
    free_list: FreePhyPageList,
    page_usage: *mut PhyPageUsage,
    start_ppn: PPN,
}
impl PhyPageAllocator {
    pub unsafe fn init(&mut self) {
        let start = VA(ekernel as usize).ceil();
        let end = VA(ConfigImpl::MEMORY_END).floor();
        for vpn in (start..end).rev() {
            self.free_list.push_front(VA::from(vpn).as_mut_ptr());
        }
        println!(
            "PhyPageAllocator init. pages: [{:#x}, {:#x}) {}",
            start.0,
            end.0,
            end.0 - start.0
        );

        let layout = Layout::from_size_align_unchecked(
            (end - start) * size_of::<PhyPageUsage>(),
            size_of::<usize>(),
        );
        println!("page_usage {:?}", layout);
        self.page_usage = alloc::alloc::alloc_zeroed(layout) as *mut PhyPageUsage;
        self.start_ppn = start.into();
    }

    pub unsafe fn alloc(&mut self) -> Option<FrameTracker> {
        let page = self.free_list.get_head();
        if page.is_null() {
            return None;
        }
        self.free_list.pop_front();

        let ppn: PPN = VA::from(page).floor().into();
        let usage = &mut *self.page_usage.add(ppn - self.start_ppn);
        debug_assert!(usage.ref_count == 0);
        usage.ref_count = 1;
        // println!("ppn = {:#x}", ppn.0);

        Some(FrameTracker { ppn })
    }

    pub unsafe fn dealloc(&mut self, frame: &FrameTracker) {
        let usage = &mut *self.page_usage.add(frame.ppn - self.start_ppn);
        debug_assert!(usage.ref_count > 0);
        usage.ref_count -= 1;
        if usage.ref_count == 0 {
            // println!("drop ppn = {:#x}", frame.ppn.0);
            let page = VA::from(VPN::from(frame.ppn)).as_mut_ptr();
            debug_assert!(!(page as *const u8).is_null());
            self.free_list.push_front(page);
        }
    }

    pub unsafe fn clone(&mut self, frame: &FrameTracker) {
        let usage = &mut *self.page_usage.add(frame.ppn - self.start_ppn);
        // println!("clone ppn = {:#x}", frame.ppn.0);
        debug_assert!(usage.ref_count > 0);
        usage.ref_count += 1;
    }
    pub unsafe fn get_ref_count(&mut self, frame: &FrameTracker) -> usize {
        let usage = &mut *self.page_usage.add(frame.ppn - self.start_ppn);
        debug_assert!(usage.ref_count > 0);
        usage.ref_count
    }
}

unsafe impl Sync for PhyPageAllocator {}
unsafe impl Send for PhyPageAllocator {}

pub fn init_frame_allocator() {
    unsafe { PHYPAGE_ALLOCATOR.lock().init() }
}

pub fn frame_alloc() -> Option<FrameTracker> {
    unsafe { PHYPAGE_ALLOCATOR.lock().alloc() }
}
