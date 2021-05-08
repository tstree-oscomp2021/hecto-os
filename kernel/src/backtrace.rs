use crate::register::*;

/// 打印 backtrace 信息，注意需要给 rustc 加上 `-Cforce-frame-pointers=yes` 选项
pub fn backtrace() {
    use crate::ffi::*;

    println!("stack backtrace:");
    let mut ra = ra();
    let mut fp = fp();
    let mut stack_num = 0;

    while ra >= stext as usize && ra <= etext as usize {
        println!("{:>4}:   ra {:016x}    fp {:016x}", stack_num, ra, fp);

        stack_num = stack_num + 1;
        unsafe {
            fp = *(fp as *const usize).offset(-2);
            if fp == 0 {
                break;
            }
            ra = *(fp as *const usize).offset(-1);
        }
    }
}
