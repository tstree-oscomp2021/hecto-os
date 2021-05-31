use alloc::{boxed::Box, string::String, sync::Arc};
use core::hash::Hash;

use fatfs::Inode;
use hashbrown::HashSet;
use lazy_static::lazy_static;

use super::*;
use crate::{
    arch::{interface::Console, ConsoleImpl},
    sync::SpinLock,
};

lazy_static! {
    pub static ref VNODE_HASHSET: SpinLock<HashSet<Arc<Vnode>>> = Default::default();
    pub static ref CONSOLE_VNODE: Arc<Vnode> = Arc::new(Vnode {
        fs: &(None, None),
        full_path: String::new(),
        inode: Box::new(ConsoleImpl::CONSOLE_INSTANCE),
    });
}

pub struct Vnode {
    /// XXX 暂时还没地方用到这个 field
    pub(super) fs: &'static (
        Option<FileSystem>,
        Option<Dir<'static, BufBlockDevice<BlockDeviceImpl>>>,
    ),
    // 完整路径
    pub full_path: String,
    //  inode，一个实现了 Inode Trait 的文件对象
    pub inode: Box<dyn Inode + Send + Sync>,
}

impl Eq for Vnode {}
impl PartialEq for Vnode {
    fn eq(&self, other: &Self) -> bool {
        self.full_path == other.full_path
    }
}

impl Hash for Vnode {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.full_path.hash(state);
    }
}
