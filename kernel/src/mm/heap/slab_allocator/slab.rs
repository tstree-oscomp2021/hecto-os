use core::{
    alloc::{GlobalAlloc, Layout},
    cmp::max,
    mem::size_of,
    ptr::null_mut,
};

use super::page::*;
use crate::VA;

#[derive(Copy, Clone, Default, Debug)]
pub struct SlabAllocator;

struct SlabChunk {
    c_next: *mut SlabChunk,
}

struct SlabZone {
    z_magic: u32,
    z_nfree: u32,
    z_nmax: u32,

    z_next: *mut SlabZone,
    z_baseptr: *mut u8,

    z_uindex: u32,
    z_chunksize: u32,
    z_zoneindex: u32,
    z_freechunk: *mut SlabChunk,
}

enum MemUsage {
    Free,
    Zone(usize),
    // 作为 Page 被分配，此为分配的页数
    Page(usize),
}
static mut MEM_USAGE: *mut MemUsage = null_mut();
static mut HEAP_START: usize = 0;
static mut HEAP_END: usize = 0;
static mut ZONE_ARRAY: [*mut SlabZone; 72] = [null_mut(); 72];
static mut ZONE_FREE: *mut SlabZone = null_mut();
static mut ZONE_FREE_CNT: usize = 0;

const ZALLOC_SLAB_MAGIC: u32 = 0x51ab51ab;

#[macro_export]
macro_rules! btokup {
    ($addr: expr) => {{
        MEM_USAGE.add((($addr) as usize - HEAP_START) / 4096)
    }};
}

unsafe impl GlobalAlloc for SlabAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        Self::malloc(max(layout.size(), layout.align()))
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        Self::free(ptr)
    }
}

impl SlabAllocator {
    const PAGE_SIZE: usize = 4 * 1024;
    const ZONE_LIMIT: usize = 16 * 1024;
    const ZONE_SIZE: usize = 32 * 1024;
    const ZONE_PAGE_CNT: usize = Self::ZONE_SIZE / Self::PAGE_SIZE;

    pub unsafe fn init(&mut self, start: usize, end: usize) {
        HEAP_START = round_up!(start, Self::PAGE_SIZE);
        HEAP_END = round_down!(end, Self::PAGE_SIZE);
        assert!(HEAP_START < HEAP_END);

        let limsize = HEAP_END - HEAP_START;
        let npages = limsize / Self::PAGE_SIZE;

        // 初始化 pages
        page_init(VA(HEAP_START), npages);

        MEM_USAGE = page_alloc(
            round_up!(npages * size_of::<MemUsage>(), Self::PAGE_SIZE) / Self::PAGE_SIZE,
        ) as *mut MemUsage;

        debug!("init end");
    }

    unsafe fn zone_index(bytes: *mut usize) -> usize {
        let mut n = *bytes;

        if n < 128 {
            n = round_up!(n, 8);
            *bytes = n;
            return n / 8 - 1;
        }
        if n < 256 {
            n = round_up!(n, 16);
            *bytes = n;
            return n / 16 + 7;
        }
        if n < 8192 {
            if n < 512 {
                n = round_up!(n, 32);
                *bytes = n;
                return n / 32 + 15;
            }
            if n < 1024 {
                n = round_up!(n, 64);
                *bytes = n;
                return n / 64 + 23;
            }
            if n < 2048 {
                n = round_up!(n, 128);
                *bytes = n;
                return n / 128 + 31;
            }
            if n < 4096 {
                n = round_up!(n, 256);
                *bytes = n;
                return n / 256 + 39;
            }
            n = round_up!(n, 512);
            *bytes = n;
            return n / 512 + 47;
        }
        if n < 16384 {
            n = round_up!(n, 1024);
            *bytes = n;
            return n / 1024 + 55;
        }

        println!("Unexpected byte count {}", n);

        return 0;
    }

