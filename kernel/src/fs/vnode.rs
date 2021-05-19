use alloc::{boxed::Box, string::String, sync::Arc};
use core::hash::Hash;

use fatfs::ReadWriteSeek;
use hashbrown::HashSet;
use lazy_static::lazy_static;

use crate::{
    arch::{interface::Console, ConsoleImpl},
    spinlock::SpinLock,
};

lazy_static! {
    pub static ref VNODE_HASHSET: SpinLock<HashSet<Arc<Vnode>>> = Default::default();
    pub static ref CONSOLE_VNODE: Arc<Vnode> = Arc::new(Vnode {
        full_path: String::new(),
        inode: Box::new(ConsoleImpl::CONSOLE_INSTANCE),
    });
}

pub struct Vnode {
    // TODO std::path::PathBuf
    pub(super) full_path: String,
    // inode，对应一个文件
    pub(super) inode: Box<dyn ReadWriteSeek + Send + Sync>,
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
