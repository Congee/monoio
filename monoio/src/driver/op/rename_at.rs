#[cfg(unix)]
use std::io;
use std::{ffi::CString, path::Path};

#[cfg(all(target_os = "linux", feature = "iouring"))]
use io_uring::{opcode, types};

use super::{Op, OpAble};
use crate::driver::util::cstr;
#[cfg(all(unix, feature = "legacy"))]
use crate::{driver::legacy::ready::Direction, syscall_u32};

pub(crate) struct RenameAt {
    from: CString,
    to: CString,
    flags: libc::c_uint,
}

impl Op<RenameAt> {
    #[allow(unused)]
    #[cfg(unix)]
    pub(crate) fn rename_at<P: AsRef<Path>>(
        from: P,
        to: P,
        flags: libc::c_uint,
    ) -> io::Result<Op<RenameAt>> {
        let from = cstr(from.as_ref())?;
        let to = cstr(to.as_ref())?;
        Op::try_submit_with(RenameAt { from, to, flags })
    }
}

impl OpAble for RenameAt {
    #[cfg(all(target_os = "linux", feature = "iouring"))]
    fn uring_op(&mut self) -> io_uring::squeue::Entry {
        let from = self.from.as_c_str().as_ptr();
        let to = self.to.as_c_str().as_ptr();
        opcode::RenameAt::new(
            types::Fd(libc::AT_FDCWD),
            from,
            types::Fd(libc::AT_FDCWD),
            to,
        )
        .flags(self.flags as _)
        .build()
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_interest(&self) -> Option<(Direction, usize)> {
        None
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_call(&mut self) -> io::Result<u32> {
        let from = self.from.as_c_str().as_ptr();
        let to = self.to.as_c_str().as_ptr();
        syscall_u32!(renameat2(
            libc::AT_FDCWD,
            from,
            libc::AT_FDCWD,
            to,
            self.flags
        ))
    }
}
