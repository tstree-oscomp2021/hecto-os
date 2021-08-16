use core::{intrinsics::transmute, mem::size_of, ptr::null_mut};

use crate::VA;

/// 一段连续的 Pages
struct PageHead {
    /// 下一个 free Page 区域
    next: *mut PageHead,
    /// pages 数量
    npages: usize,
    /// 剩余的部分
    _padding: [u8; 4096 - size_of::<*mut PageHead>() - size_of::<usize>()],
}

impl PageHead {
    /// 得到下一个相邻的 page
    fn get_right_ptr(&self) -> *const PageHead {
        unsafe { self.as_ptr().add(self.npages) }
    }

    /// UNSAFE! 下一个可能是 None
    unsafe fn get_next(&self) -> &PageHead {
        transmute(self.next)
    }

    fn as_ptr(&self) -> *const PageHead {
        self as *const PageHead
    }

    fn as_mut_ptr(&mut self) -> *mut PageHead {
        self as *mut PageHead
    }

    /// other 是否是下一个相邻的 Page 区（不管使用或未使用）
    fn is_right(&self, other: &PageHead) -> bool {
        self.get_right_ptr() == other
    }

    /// 下一个相邻的 Page 区是否为 free
    fn is_right_free(&self) -> bool {
        self.get_right_ptr() == self.next
    }
}

/// 指向一个 PageHead 中的 Node<PageHead>
static mut FREE_PAGE_LIST: *mut PageHead = null_mut();

#[allow(dead_code)]
pub unsafe fn page_free(addr: VA, npages: usize) {
    let cur: &mut PageHead = addr.as_mut();
    let mut prev = &mut FREE_PAGE_LIST;
    let mut iter_ptr = *prev;

    while !iter_ptr.is_null() {
        let iter = &mut *iter_ptr;
        // 如果 cur 左边的是 free page
        if iter.is_right(cur) {
            iter.npages += npages;
            // 如果 cur 右边的也是 free
            if iter.is_right_free() {
                iter.npages += iter.get_next().npages;
                iter.next = iter.get_next().next;
            }
            break;
        }

        // 如果 cur 右边的是 free page
        if cur.is_right(iter) {
            cur.npages = iter.npages + npages;
            cur.next = iter.next;
            // prev 是 iter 的上一个页面的 next field 的可变引用，指向 iter
            *prev = cur;
            break;
        }

        // 如果 cur 左右相邻的都不是 free page
        if cur.get_right_ptr() < iter {
            cur.npages = npages;
            cur.next = iter;
            *prev = cur;
            break;
        }

        prev = &mut iter.next;
        iter_ptr = *prev;
    }
}

#[allow(dead_code)]
pub unsafe fn page_alloc(npages: usize) -> VA {
    if npages == 0 {
        return VA(0);
    }

    let mut prev = &mut FREE_PAGE_LIST;
    let mut iter_ptr = *prev;
    let mut n;

    while !iter_ptr.is_null() {
        let iter = &mut *iter_ptr;

        if iter.npages > npages {
            // 分割
            n = &mut *iter.as_mut_ptr().add(npages);
            n.next = iter.next;
            n.npages = iter.npages - npages;
            *prev = n;
            break;
        }

        if iter.npages == npages {
            *prev = iter.next;
            break;
        }

        prev = &mut iter.next;
        iter_ptr = *prev;
    }

    VA(iter_ptr as usize)
}

pub unsafe fn page_init(addr: VA, npages: usize) {
    let cur: &mut PageHead = addr.as_mut();
    cur.npages = npages;
    cur.next = null_mut();

    FREE_PAGE_LIST = cur;
}
