use std::mem;
use std::ptr::NonNull;
use std::sync::Arc;
use crate::bindings as C;



#[derive(Clone)]
pub struct PollCQAttr(Arc<Owner>);



impl PollCQAttr{
    #[must_use]
    pub fn new_empty() -> Self{
        PollCQAttr::default()
    }

    pub(crate) fn ffi_ptr(&self) -> *mut C::ibv_poll_cq_attr {
        self.0.ffi_ptr()
    }    
}


impl Default for PollCQAttr{
    fn default() -> Self{
        Self(unsafe{mem::zeroed()})
    }
        
}

pub(crate) struct Owner {
    cq: NonNull<C::ibv_poll_cq_attr>,
}

impl Owner {
    pub(crate) fn ffi_ptr(&self) -> *mut C::ibv_poll_cq_attr {
        self.cq.as_ptr()
    }
}


