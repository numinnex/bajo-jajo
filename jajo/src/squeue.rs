use std::fmt::Display;

use crate::{resultify, IoUring};
use bitflags::bitflags;

pub struct SubmissionQueue<'ring> {
    ring: &'ring mut uring_sys2::io_uring,
}

impl<'ring> SubmissionQueue<'ring> {
    pub fn new(ring: &'ring mut IoUring) -> Self {
        Self {
            ring: &mut ring.ring,
        }
    }

    pub unsafe fn prepare_sqe<'a>(&mut self) -> Option<SQE<'a>> {
        let sqe = uring_sys2::io_uring_get_sqe(self.ring);
        if !sqe.is_null() {
            let mut sqe = SQE::new(&mut *sqe);
            sqe.clear();
            Some(sqe)
        } else {
            None
        }
    }

    pub fn submit_and_wait(&mut self, count: u32) -> std::io::Result<u32> {
        resultify(unsafe { uring_sys2::io_uring_submit_and_wait(self.ring, count) })
    }

    pub fn submit(&mut self) -> std::io::Result<u32> {
        resultify(unsafe { uring_sys2::io_uring_submit(self.ring) })
    }
}

pub struct SQE<'a> {
    sqe: &'a mut uring_sys2::io_uring_sqe,
}

impl Display for SQE<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SQE")
    }
}

impl<'a> SQE<'a> {
    pub fn new(sqe: &'a mut uring_sys2::io_uring_sqe) -> Self {
        Self { sqe }
    }

    pub fn clear(&mut self) {
        *self.sqe = unsafe { std::mem::zeroed() };
    }

    pub fn prepare_read(&mut self, buf: &mut [u8], fd: i32, offset: u64) {
        unsafe {
            uring_sys2::io_uring_prep_read(
                self.sqe,
                fd as _,
                buf.as_mut_ptr() as _,
                buf.len() as _,
                offset,
            )
        }
    }

    pub fn prepare_nop(&mut self) {
        unsafe {
            uring_sys2::io_uring_prep_nop(self.sqe);
        }
    }

    pub fn prepare_write(&mut self, buf: &[u8], fd: i32, offset: u64) {
        unsafe {
            uring_sys2::io_uring_prep_write(
                self.sqe,
                fd as _,
                buf.as_ptr() as _,
                buf.len() as _,
                offset,
            )
        }
    }
}

bitflags! {
    pub struct SubmissionFlags: u32 {
        const IOSQE_FIXED_FILE = 1 << 0;
        const IOSQE_IO_DRAIN = 1 << 1;
        const IOSQE_IO_LINK = 1 << 2;
        const IOSQE_IO_HARDLINK = 1 << 3;
        const IOSQE_ASYNC = 1 << 4;
        const IOSQE_BUFFER_SELECT = 1 << 5;
        const IOSQE_CQE_SKIP_SUCCESS = 1 << 6;
    }
}
