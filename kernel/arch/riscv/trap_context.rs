use core::mem::zeroed;

use riscv::register::sstatus::{self, Sstatus, SPP::*};

use crate::arch::interface::TrapFrame;

/// 发生中断时，保存的寄存器
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TrapFrameImpl {
    /// 通用寄存器 x0~x31
    /// 事实上 x4/tp 并没有被保存，而是用来表示当前cpuid
    pub x: [usize; 32],
    /// 保存诸多状态位的特权态寄存器
    pub sstatus: Sstatus,
    /// 保存中断地址的特权态寄存器
    pub sepc: usize,
}

/// 创建一个用 0 初始化的 TrapFrameImpl
impl Default for TrapFrameImpl {
    fn default() -> Self {
        unsafe { zeroed() }
    }
}

impl TrapFrame for TrapFrameImpl {
    /// 获取栈指针
    fn sp(&self) -> usize {
        self.x[2]
    }

    /// 设置栈指针
    fn set_sp(&mut self, value: usize) -> &mut Self {
        self.x[2] = value;
        self
    }

    /// 获取返回地址
    fn ra(&self) -> usize {
        self.x[1]
    }

    /// 设置返回地址
    fn set_ra(&mut self, value: usize) -> &mut Self {
        self.x[1] = value;
        self
    }

    /// 设置返回值
    fn set_return_value(&mut self, value: usize) -> &mut Self {
        self.x[10] = value;
        self
    }

    /// 设置入口
    fn set_entry_point(&mut self, value: usize) -> &mut Self {
        self.sepc = value;
        self
    }

    /// 按照函数调用规则写入参数
    fn set_arguments(&mut self, arguments: &[usize]) -> &mut Self {
        assert!(arguments.len() <= 8);
        self.x[10..(10 + arguments.len())].copy_from_slice(arguments);
        self
    }

    /// 为线程构建初始 `TrapFrameImpl`
    fn init(
        &mut self,
        stack_top: usize,
        entry_point: usize,
        arguments: Option<&[usize]>,
        is_user: bool,
    ) {
        // 设置栈顶指针
        self.set_sp(stack_top);
        // 设置初始参数
        if let Some(args) = arguments {
            self.set_arguments(args);
        } else {
            self.set_arguments(&[0; 4]);
        }
        // 设置入口地址
        self.sepc = entry_point;
        // println!("entry_point: {:x}", entry_point);
        // 设置 sstatus
        self.sstatus = sstatus::read();
        // 中断前处于内核态还是用户态
        if is_user {
            self.sstatus.set_spp(User);
        } else {
            self.sstatus.set_spp(Supervisor);
        }
        // 这样设置 SPIE 位，使得替换 sstatus 后关闭中断，
        // 而在 sret 到用户线程时开启中断。
        self.sstatus.set_spie(true);
    }
}
