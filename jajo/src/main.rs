use std::{
    io::{self, Read},
    mem::{self, MaybeUninit}, os::fd::AsRawFd
};

use cqueue::{CompletionQueue, CQE};
use squeue::{SubmissionQueue, SQE};

pub mod cqueue;
pub mod squeue;
fn main() {
    let buf = [69u8; 128];
    let mut read_buf = [0u8; 128];
    let mut ring = IoUring::new_with_flags(1024, Default::default(), Default::default()).unwrap();
    let file = std::fs::OpenOptions::new().read(true).write(true).create(true).open("test").unwrap();
    let fd = file.as_raw_fd();
    if let Some(mut sqe) =  ring.prepare_sqe() {
        sqe.prepare_write(&buf, fd, 0);
    }
    let _ = ring.submit_and_wait(1).expect("Failed to submit op to kernel");
    if let Some(mut sqe) = ring.prepare_sqe() {
        sqe.prepare_read(&mut read_buf, fd, 0);
    }
    let res = ring.submit_and_wait(1).expect("Failed to submit op to kernel");
    println!("read op response: {}, read values: {:?}", res, read_buf);
    // file.read_exact(&mut read_buf).unwrap();
}

bitflags::bitflags! {
    pub struct SetupFeatures: u32 {
        const IORING_FEAT_SINGLE_MMAP       = 1 << 0;
        const IORING_FEAT_NODROP            = 1 << 1;
        const IORING_FEAT_SUBMIT_STABLE     = 1 << 2;
        const IORING_FEAT_RW_CUR_POS        = 1 << 3;
        const IORING_FEAT_CUR_PERSONALITY   = 1 << 4;
        const IORING_FEAT_FAST_POLL         = 1 << 5;
        const IORING_FEAT_POLL_32BITS       = 1 << 6;
        const IORING_FEAT_SQPOLL_NONFIXED   = 1 << 7;
        const IORING_FEAT_EXT_ARG           = 1 << 8;
        const IORING_FEAT_NATIVE_WORKERS    = 1 << 9;
        const IORING_FEAT_RSRC_TAGS         = 1 << 10;
        const IORING_FEAT_CQE_SKIP          = 1 << 11;
        const IORING_FEAT_LINKED_FILE       = 1 << 12;
        const IORING_FEAT_REG_REG_RING      = 1 << 13;
        const IORING_FEAT_RECVSEND_BUNDLE   = 1 << 14;
        const IORING_FEAT_MIN_TIMEOUT       = 1 << 15;
    }
}

impl Default for SetupFeatures {
    fn default() -> Self {
        Self::empty()
    }
}

bitflags::bitflags! {
    pub struct SetupFlags: u32 {
        const IORING_SETUP_IOPOLL	         = 1 << 0;	/* io_context is polled */
        const IORING_SETUP_SQPOLL            = 1 << 1;  /* SQ poll thread */
        const IORING_SETUP_SQ_AFF	         = 1 << 2;	/* sq_thread_cpu is valid */
        const IORING_SETUP_CQSIZE	         = 1 << 3;	/* app defines CQ size */
        const IORING_SETUP_CLAMP	         = 1 << 4;  /* clamp SQ/CQ ring sizes */
        const IORING_SETUP_ATTACH_WQ	     = 1 << 5;	/* attach to existing wq */
        const IORING_SETUP_R_DISABLED	     = 1 << 6;	/* start with ring disabled */
        const IORING_SETUP_SUBMIT_ALL	     = 1 << 7;	/* continue submit on error */
        /*
         * Cooperative task running. When requests complete, they often require
         * forcing the submitter to transition to the kernel to complete. If this
         * flag is set, work will be done when the task transitions anyway, rather
         * than force an inter-processor interrupt reschedule. This avoids interrupting
         * a task running in userspace, and saves an IPI.
         */
        const IORING_SETUP_COOP_TASKRUN	      = 1 << 8;
        /*
         * If COOP_TASKRUN is set, get notified if task work is available for
         * running and a kernel transition would be needed to run it. This sets
         * IORING_SQ_TASKRUN in the sq ring flags. Not valid with COOP_TASKRUN.
         */
        const IORING_SETUP_TASKRUN_FLAG	      = 1 << 9;
        const IORING_SETUP_SQE128		      = 1 << 10; /* SQEs are 128 byte */
        const IORING_SETUP_CQE32		      = 1 << 11; /* CQEs are 32 byte */
        /*
         * Only one task is allowed to submit requests
         */
        const IORING_SETUP_SINGLE_ISSUER	  = 1 << 12;

        /*
         * Defer running task work to get events.
         * Rather than running bits of task work whenever the task transitions
         * try to do it just before it is needed.
         */
        const IORING_SETUP_DEFER_TASKRUN	  = 1 << 13;

        /*
         * Application provides the memory for the rings
         */
        const IORING_SETUP_NO_MMAP	          = 1 << 14;

        /*
         * Register the ring fd in itself for use with
         * IORING_REGISTER_USE_REGISTERED_RING; return a registered fd index rather
         * than an fd.
         */
        const IORING_SETUP_REGISTERED_FD_ONLY = 1 << 15;

        /*
         * Removes indirection through the SQ index array.
         */
        const IORING_SETUP_NO_SQARRAY		  = 1 << 16;
    }

}

impl Default for SetupFlags {
    fn default() -> Self {
        Self::empty()
    }
}

pub struct IoUring {
    ring: uring_sys2::io_uring,
}

impl IoUring {
    pub fn new_with_flags(
        entries: u32,
        flags: SetupFlags,
        features: SetupFeatures,
    ) -> io::Result<Self> {
        unsafe {
            let mut p: uring_sys2::io_uring_params = mem::zeroed();
            p.flags = flags.bits();
            p.features = features.bits();
            let mut ring = MaybeUninit::uninit();
            resultify(uring_sys2::io_uring_queue_init_params(
                entries,
                ring.as_mut_ptr(),
                &mut p,
            ))?;
            // TODO assert the size of ring.
            Ok(Self {
                ring: ring.assume_init(),
            })
        }
    }

    fn sq(&mut self) -> SubmissionQueue {
        SubmissionQueue::new(self)
    }

    fn cq(&mut self) -> CompletionQueue {
        CompletionQueue::new(self)
    }

    pub fn prepare_sqe<'a>(&mut self) -> Option<SQE<'a>> {
        unsafe { self.sq().prepare_sqe() }
    }

    pub fn submit_and_wait(&mut self, count: u32) -> std::io::Result<u32> {
        self.sq().submit_and_wait(count)
    }

    pub fn peek_cqe(&mut self) -> Option<CQE> {
        self.cq().peek_for_cqe()
    }
}

impl Drop for IoUring {
    fn drop(&mut self) {
        unsafe { uring_sys2::io_uring_queue_exit(&mut self.ring) }
    }
}

pub(crate) fn resultify(x: i32) -> io::Result<u32> {
    match x >= 0 {
        true => Ok(x as u32),
        false => Err(io::Error::from_raw_os_error(-x)),
    }
}
