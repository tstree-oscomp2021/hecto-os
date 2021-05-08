use super::{KERNEL_MAP_OFFSET, PAGE_SIZE, PAGE_SIZE_BITS};
use core::{fmt::Debug, iter::Step, mem::size_of};

#[repr(C)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
// #[rustc_layout_scalar_valid_range_start(1)]
// #[rustc_nonnull_optimization_guaranteed]
pub struct PA(pub usize);

#[repr(C)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
// #[rustc_layout_scalar_valid_range_start(1)]
// #[rustc_nonnull_optimization_guaranteed]
pub struct VA(pub usize);

#[repr(C)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct PPN(pub usize);

#[repr(C)]
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct VPN(pub usize);

/// 从指针转换为虚拟地址
impl<T> From<*const T> for VA {
    fn from(pointer: *const T) -> Self {
        Self(pointer as usize)
    }
}
/// 从指针转换为虚拟地址
impl<T> From<*mut T> for VA {
    fn from(pointer: *mut T) -> Self {
        Self(pointer as usize)
    }
}

/// 虚实页号之间的线性映射
impl From<PPN> for VPN {
    fn from(ppn: PPN) -> Self {
        Self(ppn.0 + (KERNEL_MAP_OFFSET >> PAGE_SIZE_BITS))
    }
}
/// 虚实页号之间的线性映射
impl From<VPN> for PPN {
    fn from(vpn: VPN) -> Self {
        Self(vpn.0 - (KERNEL_MAP_OFFSET >> PAGE_SIZE_BITS))
    }
}
/// 虚实地址之间的线性映射
impl From<PA> for VA {
    fn from(pa: PA) -> Self {
        Self(pa.0 + KERNEL_MAP_OFFSET)
    }
}
/// 虚实地址之间的线性映射
impl From<VA> for PA {
    fn from(va: VA) -> Self {
        Self(va.0 - KERNEL_MAP_OFFSET)
    }
}

macro_rules! implement_address_to_page_number {
    // 这里面的类型转换实现 [`From`] trait，会自动实现相反的 [`Into`] trait
    ($address_type: tt, $page_number_type: tt) => {
        /// 实现页号转地址
        impl From<$page_number_type> for $address_type {
            /// 从页号转换为地址
            fn from(page_number: $page_number_type) -> Self {
                Self(page_number.0 << PAGE_SIZE_BITS)
            }
        }
        /// 实现地址转页号
        impl From<$address_type> for $page_number_type {
            /// 从地址转换为页号，直接进行移位操作。不允许转换没有对齐的地址
            fn from(address: $address_type) -> Self {
                assert!(address.0 & (PAGE_SIZE - 1) == 0);
                Self(address.0 >> PAGE_SIZE_BITS)
            }
        }
        impl $address_type {
            /// 将地址转换为页号，向下取整
            pub const fn floor(self) -> $page_number_type {
                $page_number_type(self.0 >> PAGE_SIZE_BITS)
            }
            /// 将地址转换为页号，向上取整
            pub const fn ceil(self) -> $page_number_type {
                $page_number_type((self.0 - 1 + PAGE_SIZE) >> PAGE_SIZE_BITS)
            }
            /// 低 12 位的 offset
            pub const fn page_offset(&self) -> usize {
                self.0 & (PAGE_SIZE - 1)
            }
        }
    };
}
implement_address_to_page_number! {PA, PPN}
implement_address_to_page_number! {VA, VPN}

impl VPN {
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        for i in (0..3).rev() {
            idx[i] = vpn & 511;
            vpn >>= 9;
        }
        idx
    }
    pub fn get_array<T>(&self) -> &'static mut [T] {
        assert!(PAGE_SIZE % size_of::<T>() == 0);
        unsafe {
            core::slice::from_raw_parts_mut(
                (self.0 << PAGE_SIZE_BITS) as *mut T,
                PAGE_SIZE / size_of::<T>(),
            )
        }
    }
}

impl VA {
    pub fn get_ref<T>(&self) -> &'static T {
        unsafe { &*(self.0 as *const T) }
    }
    pub fn get_mut<T>(&self) -> &'static mut T {
        unsafe { &mut *(self.0 as *mut T) }
    }
}

/// 为各种仅包含一个 usize 的类型实现运算操作
/// TODO 把用不到的删掉
macro_rules! implement_usize_operations {
    ($type_name: ty) => {
        /// `+`
        #[allow(unused_unsafe)]
        impl core::ops::Add<usize> for $type_name {
            type Output = Self;
            fn add(self, other: usize) -> Self::Output {
                Self(self.0 + other)
            }
        }
        /// `+=`
        #[allow(unused_unsafe)]
        impl core::ops::AddAssign<usize> for $type_name {
            fn add_assign(&mut self, rhs: usize) {
                unsafe {
                    self.0 += rhs;
                }
            }
        }
        /// `-`
        #[allow(unused_unsafe)]
        impl core::ops::Sub<usize> for $type_name {
            type Output = Self;
            fn sub(self, other: usize) -> Self::Output {
                Self(self.0 - other)
            }
        }
        /// `-`
        impl core::ops::Sub<$type_name> for $type_name {
            type Output = usize;
            fn sub(self, other: $type_name) -> Self::Output {
                self.0 - other.0
            }
        }
        /// `-=`
        #[allow(unused_unsafe)]
        impl core::ops::SubAssign<usize> for $type_name {
            fn sub_assign(&mut self, rhs: usize) {
                self.0 -= rhs;
            }
        }
        /// 和 usize 相互转换
        #[allow(unused_unsafe)]
        impl From<usize> for $type_name {
            fn from(value: usize) -> Self {
                Self(value)
            }
        }
        /// 和 usize 相互转换
        impl From<$type_name> for usize {
            fn from(value: $type_name) -> Self {
                value.0
            }
        }
        /// 是否有效（0 为无效）
        impl $type_name {
            pub fn valid(&self) -> bool {
                self.0 != 0
            }
        }
        /// {} 输出
        impl core::fmt::Display for $type_name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}(0x{:x})", stringify!($type_name), self.0)
            }
        }
    };
}
implement_usize_operations! {PA}
implement_usize_operations! {VA}
implement_usize_operations! {PPN}
implement_usize_operations! {VPN}

// TODO 测试一下有无问题
unsafe impl Step for VPN {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        if *start <= *end {
            Some((*end - *start) as usize)
        } else {
            None
        }
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        Some(start + count)
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        Some(start - count)
    }
}

unsafe impl Step for PPN {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        if *start <= *end {
            Some((*end - *start) as usize)
        } else {
            None
        }
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        Some(start + count)
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        Some(start - count)
    }
}

pub type VPNRange = core::ops::Range<VPN>;
pub type VARange = core::ops::Range<VA>;
