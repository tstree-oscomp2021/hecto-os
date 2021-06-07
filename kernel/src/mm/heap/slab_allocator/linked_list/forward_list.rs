use core::ops::{Deref, DerefMut};

/// 前向（单向）链表
///
/// `T` 实际上已经存在与内存中
pub struct ForwardList<T> {
    head: *mut ForwardListNode<T>,
}

pub struct ForwardListNode<T> {
    next: *mut ForwardListNode<T>,
    element: T,
}

impl<T> ForwardList<T> {
    pub const fn new() -> Self {
        Self {
            head: core::ptr::null_mut(),
        }
    }

    /// Remove node from list.
    /// This operation should compute in O(n) time.
    /// TODO 换成双向链表
    pub unsafe fn remove(&mut self, node: *mut ForwardListNode<T>) {
        let mut prev = &mut self.head;
        while *prev != node {
            prev = &mut (&mut **prev).next;
        }
        *prev = (&*node).next;
    }

    pub unsafe fn set_head(&mut self, new: *mut ForwardListNode<T>) {
        self.head = new;
    }

    pub unsafe fn get_head(&mut self) -> *mut ForwardListNode<T> {
        self.head
    }

    pub unsafe fn push_front(&mut self, new: *mut ForwardListNode<T>) {
        (&mut *new).next = core::mem::replace(&mut self.head, new);
    }

    pub unsafe fn pop_front(&mut self) {
        self.head = (&*self.head).next;
    }
}
impl<T> Deref for ForwardList<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.head }
    }
}
impl<T> DerefMut for ForwardList<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.head }
    }
}

impl<T> ForwardListNode<T> {
    #[allow(dead_code)]
    pub unsafe fn insert_after(&mut self, new: *mut ForwardListNode<T>) {
        (&mut *new).next = core::mem::replace(&mut self.next, new);
    }
    #[allow(dead_code)]
    pub unsafe fn erase_after(&mut self) {
        self.next = (&*self.next).next;
    }

    pub unsafe fn has_next(&self) -> bool {
        !self.next.is_null()
    }
    #[allow(dead_code)]
    pub unsafe fn get_next(&self) -> *mut ForwardListNode<T> {
        self.next
    }
}
impl<T> Deref for ForwardListNode<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.element
    }
}
impl<T> DerefMut for ForwardListNode<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.element
    }
}
