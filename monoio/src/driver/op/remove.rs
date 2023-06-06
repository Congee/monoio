#[cfg(unix)]
use std::io;
use std::{ffi::CString, path::Path};

#[cfg(all(target_os = "linux", feature = "iouring"))]
use io_uring::{opcode, types};

use super::{Op, OpAble};
use crate::driver::util::cstr;
#[cfg(all(unix, feature = "legacy"))]
use crate::{driver::legacy::ready::Direction, syscall_u32};

pub(crate) struct UnlinkAt {
    #[cfg(unix)]
    path: CString,
    flags: libc::c_int,
}

impl Op<UnlinkAt> {
    #[allow(unused)]
    #[cfg(unix)]
    pub(crate) fn unlink_at<P: AsRef<Path>>(
        path: P,
        flags: libc::c_int,
    ) -> io::Result<Op<UnlinkAt>> {
        let path = cstr(path.as_ref())?;
        Op::try_submit_with(UnlinkAt { path, flags })
    }
}

impl OpAble for UnlinkAt {
    #[cfg(all(target_os = "linux", feature = "iouring"))]
    fn uring_op(&mut self) -> io_uring::squeue::Entry {
        let ptr = self.path.as_c_str().as_ptr();
        opcode::UnlinkAt::new(types::Fd(libc::AT_FDCWD), ptr)
            .flags(self.flags)
            .build()
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_interest(&self) -> Option<(Direction, usize)> {
        None
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_call(&mut self) -> io::Result<u32> {
        let ptr = self.path.as_c_str().as_ptr();
        syscall_u32!(unlinkat(libc::AT_FDCWD, ptr, self.flags))
    }
}
