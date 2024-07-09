use std::io;
use std::sync::Arc;
use crate::bindings::{self as C };
use crate::cq::CompletionQueue;
use crate::error::custom_error;
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

    #[must_use] 
    pub fn new(owner: Arc<Owner>) -> Self{
        QueuePairEx(owner)
    }
    
    pub fn start_wr(&mut self){
        unsafe {
            C::ibv_wr_start(self.ffi_ptr());
        }
    }
    
    pub fn post_send(&self) -> io::Result<()> {
        let qp = self.ffi_ptr();
        unsafe {
            C::ibv_wr_send(qp);
        }
        Ok(())
    }

    pub fn  set_sge(&mut self, lkey: u32, addr: u64, length: u32){
        let qpex = self.ffi_ptr();
        unsafe{
            C::ibv_wr_set_sge(qpex, lkey, addr, length);
        }
    }

    pub fn wr_complete(&mut self) -> Result<(), std::io::Error >{
        let qpx = self.ffi_ptr();
        unsafe {
            match C::ibv_wr_complete(qpx){
                0 => Ok(()),
                _ => Err(custom_error("Failed to post send"))
            }
        }
    }

    
    #[inline]
    pub fn wr_id(&mut self, wr_id: u64) -> &mut Self{
        unsafe{
            (*self.ffi_ptr()).wr_id = wr_id;
        }
        self
    }

    #[inline]
    pub fn wr_flags(&mut self, wr_flags: u32) -> &mut Self{
        unsafe{
            (*self.ffi_ptr()).wr_flags = wr_flags;
        }
        self
    }


}
