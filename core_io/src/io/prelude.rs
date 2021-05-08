//! The I/O Prelude.
//!
//! The purpose of this module is to alleviate imports of many common I/O traits
//! by adding a glob import to the top of I/O heavy modules:
//!
//! ```
//! # #![allow(unused_imports)]
//! use std::io::prelude::*;
//! ```

#[cfg(feature = "collections")]
pub use super::BufRead;
pub use super::{Read, Seek, Write};

#[cfg(feature = "collections")]
pub use alloc::boxed::Box;
#[cfg(feature = "collections")]
pub use alloc::vec::Vec;
