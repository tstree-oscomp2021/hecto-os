use core::{ops::BitOr, slice::from_raw_parts_mut};

use super::*;
use crate::{
    arch::{PTEImpl, PageTableImpl},
    board::{interface::Config, ConfigImpl},
};

pub trait PTE: BitOr<Output = Self> + Sized + Copy {
    const EMPTY: Self;
    const READABLE: Self;
    const WRITABLE: Self;
    const EXECUTABLE: Self;
    const USER: Self;
    const VALID: Self;

    fn new(ppn: PPN, flags: Self) -> Self;
    fn ppn(self) -> PPN;
    fn is_valid(self) -> bool;

    #[inline]
    fn readable(value: bool) -> Self {
        if value {
            Self::READABLE
        } else {
            Self::EMPTY
        }
    }
    #[inline]
    fn writable(value: bool) -> Self {
        if value {
            Self::WRITABLE
        } else {
            Self::EMPTY
        }
    }
    #[inline]
    fn executable(value: bool) -> Self {
        if value {
            Self::EXECUTABLE
        } else {
            Self::EMPTY
        }
    }
}

pub trait PageTable {
    /// 查找 vpn 虚拟页号对应的 pte
    fn find_pte_create(&mut self, vpn: VPN) -> Option<&mut PTEImpl>;
    /// 查找 vpn 虚拟页号对应的 pte
    fn find_pte(&self, vpn: VPN) -> Option<&mut PTEImpl>;
    /// 激活页表
    fn activate(&self);
    fn new_kernel() -> Self;

    /// TODO 判断是否为线性 map 的区域
    fn translate_va(&self, va: VA) -> Option<PA> {
        self.find_pte(va.floor())
            .map(|pte: &mut PTEImpl| PA::from(pte.ppn()) + va.page_offset())
    }

    /// 将 vpn 和 ppn（虚拟页面与物理页面）建立起联系
    fn map_one(&mut self, vpn: VPN, ppn: PPN, flags: PTEImpl) {
        let pte: &mut PTEImpl = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PTEImpl::new(ppn, flags | PTEImpl::VALID);
    }
    /// unmap 一个页面
    fn unmap_one(&mut self, vpn: VPN) {
        let pte: &mut PTEImpl = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PTEImpl::EMPTY;
    }

    /// TODO 考虑页面不够的情况
    fn map(&mut self, area: &mut MapArea, data: Option<&[u8]>) {
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
                        // info!("src_vpn_range {:x?}", src_vpn_range);
                        // info!("vpn {:x?}", area.vpn_range());
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
}

use alloc::vec;

use lazy_static::lazy_static;

use crate::{board::*, KERNEL_PROCESS};

lazy_static! {
    /// 请通过内核进程而非此变量来映射内核栈，因为映射涉及到页框的创建和保存
    pub static ref KERNEL_PAGE_TABLE: &'static PageTableImpl =
        unsafe { &*(&KERNEL_PROCESS.inner.lock().memory_set.page_table as *const PageTableImpl) };
}

// TODO 应放在 board 里
/// 仅在初始化 KERNEL_PROCESS 时被调用
pub fn kernel_page_table() -> PageTableImpl {
    let mut frame = frame_alloc().unwrap();
    frame.zero();
    let mut page_table = PageTableImpl {
        root: frame,
        frames: vec![],
    };
    // TODO 表驱动
    let mut areas = [
        MapArea::new(
            (stext as usize).into()..(etext as usize).into(),
            MapType::Linear,
            PTEImpl::READABLE | PTEImpl::EXECUTABLE,
        ),
        MapArea::new(
            (srodata as usize).into()..(erodata as usize).into(),
            MapType::Linear,
            PTEImpl::READABLE,
        ),
        MapArea::new(
            (sdata as usize).into()..(edata as usize).into(),
            MapType::Linear,
            PTEImpl::READABLE | PTEImpl::WRITABLE,
        ),
        MapArea::new(
            (sbss_with_stack as usize).into()..(ebss as usize).into(),
            MapType::Linear,
            PTEImpl::READABLE | PTEImpl::WRITABLE,
        ),
        MapArea::new(
            (ekernel as usize).into()..ConfigImpl::MEMORY_END.into(),
            MapType::Linear,
            PTEImpl::READABLE | PTEImpl::WRITABLE,
        ),
    ];
    for pair in ConfigImpl::MMIO {
        page_table.map(
            &mut MapArea::new(
                (pair.0 + ConfigImpl::KERNEL_MAP_OFFSET).into()
                    ..(pair.0 + pair.1 + ConfigImpl::KERNEL_MAP_OFFSET).into(),
                MapType::Linear,
                PTEImpl::READABLE | PTEImpl::WRITABLE,
            ),
            None,
        );
    }

    for area in areas.iter_mut() {
        page_table.map(area, None);
    }

    // XXX 用于映射末尾内核栈的第三级页表（可以映射一个 G，在 k210 上绝对是够了的）
    let vpn = VPN::from(VA(ConfigImpl::KERNEL_STACK_TOP)).indexes()[0];
    let pte: &mut PTEImpl = &mut VPN::from(page_table.root.ppn).get_array()[vpn];
    let frame = frame_alloc().unwrap();
    *pte = PTEImpl::new(frame.ppn, PTEImpl::VALID);
    page_table.frames.push(frame);

    // remap_kernel();

    page_table
}
