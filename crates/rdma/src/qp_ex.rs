use std::io;
use std::sync::Arc;
use crate::bindings::{self as C };
use crate::cq::CompletionQueue;
use crate::pd::ProtectionDomain;
use crate::srq::SharedReceiveQueue;

use std::ptr::NonNull;



struct Owner {
    qp_ex: NonNull<C::ibv_qp_ex>,

    _pd: Option<ProtectionDomain>,
    send_cq: Option<CompletionQueue>,
    recv_cq: Option<CompletionQueue>,
    _srq: Option<SharedReceiveQueue>,
}

impl Owner {
    fn ffi_ptr(&self) -> *mut C::ibv_qp_ex {
        self.qp_ex.as_ptr()
    }
}





#[derive(Clone)]
pub struct QueuePairEx(Arc<Owner>);

impl QueuePairEx {
    pub(crate) fn ffi_ptr(&self) -> *mut C::ibv_qp_ex {
        self.0.ffi_ptr()
    }

    #[inline]
    pub fn create(ctx: &Context, mut options: QueuePairOptions) -> io::Result<Self> {
        // SAFETY: ffi
        let owner = unsafe {
            let context = ctx.ffi_ptr();
            let qp_attr = &mut options.attr;

            let qp_ex = create_resource(
                || C::ibv_create_qp_ex(context, qp_attr),
                || "failed to create queue pair",
            )?;

            Arc::new(Owner {
                qp_ex,
                _pd: options.pd,
                send_cq: options.send_cq,
                recv_cq: options.recv_cq,
                _srq: options.srq,
            })
        };
        Ok(Self(owner))
    }

}
