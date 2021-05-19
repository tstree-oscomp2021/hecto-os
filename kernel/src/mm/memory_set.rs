use alloc::collections::{BTreeMap, BTreeSet};

use xmas_elf::{program::Type, ElfFile};

use super::{FrameTracker, VARange, VPNRange, VA, VPN};
use crate::arch::{
    interface::{PageTable, PTE},
    PTEImpl, PageTableImpl,
};

/// 每个 proccess 的地址空间，类似于 Linux 中的 mm_struct
pub struct MemorySet {
    pub page_table: PageTableImpl,
    /// TODO 换成 BTreeMap
    pub areas: BTreeSet<MapArea>,
}

impl MemorySet {
    /// 创建一个映射了内核区域的 MemorySet
    pub fn new_kernel() -> Self {
        Self {
            page_table: PageTableImpl::new_kernel(),
            areas: BTreeSet::new(),
        }
    }

    /// 移除一段 area
    pub fn remove_area(&mut self, va_end: VA) {
        // 此处构造的 MapArea 只需关注 va_range.start
        let area = self
            .areas
            .take(&MapArea::new(
                VA(0)..va_end,
                MapType::Linear,
                PTEImpl::EMPTY,
            ))
            .unwrap();
        for vpn in area.vpn_range() {
            self.page_table.unmap_one(vpn);
        }
    }

    /// 在地址空间插入一段按帧映射的区域，未检查重叠区域
    #[inline]
    pub fn insert_framed_area(&mut self, va_range: VARange, permission: PTEImpl) {
        self.insert_area(MapArea::new(va_range, MapType::Framed, permission), None);
    }

    /// 在地址空间插入一个新的逻辑段，未检查重叠区域
    fn insert_area(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        self.page_table.map(&mut map_area, data);
        self.areas.insert(map_area);
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
            // info!("{:?}", ph);
            let start_addr = ph.virtual_addr() as usize; // segment 在内存中的虚拟起始地址
            let flags = ph.flags(); // RWX 权限
            let offset = ph.offset() as usize; // segment 相对于 ELF 文件开头的偏移
            memory_set.insert_area(
                // TODO va_range 取整
                MapArea::new(
                    start_addr.into()..(start_addr + ph.mem_size() as usize).into(),
                    MapType::Framed,
                    PTEImpl::USER
                        | PTEImpl::readable(flags.is_read())
                        | PTEImpl::writable(flags.is_write())
                        | PTEImpl::executable(flags.is_execute()),
                ),
                Some(&file.input[offset..offset + ph.file_size() as usize]),
            );
        }

        memory_set
    }
}

/// 一段连续地址的虚拟内存映射片段，Linux 中，线性区描述符为 vm_area_struct
/// 实现了 `Ord` Trait（通过比较 va_range.end）
pub struct MapArea {
    pub va_range: VARange,
    pub data_frames: BTreeMap<VPN, FrameTracker>,
    pub map_type: MapType,
    pub map_perm: PTEImpl,
}

impl MapArea {
    pub fn new(va_range: VARange, map_type: MapType, map_perm: PTEImpl) -> Self {
        Self {
            va_range,
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    pub fn vpn_range(&self) -> VPNRange {
        self.va_range.start.floor()..self.va_range.end.ceil()
    }
}

impl Ord for MapArea {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.va_range.end.cmp(&other.va_range.end)
    }
}
impl PartialOrd for MapArea {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.va_range.end.cmp(&other.va_range.end))
    }
}
impl Eq for MapArea {}
impl PartialEq for MapArea {
    fn eq(&self, other: &Self) -> bool {
        self.va_range.end == other.va_range.end
    }
}

pub enum MapType {
    /// 线性映射
    Linear,
    /// 按帧映射
    Framed,
}
