use std::mem;

use crate::bindings::ibv_poll_cq_attr;

pub struct PollCQAttr{
    attr: ibv_poll_cq_attr
}

impl PollCQAttr{
    #[must_use]
    pub fn new_empty() -> Self{
        PollCQAttr::default()
    }
}


impl Default for PollCQAttr{
    fn default() -> Self{
        Self{
            attr: unsafe{mem::zeroed()}
        }
    }
}
