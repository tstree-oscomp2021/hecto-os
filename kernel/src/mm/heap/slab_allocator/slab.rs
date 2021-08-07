use core::{
    alloc::{GlobalAlloc, Layout},
    cmp::{max, min},
    mem::size_of,
    ptr::null_mut,
};

use super::{linked_list::forward_list::*, page::*};
use crate::VA;

/// Slab 分配器
#[derive(Copy, Clone)]
pub struct SlabAllocator;

/// 内存块，有 [`NZONES`]+ 种大小
struct SlabChunk;
type SlabChunkList = ForwardList<SlabChunk>;
type SlabChunkNode = ForwardListNode<SlabChunk>;

/// 用来描述一个 [`ZONE_SIZE`] 大小的区域，[`SlabZoneNode`]
/// 结构体位于该区域起始处
struct SlabZone {
    z_magic: u32,
    /// 剩余的空闲 [`SlabChunkNode`] 数
    z_nfree: u32,
    /// 该 SlabZone 刚被创建时的空闲 [`SlabChunkNode`] 个数
    z_nmax: u32,
    /// 该 SlabZone 创建后就从未被使用过的 [`SlabChunkNode`] 的索引
    z_uindex: u32,
    /// 该 [`SlabZoneNode`] 中的第 0 个 [`SlabChunkNode`] 的地址
    z_baseptr: *mut SlabChunkNode,
    /// 所属链表在 [`ZONE_ARRAY`] 中的索引
    z_zoneindex: u32,
    /// 空闲 [`SlabChunkNode`] 链表
    z_freechunk: SlabChunkList,
}
type SlabZoneList = ForwardList<SlabZone>;
type SlabZoneNode = ForwardListNode<SlabZone>;

/// 记录每 page 的使用情况
#[repr(u64)]
enum PageUsage {
    /// 未被使用
    Free,
    /// 用作 Slab alloc，相对于 SlabZoneNode 所在 page 的偏移量
    Zone(u32),
    /// 用作 Page alloc，分配的页数
    Page(u32),
}
/// PageUsage 数组
static mut PAGE_USAGE_ARRAY: *mut PageUsage = null_mut();

static mut HEAP_START: usize = 0;
static mut HEAP_END: usize = 0;

/// [`SlabZoneList`] 的数量
const NZONES: usize = 72;
/// [`NZONES`] 个 [`SlabZoneNode`] 链表，每个链表各自用来分配不同大小的块
static mut ZONE_ARRAY: [SlabZoneList; NZONES] = [const { SlabZoneList::new() }; NZONES];
/// 空闲 [`SlabZoneNode`] 链表
static mut ZONE_FREE: SlabZoneList = ForwardList::new();
/// 空闲 [`SlabZoneNode`] 链表中的 SlabZone 个数
static mut ZONE_FREE_CNT: usize = 0;
/// 空闲 [`SlabZoneNode`] 链表中的 SlabZone 个数上限
const ZONE_FREE_CNT_LIMIT: usize = 4;
/// Page 的大小
const PAGE_SIZE: usize = 4 * 1024;
/// SlabChunk 大小上限，超过此数值将直接通过 page_alloc 分配
static mut CHUNK_LIMIT: usize = 0;
/// SlabZone 大小
static mut ZONE_SIZE: usize = 0;
/// 一个 SlabZone 所占的 page 数量
static mut ZONE_PAGE_CNT: usize = 0;

const ZALLOC_SLAB_MAGIC: u32 = 0x51ab51ab;

/// 获取地址所在 Page 的 [`PageUsage`]
#[macro_export]
macro_rules! btokup {
    ($addr: expr) => {{
        PAGE_USAGE_ARRAY.add((($addr) as *const _ as usize - HEAP_START) / 4096)
    }};
}

unsafe impl GlobalAlloc for SlabAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        Self::malloc(max(layout.size(), layout.align())) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        Self::free(ptr as *mut SlabChunkNode)
    }
}

impl SlabAllocator {
    pub unsafe fn init(&mut self, start: usize, end: usize) {
        HEAP_START = round_up!(start, PAGE_SIZE);
        HEAP_END = round_down!(end, PAGE_SIZE);
        assert!(HEAP_START < HEAP_END);

        let limsize = HEAP_END - HEAP_START;
        let npages = limsize / PAGE_SIZE;
        // 初始化 pages
        page_init(VA(HEAP_START), npages);

        // 计算合适的 ZONE_SIZE 大小
        ZONE_SIZE = 32 * 1024;
        while ZONE_SIZE < 128 * 1024 && (ZONE_SIZE << 1) < (limsize / 1024) {
            ZONE_SIZE <<= 1;
        }
        ZONE_PAGE_CNT = ZONE_SIZE / PAGE_SIZE;
        // 计算合适的 CHUNK_LIMIT 大小
        CHUNK_LIMIT = min(ZONE_SIZE / 4, 16 * 1024);

        PAGE_USAGE_ARRAY =
            page_alloc(round_up!(npages * size_of::<PageUsage>(), PAGE_SIZE) / PAGE_SIZE).as_mut();
    }

    unsafe fn zone_index(bytes: *mut usize) -> usize {
        debug_assert!(*bytes <= CHUNK_LIMIT);

        let mut n = *bytes;
        let i: usize = max(
            7,
            0usize.leading_zeros() as usize - n.leading_zeros() as usize,
        ) - 1
            - 3;

        n = round_up!(n, 1 << i);
        *bytes = n;
        return (n >> i) + 8 * (i - 3) - 1;
    }

