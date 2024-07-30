use std::mem;
use crate::bindings as C;



#[derive(Clone)]
pub struct PollCQAttr{
    attr: C::ibv_poll_cq_attr
}

impl PollCQAttr{
    #[must_use]
    pub fn new_empty() -> Self{
        PollCQAttr::default()
    }

    pub(crate) fn ffi_ptr(&mut self) -> *mut C::ibv_poll_cq_attr {
        &mut self.attr
    }    
}


impl Default for PollCQAttr{
    fn default() -> Self{
        Self{attr: unsafe{mem::zeroed()}}
    }
        
}
