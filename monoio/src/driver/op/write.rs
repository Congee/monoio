use std::io;

#[cfg(all(target_os = "linux", feature = "iouring"))]
use io_uring::{opcode, types};
#[cfg(all(unix, feature = "legacy"))]
use {
    crate::{driver::legacy::ready::Direction, syscall_u32},
    std::os::unix::prelude::AsRawFd,
};

use super::{super::shared_fd::SharedFd, Op, OpAble};
use crate::{
    buf::{IoBuf, IoVecBuf},
    BufResult,
};

pub(crate) struct Write<T> {
    /// Holds a strong ref to the FD, preventing the file from being closed
    /// while the operation is in-flight.
    #[allow(unused)]
    fd: SharedFd,
    offset: u64,

    pub(crate) buf: T,
}

impl<T: IoBuf> Op<Write<T>> {
    pub fn write_at(fd: &SharedFd, buf: T, offset: u64) -> io::Result<Op<Write<T>>> {
        Op::submit_with(Write {
            fd: fd.clone(),
            offset,
            buf,
        })
    }

    pub async fn write(self) -> BufResult<usize, T> {
        let complete = self.await;
        (complete.meta.result.map(|v| v as _), complete.data.buf)
    }
}

impl<T: IoBuf> OpAble for Write<T> {
    #[cfg(all(target_os = "linux", feature = "iouring"))]
    fn uring_op(&mut self) -> io_uring::squeue::Entry {
        opcode::Write::new(
            types::Fd(self.fd.raw_fd()),
            self.buf.read_ptr(),
            self.buf.bytes_init() as _,
        )
        .offset(self.offset)
        .build()
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_interest(&self) -> Option<(Direction, usize)> {
        self.fd
            .registered_index()
            .map(|idx| (Direction::Write, idx))
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_call(&mut self) -> io::Result<u32> {
        let fd = self.fd.as_raw_fd();
        let seek_offset = libc::off_t::try_from(self.offset)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "offset too big"))?;
        let neg_seek_offset = seek_offset
            .checked_neg()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "offset too big"))?;
        if self.offset != 0 {
            syscall_u32!(lseek(fd, seek_offset, libc::SEEK_CUR))?;
            syscall_u32!(write(
                fd,
                self.buf.read_ptr() as _,
                self.buf.bytes_init().min(u32::MAX as usize)
            ))
            .map_err(|e| {
                // seek back if read fail...
                let _ = syscall_u32!(lseek(fd, neg_seek_offset, libc::SEEK_CUR));
                e
            })
        } else {
            syscall_u32!(write(
                fd,
                self.buf.read_ptr() as _,
                self.buf.bytes_init().min(u32::MAX as usize)
            ))
        }
    }
}

pub struct WriteVec<T> {
    /// Holds a strong ref to the FD, preventing the file from being closed
    /// while the operation is in-flight.
    #[allow(unused)]
    fd: SharedFd,

    pub(crate) buf_vec: T,
}

impl<T: IoVecBuf> Op<WriteVec<T>> {
    pub fn writev(fd: &SharedFd, buf_vec: T) -> io::Result<Self> {
        Op::submit_with(WriteVec {
            fd: fd.clone(),
            buf_vec,
        })
    }

    #[allow(unused)]
    pub(crate) fn writev_raw(fd: &SharedFd, buf_vec: T) -> WriteVec<T> {
        WriteVec {
            fd: fd.clone(),
            buf_vec,
        }
    }

    pub async fn write(self) -> BufResult<usize, T> {
        let complete = self.await;
        (complete.meta.result.map(|v| v as _), complete.data.buf_vec)
    }
}

impl<T: IoVecBuf> OpAble for WriteVec<T> {
    #[cfg(all(target_os = "linux", feature = "iouring"))]
    fn uring_op(&mut self) -> io_uring::squeue::Entry {
        let ptr = self.buf_vec.read_iovec_ptr() as *const _;
        let len = self.buf_vec.read_iovec_len() as _;
        opcode::Writev::new(types::Fd(self.fd.raw_fd()), ptr, len).build()
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_interest(&self) -> Option<(Direction, usize)> {
        self.fd
            .registered_index()
            .map(|idx| (Direction::Write, idx))
    }

    #[cfg(all(unix, feature = "legacy"))]
    fn legacy_call(&mut self) -> io::Result<u32> {
        syscall_u32!(writev(
            self.fd.raw_fd(),
            self.buf_vec.read_iovec_ptr(),
            self.buf_vec.read_iovec_len().min(i32::MAX as usize) as _
        ))
    }
}
