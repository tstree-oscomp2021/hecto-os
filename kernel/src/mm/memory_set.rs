use alloc::{collections::BTreeMap, sync::Arc};

use xmas_elf::{program::Type, ElfFile};

use super::{FrameTracker, VARange, VARangeOrd, VA, VPN};
use crate::{
    arch::{
        interface::{PageTable, PTE},
        PTEImpl, PageTableImpl,
    },
    board::{interface::Config, ConfigImpl},
    frame_alloc,
};

#[derive(Clone, Copy)]
pub enum MapType {
    /// 线性映射
    Linear,
    /// 按帧映射
    Framed,
    /// 设备
    Device,
}

/// 一段连续地址的虚拟内存映射片段，Linux 中，线性区描述符为 vm_area_struct
#[derive(Clone)]
pub struct MapArea {
    pub data_frames: BTreeMap<VPN, FrameTracker>,
    pub map_type: MapType,
    pub map_perm: PTEImpl,
}

/// 每个 proccess 的地址空间，类似于 Linux 中的 mm_struct
pub struct MemorySet {
    pub page_table: PageTableImpl,
    pub areas: BTreeMap<VARangeOrd, MapArea>,
}

impl MemorySet {
    /// 创建一个映射了内核区域的 MemorySet
    pub fn new_kernel() -> Self {
        let mut page_table = PageTableImpl::new_kernel();
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

        Self {
            page_table,
            areas: BTreeMap::<VARangeOrd, MapArea>::new(),
        }
    }

    /// fork 一份 CoW 的 MemorySet
    pub fn fork(&mut self) -> Self {
        let mut new_ms = Self::new_kernel();

        for (range, area) in self.areas.iter() {
            let mut flags = area.map_perm;
            if flags.contains(PTEImpl::WRITABLE) || flags.contains(PTEImpl::COW) {
                flags.remove(PTEImpl::WRITABLE);
                flags.insert(PTEImpl::COW);
                // trace!("{:#x?} {:?}", range.0, flags);
                for (&vpn, frame_tracker) in area.data_frames.iter() {
                    new_ms.page_table.map_one(vpn, frame_tracker.ppn, flags);
                    self.page_table.remap_one(vpn, frame_tracker.ppn, flags);
                }
                new_ms.areas.insert(range.clone(), area.clone());
            }
        }

        new_ms
    }

    pub fn handle_pagefault(&mut self, va: VA) {
        let vpn = va.floor();
        let pte = self.page_table.find_pte(vpn).unwrap();
        debug!("{:?} vpn {:#x} ppn {:#x}", pte, vpn.0, pte.ppn().0);
        if !pte.contains(PTEImpl::COW) {
            panic!("handle_pagefault error");
        }
        pte.remove(PTEImpl::COW);
        pte.insert(PTEImpl::WRITABLE);

        // 接下来判断是否需要复制页面
        let area = self.areas.get_mut(&VARangeOrd(va..va)).unwrap();
        let frame = area.data_frames.get_mut(&vpn).unwrap();
        if Arc::strong_count(frame) > 1 {
            let new_frame = frame_alloc().unwrap();
            VPN::from(new_frame.ppn)
                .get_array::<usize>()
                .copy_from_slice(vpn.get_array());
            pte.set_ppn(new_frame.ppn);
            // trace!("{:?}", pte);
            *frame = new_frame;

            #[cfg(feature = "k210")]
            unsafe {
                asm!("fence", "fence.i", ".word 0x10400073", "fence", "fence.i");
            }
        }
    }

    /// 移除一段 area
    pub fn remove_area(&mut self, va_end: VA) {
        for vpn in self
            .areas
            .remove_entry(&VARangeOrd(va_end..va_end))
            .unwrap()
            .0
            .vpn_range()
        {
            self.page_table.unmap_one(vpn);
        }
    }

    /// 在地址空间插入一段按帧映射的区域，未检查重叠区域
    pub fn insert_framed_area(
        &mut self,
        va_range: VARange,
        map_perm: PTEImpl,
        data: Option<&[u8]>,
    ) {
        let mut area = MapArea {
            data_frames: BTreeMap::new(),
            map_type: MapType::Framed,
            map_perm,
        };
        // debug!("{:#x?} {:?}", va_range, map_perm);
        self.page_table
            .map(VARangeOrd(va_range.clone()), &mut area, data);
        self.areas.insert(VARangeOrd(va_range), area);
    }

    /// 通过 elf 文件创建内存映射（不包括栈）
    pub fn from_elf(file: &ElfFile) -> Self {
        // 建立带有内核映射的 MemorySet
        let mut memory_set = Self::new_kernel();
        // 映射所有 Segment
        for ph in file.program_iter() {
            if ph.get_type() != Ok(Type::Load) {
                continue;
            }
            // println!("{:?}", ph);
            let start_addr = ph.virtual_addr() as usize; // segment 在内存中的虚拟起始地址
            let offset = ph.offset() as usize; // segment 相对于 ELF 文件开头的偏移
            let flags = ph.flags(); // RWX 权限
            let mut map_perm = PTEImpl::USER;
            map_perm.set(PTEImpl::READABLE, flags.is_read());
            map_perm.set(PTEImpl::WRITABLE, flags.is_write());
            map_perm.set(PTEImpl::EXECUTABLE, flags.is_execute());
            memory_set.insert_framed_area(
                // TODO va_range 取整
                start_addr.into()..(start_addr + ph.mem_size() as usize).into(),
                map_perm,
                Some(&file.input[offset..offset + ph.file_size() as usize]),
            );
        }

        memory_set
    }

    // 在低地址区域划分一块可用的区域，返回 va_end
    pub fn alloc_user_area(&mut self, mut size: usize) -> VA {
        size += 2 * ConfigImpl::PAGE_SIZE;

        let mut area_iter = self.areas.keys();
        let mut va_end = VA(round_up!(
            area_iter.next().unwrap().0.end.0,
            ConfigImpl::PAGE_SIZE
        )) + size;
        for area in area_iter {
            if va_end <= area.0.start {
                break;
            }
            va_end = VA(round_up!(area.0.end.0, ConfigImpl::PAGE_SIZE)) + size;
        }

        va_end - ConfigImpl::PAGE_SIZE
    }
}
