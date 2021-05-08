use super::*;
use crate::process::KERNEL_PROCESS;

use alloc::{vec, vec::Vec};
use bitflags::*;
use core::slice::from_raw_parts_mut;
use lazy_static::lazy_static;
use log::*;
use riscv::register::satp;

use crate::{config::MMIO, ffi::*};

bitflags! {
    /// 页表项中的 8 个标志位
    #[derive(Default)]
    pub struct PTEFlags: u8 {
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

macro_rules! implement_flags {
    ($field: ident, $name: ident, $quote: literal) => {
        impl PTEFlags {
            #[doc = "返回 `Flags::"]
            #[doc = $quote]
            #[doc = "` 或 `Flags::empty()`"]
            pub fn $name(value: bool) -> PTEFlags {
                if value {
                    PTEFlags::$field
                } else {
                    PTEFlags::empty()
                }
            }
        }
    };
}

implement_flags! {READABLE, readable, "READABLE"}
implement_flags! {WRITABLE, writable, "WRITABLE"}
implement_flags! {EXECUTABLE, executable, "EXECUTABLE"}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PPN, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    pub fn ppn(&self) -> PPN {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::VALID) != PTEFlags::empty()
    }
}

lazy_static! {
    /// 请通过内核进程而非此变量来映射内核栈，因为映射涉及到页框的创建和保存
    pub static ref KERNEL_PAGE_TABLE: &'static PageTable =
        unsafe { &*(&KERNEL_PROCESS.inner.lock().memory_set.page_table as *const PageTable) };
}

/// 仅在初始化 KERNEL_PROCESS 时被调用
pub fn kernel_page_table() -> PageTable {
    let mut frame = frame_alloc().unwrap();
    frame.zero();
    let mut page_table = PageTable {
        root: frame,
        frames: vec![],
    };
    // TODO 表驱动
    let mut areas = [
        MapArea::new(
            (stext as usize).into()..(etext as usize).into(),
            MapType::Linear,
            PTEFlags::READABLE | PTEFlags::EXECUTABLE,
        ),
        MapArea::new(
            (srodata as usize).into()..(erodata as usize).into(),
            MapType::Linear,
            PTEFlags::READABLE,
        ),
        MapArea::new(
            (sdata as usize).into()..(edata as usize).into(),
            MapType::Linear,
            PTEFlags::READABLE | PTEFlags::WRITABLE,
        ),
        MapArea::new(
            (sbss_with_stack as usize).into()..(ebss as usize).into(),
            MapType::Linear,
            PTEFlags::READABLE | PTEFlags::WRITABLE,
        ),
        MapArea::new(
            (ekernel as usize).into()..MEMORY_END.into(),
            MapType::Linear,
            PTEFlags::READABLE | PTEFlags::WRITABLE,
        ),
    ];
    for pair in MMIO {
        page_table.map(
            &mut MapArea::new(
                ((*pair).0 + KERNEL_MAP_OFFSET).into()
                    ..((*pair).0 + (*pair).1 + KERNEL_MAP_OFFSET).into(),
                MapType::Linear,
                PTEFlags::READABLE | PTEFlags::WRITABLE,
            ),
            None,
        );
    }

    for area in areas.iter_mut() {
        page_table.map(area, None);
    }

    // XXX 用于映射末尾内核栈的第三级页表（可以映射一个 G，在 k210 上绝对是够了的）
    let vpn = VPN::from(VA(KERNEL_STACK_TOP)).indexes()[0];
    let pte: &mut PageTableEntry = &mut VPN::from(page_table.root.ppn).get_array()[vpn];
    let frame = frame_alloc().unwrap();
    *pte = PageTableEntry::new(frame.ppn, PTEFlags::VALID);
    page_table.frames.push(frame);

    remap_kernel();

    page_table
}

pub fn remap_kernel() {
    for pair in MMIO {
        info!(
            " mmio   regs    [{:#x}, {:#x})",
            (*pair).0 + KERNEL_MAP_OFFSET,
            (*pair).0 + (*pair).1 + KERNEL_MAP_OFFSET
        );
    }
    info!(
        ".text   section [{:#x}, {:#x})",
        stext as usize, etext as usize
    );
    info!(
        ".rodata section [{:#x}, {:#x})",
        srodata as usize, erodata as usize
    );
    info!(
        ".data   section [{:#x}, {:#x})",
        sdata as usize, edata as usize
    );
    info!(
        ".bss    section [{:#x}, {:#x})",
        sbss_with_stack as usize, ebss as usize
    );
    info!(
        " remain memory  [{:#x}, {:#x})",
        ekernel as usize, MEMORY_END
    );
}

