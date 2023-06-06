use std::ffi::CString;
#[cfg(unix)]
use std::io;
use std::path::Path;

#[cfg(all(target_os = "linux", feature = "iouring"))]
use io_uring::{opcode, types};

use super::{driver::shared_fd::SharedFd, Op, OpAble};
use crate::driver::util::cstr;
#[cfg(all(unix, feature = "legacy"))]
use crate::{driver::legacy::ready::Direction, syscall_u32};

pub(crate) struct Statx {
    fd: Option<SharedFd>,
    path: Option<CString>,
    flags: libc::c_int,
    mask: libc::c_uint,
    statxbuf: *mut libc::statx,
}

impl Op<Statx> {
    #[allow(unused)]
    #[cfg(unix)]
    pub(crate) fn statx<P: AsRef<Path>>(
        fd: Option<SharedFd>,
        path: Option<P>,
        flags: libc::c_int,
        mask: libc::c_uint,
        buf: *mut libc::statx,
    ) -> io::Result<Op<Statx>> {
        let path = path.map(|p| cstr(p.as_ref())).transpose()?;
        Op::try_submit_with(Statx {
            fd,
            path,
            flags,
            mask,
            statxbuf: buf,
        })
    }
}

impl OpAble for Statx {
    #[cfg(all(target_os = "linux", feature = "iouring"))]
    fn uring_op(&mut self) -> io_uring::squeue::Entry {
        let raw_fd = self.fd.as_ref().map_or(libc::AT_FDCWD, |fd| fd.raw_fd());
        let path = self
            .path
            .clone()
            .map_or(std::ptr::null(), |p| p.as_c_str().as_ptr());

        opcode::Statx::new(types::Fd(raw_fd), path, self.statxbuf as *mut _)
            .flags(self.flags)
            .mask(self.mask)
            .build()
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_interest(&self) -> Option<(Direction, usize)> {
        None
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_call(&mut self) -> io::Result<u32> {
        let raw_fd = self.fd.as_ref().map_or(libc::AT_FDCWD, |fd| fd.raw_fd());
        let path = self
            .path
            .clone()
            .map_or(std::ptr::null(), |p| p.as_c_str().as_ptr());

        syscall_u32!(statx(raw_fd, path, self.flags, self.mask, self.statxbuf))
    }
}