    /// 分配一个 SlabChunkNode
    unsafe fn malloc(mut size: usize) -> *mut SlabChunkNode {
        let chunk: *mut SlabChunkNode;
        let kup: *mut PageUsage;
        let mut zn: *mut SlabZoneNode;
        let zi: usize;

        if size == 0 {
            return null_mut();
        }

        // 如果超过了 CHUNK_LIMIT，直接 page_alloc
        if size > CHUNK_LIMIT {
            let npages = round_up!(size, PAGE_SIZE) / PAGE_SIZE;
            chunk = page_alloc(npages).as_mut_ptr();
            if chunk.is_null() {
                return null_mut();
            }
            // 在 memusage 数组里记录页面被使用的信息
            *btokup!(chunk) = PageUsage::Page(npages as u32);

            return chunk;
        }

        zi = Self::zone_index(&mut size);
        debug_assert!(zi < NZONES);

        // 如果该 size 对应的 SlabZoneList 链表上存在 SlabZoneNode
        zn = ZONE_ARRAY[zi].get_head();
        if let Some(zn) = zn.as_mut() {
            zn.z_nfree -= 1;
            // 如果之后没了空闲块，将该 SlabZoneNode 从链表中移除
            if zn.z_nfree == 0 {
                ZONE_ARRAY[zi].pop_front();
            }

            // 先使用该 SlabZoneNode 创建后就从未被使用过的 SlabChunkNode
            if zn.z_uindex != zn.z_nmax {
                chunk = (zn.z_baseptr as *mut u8).add(zn.z_uindex as usize * size)
                    as *mut SlabChunkNode;
                zn.z_uindex += 1;
            } else {
                chunk = zn.z_freechunk.get_head();
                zn.z_freechunk.pop_front();
            }

            return chunk;
        }

        // 没有用来分配该 size 的 SlabZoneNode 了，就整一个空闲的 SlabZoneNode
        zn = ZONE_FREE.get_head();
        if !zn.is_null() {
            ZONE_FREE.pop_front();
            ZONE_FREE_CNT -= 1;
        } else {
            zn = page_alloc(ZONE_SIZE / PAGE_SIZE).as_mut();
            if zn.is_null() {
                return null_mut();
            }
            kup = btokup!(zn);
            for off in 0..ZONE_PAGE_CNT {
                *kup.add(off) = PageUsage::Zone(off as u32);
            }
        }
        zn.write_bytes(0, 1);
        let mut off = size_of::<SlabZoneNode>();
        if (size | (size - 1)) + 1 == (size << 1) {
            off = (off + size - 1) & !(size - 1);
        } else {
            off = round_up!(off, 8);
        }
        let z = &mut *zn;
        z.z_magic = ZALLOC_SLAB_MAGIC;
        z.z_zoneindex = zi as u32;
        z.z_nmax = ((ZONE_SIZE - off) / size) as u32;
        z.z_nfree = z.z_nmax - 1; // 此次分配用掉了一个
        z.z_baseptr = (zn as *mut u8).add(off) as *mut SlabChunkNode;
        z.z_uindex = 1;
        ZONE_ARRAY[zi].set_head(zn);

        z.z_baseptr
    }

    /// 回收 SlabChunkNode
    unsafe fn free(chunk: *mut SlabChunkNode) {
        if chunk.is_null() {
            return;
        }

        let mut kup = btokup!(chunk);
        match &mut *kup {
            PageUsage::Free => {
                panic!("should not be PageUsage::Free");
            }
            PageUsage::Zone(size) => {
                let zn: &mut SlabZoneNode =
                    VA(round_down!(chunk, PAGE_SIZE) - *size as usize * PAGE_SIZE).as_mut();
                debug_assert_eq!(zn.z_magic, ZALLOC_SLAB_MAGIC);
                zn.z_freechunk.push_front(chunk);
                zn.z_nfree += 1;
                if zn.z_nfree == 1 {
                    ZONE_ARRAY[zn.z_zoneindex as usize].push_front(zn);
                    return;
                }

                // 如果该 SlabZoneNode 完全 free 了，并且链表里存在其他 SlabZoneNode 用来分配，
                // 我们就可以放心地将该 SlabZoneNode 移入 ZONE_FREE 或者回收掉
                if zn.z_nfree == zn.z_nmax
                    && (zn.has_next() || ZONE_ARRAY[zn.z_zoneindex as usize].get_head() != zn)
                {
                    zn.z_magic = 0;
                    // 从 ZONE_ARRAY list 中移除 zn
                    ZONE_ARRAY[zn.z_zoneindex as usize].remove(zn);
                    // 添加到 ZONE_FREE list 中
                    if ZONE_FREE_CNT < ZONE_FREE_CNT_LIMIT {
                        ZONE_FREE.push_front(zn);
                        ZONE_FREE_CNT += 1;
                    }
                    // 回收 SlabZoneNode
                    else {
                        kup = btokup!(zn);
                        for i in 0..ZONE_PAGE_CNT {
                            *kup.add(i) = PageUsage::Free;
                        }
                        page_free(zn.into(), ZONE_SIZE / PAGE_SIZE);
                    }
                }
            }
            PageUsage::Page(size) => {
                page_free(chunk.into(), *size as usize);
                *size = 0;
            }
            #[allow(unreachable_patterns)]
            _ => {
                panic!("BUG!!");
            }
        }
    }
}