/// 一个 task 的所有页表信息
pub struct PageTable {
    /// 根页表的页框
    root: FrameTracker,
    /// 其他子页表的页框
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    /// 创建一个映射了内核的页表
    pub fn new_kernel() -> Self {
        let frame = frame_alloc().unwrap();
        // 不需要清零，直接从 KERNEL_PAGE_TABLE 复制就行！
        VPN::from(frame.ppn)
            .get_array::<PageTableEntry>()
            .copy_from_slice(VPN::from(KERNEL_PAGE_TABLE.root.ppn).get_array::<PageTableEntry>());
        PageTable {
            root: frame,
            frames: vec![],
        }
    }

    /// TODO 考虑页面不够的情况
    pub fn map(&mut self, area: &mut MapArea, data: Option<&[u8]>) {
        match area.map_type {
            MapType::Linear => {
                for vpn in area.vpn_range() {
                    self.map_one(vpn, vpn.into(), area.map_perm);
                }
                // 线性映射的 area 是一段连续的地址，可以直接复制
                if let Some(data) = data {
                    unsafe {
                        from_raw_parts_mut(area.va_range.start.get_mut(), data.len())
                            .copy_from_slice(data);
                    }
                }
            }
            MapType::Framed => {
                match data {
                    // 有数据，且数据长度不为 0
                    Some(data) if data.len() != 0 => {
                        let src_vpn_range = VA::from(data.as_ptr()).floor()
                            ..VA::from(data.as_ptr() as usize + data.len()).ceil();
                        // log::info!("src_vpn_range {:x?}", src_vpn_range);
                        // log::info!("vpn {:x?}", area.vpn_range());
                        // XXX self.va_range.start 和 end 可能并非 4k 对齐的，导致多复制了一些数据
                        for (vpn, src_vpn) in area.vpn_range().zip(src_vpn_range) {
                            let mut dst_frame = frame_alloc().unwrap();
                            self.map_one(vpn, dst_frame.ppn, area.map_perm);
                            // dst_frame 被编译器 deref_mut 成了 &[usize]
                            dst_frame.copy_from_slice(src_vpn.get_array::<usize>());
                            // println!("{:?}", src_vpn.get_array::<usize>());
                            area.data_frames.insert(vpn, dst_frame);
                        }
                    }
                    // 数据长度为 0，说明是 bss 段
                    Some(_) => {
                        // println!("{:x?}", area.va_range);
                        for vpn in area.vpn_range() {
                            let mut dst_frame = frame_alloc().unwrap();
                            self.map_one(vpn, dst_frame.ppn, area.map_perm);
                            // dst_frame 被编译器 deref_mut 成了 &[usize]
                            dst_frame.fill(0);
                            area.data_frames.insert(vpn, dst_frame);
                        }
                    }
                    // 内核栈/用户栈
                    _ => {
                        for vpn in area.vpn_range() {
                            let dst_frame = frame_alloc().unwrap();
                            self.map_one(vpn, dst_frame.ppn, area.map_perm);
                            area.data_frames.insert(vpn, dst_frame);
                        }
                    }
                }
            }
        }
    }

    /// 将 vpn 和 ppn（虚拟页面与物理页面）建立起联系
    pub fn map_one(&mut self, vpn: VPN, ppn: PPN, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::VALID);
    }
    /// unmap 一个页面
    pub fn unmap_one(&mut self, vpn: VPN) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    /// 查找 vpn 虚拟页号对应的 pte
    fn find_pte_create(&mut self, vpn: VPN) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut pte: &mut PageTableEntry = &mut VPN::from(self.root.ppn).get_array()[idxs[0]];
        for &idx in &idxs[1..] {
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::VALID);
                self.frames.push(frame);
            }
            pte = &mut VPN::from(pte.ppn()).get_array()[idx];
        }
        Some(pte)
    }
    /// 查找 vpn 虚拟页号对应的 pte
    fn find_pte(&self, vpn: VPN) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut pte: &mut PageTableEntry = &mut VPN::from(self.root.ppn).get_array()[idxs[0]];
        for &idx in &idxs[1..] {
            if !pte.is_valid() {
                return None;
            }
            pte = &mut VPN::from(pte.ppn()).get_array()[idx];
        }
        Some(pte)
    }
    /// TODO 判断是否为线性 map 的区域
    pub fn translate_va(&self, va: VA) -> Option<PA> {
        self.find_pte(va.floor())
            .map(|pte| PA::from(pte.ppn()) + va.page_offset())
    }
    /// 激活页表
    pub fn activate(&self) {
        // Sv39
        let satp = 8usize << 60 | self.root.ppn.0;
        unsafe {
            satp::write(satp);
            llvm_asm!("sfence.vma" :::: "volatile");
        }
    }
}
