use crate::error::create_resource;
use crate::pd::{self, ProtectionDomain};
use crate::resource::Resource;
use crate::utils::c_uint_to_u32;

use rdma_sys::{ibv_alloc_mw, ibv_dealloc_mw, ibv_mw};
use rdma_sys::{IBV_MW_TYPE_1, IBV_MW_TYPE_2};

use std::io;
use std::os::raw::c_uint;
use std::ptr::NonNull;
use std::sync::Arc;

pub struct MemoryWindow(Arc<Owner>);

/// SAFETY: resource type
unsafe impl Resource for MemoryWindow {
    type Owner = Owner;

    fn as_owner(&self) -> &Arc<Self::Owner> {
        &self.0
    }
}

impl MemoryWindow {
    #[inline]
    pub fn alloc(pd: &ProtectionDomain, mw_type: MemoryWindowType) -> io::Result<Self> {
        // SAFETY: ffi
        let owner = unsafe {
            let mw_type = mw_type.to_c_uint();
            let mw = create_resource(
                || ibv_alloc_mw(pd.ffi_ptr(), mw_type),
                || "failed to allocate memory window",
            )?;
            Arc::new(Owner {
                mw,
                _pd: pd.strong_ref(),
            })
        };
        Ok(Self(owner))
    }
}

pub(crate) struct Owner {
    mw: NonNull<ibv_mw>,
    _pd: Arc<pd::Owner>,
}

/// SAFETY: owned type
unsafe impl Send for Owner {}
/// SAFETY: owned type
unsafe impl Sync for Owner {}

impl Owner {
    fn ffi_ptr(&self) -> *mut ibv_mw {
        self.mw.as_ptr()
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        // SAFETY: ffi
        unsafe {
            let mw = self.ffi_ptr();
            let ret = ibv_dealloc_mw(mw);
            assert_eq!(ret, 0);
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum MemoryWindowType {
    Type1 = c_uint_to_u32(IBV_MW_TYPE_1),
    Type2 = c_uint_to_u32(IBV_MW_TYPE_2),
}

impl MemoryWindowType {
    #[allow(clippy::as_conversions)]
    fn to_c_uint(self) -> c_uint {
        self as u32 as c_uint
    }
}