    #[allow(dead_code)]
    unsafe fn malloc(mut size: usize) -> *mut u8 {
        let chunk: *mut SlabChunk;
        let mut kup: *mut MemUsage;
        let mut z: *mut SlabZone;
        let zi: usize;

        if size == 0 {
            return null_mut();
        }

        // 如果超过了 ZONE_LIMIT，直接 page_alloc
        if size >= Self::ZONE_LIMIT {
            size = round_up!(size, Self::PAGE_SIZE);
            chunk = page_alloc(size / Self::PAGE_SIZE) as *mut SlabChunk;
            if chunk.is_null() {
                return null_mut();
            }

            // 在 memusage 数组里记录页面被使用的信息
            *btokup!(chunk) = MemUsage::Page(size / Self::PAGE_SIZE);

            return chunk as *mut u8;
        }

        zi = Self::zone_index(&mut size);
        debug_assert!(zi < 72);
        z = ZONE_ARRAY[zi];

        // 如果 size 对应的 slab_zone 链表存在，则该取该链表的第一个 zone 节点的空闲块
        if !z.is_null() {
            let z = &mut *z;
            z.z_nfree -= 1;
            // 如果之后没了空闲块，将该 slab_zone 从链表中移除
            if z.z_nfree == 0 {
                ZONE_ARRAY[zi] = z.z_next;
                z.z_next = null_mut();
            }

            // 尽量使用 never-before-used-memory area 中的 chunk
            if z.z_uindex != z.z_nmax {
                chunk = z.z_baseptr.add(z.z_uindex as usize * size) as *mut SlabChunk;
                z.z_uindex += 1;
            } else {
                chunk = z.z_freechunk;
                z.z_freechunk = (&*chunk).c_next;
            }

            return chunk as *mut u8;
        }

        // 没有用来分配该 size 的 slab_zone 了
        z = ZONE_FREE;
        if !z.is_null() {
            ZONE_FREE = (&*z).z_next;
            ZONE_FREE_CNT -= 1;
        } else {
            z = page_alloc(Self::ZONE_SIZE / Self::PAGE_SIZE) as *mut SlabZone;
            if z.is_null() {
                return null_mut();
            }
            kup = btokup!(z);
            for off in 0..Self::ZONE_PAGE_CNT {
                *kup = MemUsage::Zone(off);
                kup = kup.add(1);
            }
        }
        core::slice::from_raw_parts_mut(z as *mut u8, size_of::<SlabZone>()).fill(0);
        let mut off = size_of::<SlabZone>();
        if (size | (size - 1)) + 1 == (size << 1) {
            off = (off + size - 1) & !(size - 1);
        } else {
            off = round_up!(off, 8);
        }
        let z = &mut *z;
        z.z_magic = ZALLOC_SLAB_MAGIC;
        z.z_zoneindex = zi as u32;
        z.z_nmax = ((Self::ZONE_SIZE - off) / size) as u32;
        z.z_nfree = z.z_nmax - 1; // 此次分配用掉了一个
        z.z_baseptr = (z as *const SlabZone as *mut u8).add(off);
        z.z_uindex = 1;
        z.z_chunksize = size as u32;
        z.z_next = null_mut();
        ZONE_ARRAY[zi] = z;

        z.z_baseptr
    }

    unsafe fn free(ptr: *const u8) {
        if ptr.is_null() {
            return;
        }

        let mut chunk: &mut SlabChunk;
        let mut kup: *mut MemUsage;

        kup = btokup!(ptr);
        match &mut *kup {
            MemUsage::Free => {
                panic!("should not be MemUsage::Free");
            }
            MemUsage::Zone(size) => {
                let z = &mut *(((ptr as usize & !(Self::PAGE_SIZE - 1)) - *size * Self::PAGE_SIZE)
                    as *mut SlabZone);
                assert_eq!(z.z_magic, ZALLOC_SLAB_MAGIC);
                chunk = &mut *(ptr as *mut SlabChunk);
                chunk.c_next = z.z_freechunk;
                z.z_freechunk = chunk;
                z.z_nfree += 1;
                if z.z_nfree == 1 {
                    z.z_next = ZONE_ARRAY[z.z_zoneindex as usize];
                    ZONE_ARRAY[z.z_zoneindex as usize] = z;
                }

                // 如果该 zone 完全 free 了，并且链表里存在其他 zone 用来分配，
                // 我们就可以放心将该 zone 移入 zone_free
                if z.z_nfree == z.z_nmax
                    && (!z.z_next.is_null() || ZONE_ARRAY[z.z_zoneindex as usize] != z)
                {
                    // 从 zone array list 中移除
                    let mut pz: &mut *mut SlabZone = &mut ZONE_ARRAY[z.z_zoneindex as usize];
                    while *pz != z {
                        pz = &mut (&mut **pz).z_next;
                    }
                    *pz = z.z_next;
                    z.z_magic = 0;
                    // 添加到 free zone list 中
                    if ZONE_FREE_CNT < 2 {
                        z.z_next = ZONE_FREE;
                        ZONE_FREE = z;
                        ZONE_FREE_CNT += 1;
                    }
                    // 回收 page
                    else {
                        kup = btokup!(z as *const _);
                        for _ in 0..ZONE_FREE_CNT {
                            *kup = MemUsage::Free;
                            kup = kup.add(1);
                        }

                        page_free(
                            VA(z as *const _ as usize),
                            Self::ZONE_SIZE / Self::PAGE_SIZE,
                        );
                    }
                }
            }
            MemUsage::Page(size) => {
                page_free(VA(ptr as usize), *size);
                *size = 0;
            }
            #[allow(unreachable_patterns)]
            _ => {
                panic!("BUG!!");
            }
        }
    }
}
