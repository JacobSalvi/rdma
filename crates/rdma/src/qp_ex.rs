use std::io;
use std::sync::Arc;
use crate::bindings::{self as C };
use crate::cq::CompletionQueue;
use crate::pd::ProtectionDomain;
use crate::srq::SharedReceiveQueue;

use std::ptr::NonNull;



pub struct Owner {
    qp_ex: NonNull<C::ibv_qp_ex>,

    pd: Option<ProtectionDomain>,
    send_cq: Option<CompletionQueue>,
    recv_cq: Option<CompletionQueue>,
    srq: Option<SharedReceiveQueue>,
}

impl Owner {

    #[must_use] 
    pub fn new(qp_ex: NonNull<C::ibv_qp_ex>, pd: Option<ProtectionDomain>,  send_cq: Option<CompletionQueue>,  
        recv_cq: Option<CompletionQueue>, srq: Option<SharedReceiveQueue>) -> Self{
        Owner {qp_ex, pd, send_cq, recv_cq, srq}
    }
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

    pub fn new(owner: Arc<Owner>) -> Self{
        QueuePairEx(owner)
    }

}
