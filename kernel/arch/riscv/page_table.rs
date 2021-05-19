use bitflags::bitflags;
use riscv::register::satp;

use crate::{
    arch::interface::{PageTable, PTE},
    frame_alloc, FrameTracker, KERNEL_PAGE_TABLE, PPN, VPN,
};

bitflags! {
    /// 页表项中的 8 个标志位
    #[derive(Default)]
    pub struct PTEImpl: usize {
        const EMPTY =            0;
        /// 有效位
        const VALID =       1 << 0;
        /// 可读位
        const READABLE =    1 << 1;
        /// 可写位
        const WRITABLE =    1 << 2;
        /// 可执行位
        const EXECUTABLE =  1 << 3;
        /// 用户位
        const USER =        1 << 4;
        /// 全局位
        const GLOBAL =      1 << 5;
        /// 已使用位
        const ACCESSED =    1 << 6;
        /// 已修改位
        const DIRTY =       1 << 7;
    }
}

impl PTE for PTEImpl {
    const EMPTY: Self = Self::EMPTY;
    const EXECUTABLE: Self = Self::EXECUTABLE;
    const READABLE: Self = Self::READABLE;
    const USER: Self = Self::USER;
    const VALID: Self = Self::VALID;
    const WRITABLE: Self = Self::WRITABLE;

    fn new(ppn: PPN, flags: Self) -> Self {
        Self {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }

    fn ppn(self) -> PPN {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    fn is_valid(self) -> bool {
        (self & Self::VALID) != Self::EMPTY
    }
}

use alloc::{vec, vec::Vec};

/// TODO field 都 private
pub struct PageTableImpl {
    /// 根页表的页框
    pub root: FrameTracker,
    /// 其他子页表的页框
    pub frames: Vec<FrameTracker>,
}

impl PageTable for PageTableImpl {
    fn new_kernel() -> Self {
        let frame = frame_alloc().unwrap();
        // 不需要清零，直接从 KERNEL_PAGE_TABLE 复制就行！
        VPN::from(frame.ppn)
            .get_array::<PTEImpl>()
            .copy_from_slice(VPN::from(KERNEL_PAGE_TABLE.root.ppn).get_array::<PTEImpl>());
        PageTableImpl {
            root: frame,
            frames: vec![],
        }
    }

    /// 查找 vpn 虚拟页号对应的 pte
    fn find_pte_create(&mut self, vpn: VPN) -> Option<&mut PTEImpl> {
        let idxs = vpn.indexes();
        let mut pte: &mut PTEImpl = &mut VPN::from(self.root.ppn).get_array()[idxs[0]];
        for &idx in &idxs[1..] {
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PTEImpl::new(frame.ppn, PTEImpl::VALID);
                self.frames.push(frame);
            }
            pte = &mut VPN::from(pte.ppn()).get_array()[idx];
        }
        Some(pte)
    }

    /// 查找 vpn 虚拟页号对应的 pte
    fn find_pte(&self, vpn: VPN) -> Option<&mut PTEImpl> {
        let idxs = vpn.indexes();
        let mut pte: &mut PTEImpl = &mut VPN::from(self.root.ppn).get_array()[idxs[0]];
        for &idx in &idxs[1..] {
            if !pte.is_valid() {
                return None;
            }
            pte = &mut VPN::from(pte.ppn()).get_array()[idx];
        }
        Some(pte)
    }

    /// 激活页表
    fn activate(&self) {
        // Sv39
        let satp = 8usize << 60 | self.root.ppn.0;
        unsafe {
            satp::write(satp);
            llvm_asm!("sfence.vma" :::: "volatile");
        }
    }
}
