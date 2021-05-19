use crate::arch::interface::TaskContext;

#[repr(C)]
#[derive(Debug, Default)]
pub struct TaskContextImpl {
    ra: usize,
    /// Saved Register，被调用者需要保存的寄存器。
    s: [usize; 12],
}

impl TaskContext for TaskContextImpl {
    fn set_ra(&mut self, value: usize) -> &mut Self {
        self.ra = value;
        self
    }
}
