use std::mem::MaybeUninit;

use crate::IoUring;
use bitflags::bitflags;

pub struct CompletionQueue<'ring> {
    ring: &'ring mut uring_sys2::io_uring,
}

impl<'ring> CompletionQueue<'ring> {
    pub fn new(ring: &'ring mut IoUring) -> Self {
        Self {
            ring: &mut ring.ring,
        }
    }

    pub fn peek_for_cqe(&mut self) -> Option<CQE> {
        unsafe {
            let mut cqe = MaybeUninit::uninit();
            uring_sys2::io_uring_peek_cqe(self.ring, cqe.as_mut_ptr());
            let cqe = cqe.assume_init();
            if !cqe.is_null() {
                Some(CQE::new(self.ring, &mut *cqe))
            } else {
                None
            }
        }
    }
}

pub struct CQE {
    user_data: u64,
    res: i32,
    flags: CompletionFlags,
}

impl CQE {
    pub fn new(ring: &mut uring_sys2::io_uring, cqe: &mut uring_sys2::io_uring_cqe) -> CQE {
        let user_data = cqe.user_data;
        let res = cqe.res;
        let flags = CompletionFlags::from_bits_truncate(cqe.flags);
        unsafe {
            uring_sys2::io_uring_cqe_seen(ring, cqe);
        }
        CQE {
            user_data,
            res,
            flags,
        }
    }
}

bitflags! {
    pub struct CompletionFlags: u32 {
        const IORING_CQE_F_BUFFER         = 1 << 0;
        const IORING_CQE_F_MORE           = 1 << 1;
        const IORING_CQE_F_SOCK_NONEMPTY  = 1 << 2;
        const IORING_CQE_F_NOTIF          = 1 << 3;
        const IORING_CQE_F_BUF_MORE       = 1 << 4;
        const IORING_CQE_BUFFER_SHIFT     = 16;
    }
}
