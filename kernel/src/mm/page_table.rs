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
    fn set_ppn(&mut self, ppn: PPN);
    fn ppn(self) -> PPN;
    fn is_valid(self) -> bool;
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

    /// 重新映射
    fn remap_one(&mut self, vpn: VPN, ppn: PPN, flags: PTEImpl) {
        let pte: &mut PTEImpl = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:x} has not been mapped before", vpn.0);
        *pte = PTEImpl::new(ppn, flags | PTEImpl::VALID);
    }

    /// 将 vpn 和 ppn（虚拟页面与物理页面）建立起联系
    fn map_one(&mut self, vpn: VPN, ppn: PPN, flags: PTEImpl) {
        let pte: &mut PTEImpl = self.find_pte_create(vpn).unwrap();
        assert!(
            !pte.is_valid(),
            "vpn {:#x} ppn {:#x} has been mapped before, {:?}, ppn {:#x}",
            vpn.0,
            ppn.0,
            pte,
            pte.ppn().0
        );
        // println!("map vpn {:#x}, ppn {:#x}", vpn.0, ppn.0);
        *pte = PTEImpl::new(ppn, flags | PTEImpl::VALID);
    }
    /// unmap 一个页面
    fn unmap_one(&mut self, vpn: VPN) {
        let pte: &mut PTEImpl = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:x} has not been mapped before", vpn.0);
        *pte = PTEImpl::EMPTY;
    }

    /// TODO 考虑页面不够的情况
    fn map(&mut self, va_range: VARangeOrd, area: &mut MapArea, data: Option<&[u8]>) {
        match area.map_type {
            MapType::Linear => {
                for vpn in va_range.vpn_range() {
                    self.map_one(vpn, vpn.into(), area.map_perm);
                }
                // 线性映射的 area 是一段连续的地址，可以直接复制
                if let Some(data) = data {
                    unsafe {
                        from_raw_parts_mut(va_range.0.start.get_mut(), data.len())
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
                        // println!("src_vpn_range {:x?}", src_vpn_range);
                        // println!("vpn {:x?}", va_range.vpn_range());
                        // XXX va_range.start 和 end 可能并非 4k 对齐的，导致多复制了一些数据
                        for (vpn, src_vpn) in va_range.vpn_range().zip(src_vpn_range) {
                            let dst_frame = frame_alloc().unwrap();
                            self.map_one(vpn, dst_frame.ppn, area.map_perm);
                            VPN::from(dst_frame.ppn)
                                .get_array()
                                .copy_from_slice(src_vpn.get_array::<usize>());
                            // println!("{:?}", src_vpn.get_array::<usize>());
                            area.data_frames.insert(vpn, dst_frame);
                        }
                    }
                    // 数据长度为 0，说明是 bss 段
                    Some(_) => {
                        for vpn in va_range.vpn_range() {
                            let dst_frame = frame_alloc().unwrap();
                            self.map_one(vpn, dst_frame.ppn, area.map_perm);
                            VPN::from(dst_frame.ppn).get_array().fill(0usize);
                            area.data_frames.insert(vpn, dst_frame);
                        }
                    }
                    // 内核栈/用户栈
                    _ => {
                        for vpn in va_range.vpn_range() {
                            let dst_frame = frame_alloc().unwrap();
                            self.map_one(vpn, dst_frame.ppn, area.map_perm);
                            area.data_frames.insert(vpn, dst_frame);
                        }
                    }
                }
            }
            MapType::Device => {
                for vpn in va_range.vpn_range() {
                    self.map_one(vpn, PPN(vpn.0), area.map_perm);
                }
            }
        }
    }
}

use alloc::{collections::BTreeMap, vec};

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
    let frame = frame_alloc().unwrap();
    VPN::from(frame.ppn)
        .get_array::<PTEImpl>()
        .fill(PTEImpl::EMPTY);
    let mut page_table = PageTableImpl {
        root: frame,
        frames: vec![],
    };

    let areas: [(VARange, PTEImpl); 5] = [
        (
            (stext as usize).into()..(etext as usize).into(),
            PTEImpl::READABLE | PTEImpl::EXECUTABLE,
        ),
        (
            (srodata as usize).into()..(erodata as usize).into(),
            PTEImpl::READABLE,
        ),
        (
            (sdata as usize).into()..(edata as usize).into(),
            PTEImpl::READABLE | PTEImpl::WRITABLE,
        ),
        (
            (sbss_with_stack as usize).into()..(ebss as usize).into(),
            PTEImpl::READABLE | PTEImpl::WRITABLE,
        ),
        (
            (ekernel as usize).into()..ConfigImpl::MEMORY_END.into(),
            PTEImpl::READABLE | PTEImpl::WRITABLE,
        ),
    ];
    for area in areas {
        page_table.map(
            VARangeOrd(area.0),
            &mut MapArea {
                data_frames: BTreeMap::new(),
                map_type: MapType::Linear,
                map_perm: area.1,
            },
            None,
        );
    }
    for pair in ConfigImpl::MMIO {
        page_table.map(
            VARangeOrd(VA(pair.0)..VA(pair.0 + pair.1)),
            &mut MapArea {
                data_frames: BTreeMap::new(),
                map_type: MapType::Device,
                map_perm: PTEImpl::READABLE | PTEImpl::WRITABLE,
            },
            None,
        );
    }

    // XXX 用于映射末尾内核栈的第三级页表（可以映射一个 G，在 k210 上绝对是够了的）
    let vpn = VA(ConfigImpl::KERNEL_STACK_TOP).floor().indexes()[0];
    let pte: &mut PTEImpl = &mut VPN::from(page_table.root.ppn).get_array()[vpn];
    let frame = frame_alloc().unwrap();
    VPN::from(frame.ppn)
        .get_array::<PTEImpl>()
        .fill(PTEImpl::EMPTY);
    *pte = PTEImpl::new(frame.ppn, PTEImpl::VALID);
    page_table.frames.push(frame);

    // remap_kernel();

    page_table
}
