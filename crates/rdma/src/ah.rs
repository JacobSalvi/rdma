use crate::bindings as C;
use crate::error::create_resource;
use crate::pd::{self, ProtectionDomain};
use crate::resource::Resource;

use std::io;
use std::mem;
use std::ptr::NonNull;
use std::sync::Arc;

pub struct AddressHandle(Arc<Owner>);

impl AddressHandle {
    #[inline]
    pub fn create(pd: &ProtectionDomain, mut options: AddressHandleOptions) -> io::Result<Self> {
        // SAFETY: ffi
        let owner = unsafe {
            let attr = &mut options.attr;
            let ah = create_resource(
                || C::ibv_create_ah(pd.ffi_ptr(), attr),
                || "failed to create address handle",
            )?;
            Arc::new(Owner {
                ah,
                _pd: pd.strong_ref(),
            })
        };
        Ok(Self(owner))
    }
}

pub(crate) struct Owner {
    ah: NonNull<C::ibv_ah>,

    _pd: Arc<pd::Owner>,
}

// SAFETY: owned type
unsafe impl Send for Owner {}
// SAFETY: owned type
unsafe impl Sync for Owner {}

impl Owner {
    fn ffi_ptr(&self) -> *mut C::ibv_ah {
        self.ah.as_ptr()
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        // SAFETY: ffi
        unsafe {
            let ah = self.ffi_ptr();
            let ret = C::ibv_destroy_ah(ah);
            assert_eq!(ret, 0);
        }
    }
}

pub struct AddressHandleOptions {
    attr: C::ibv_ah_attr,
}

impl Default for AddressHandleOptions {
    #[inline]
    fn default() -> Self {
        Self {
            // SAFETY: POD ffi type
            attr: unsafe { mem::zeroed() },
        }
    }
}

impl AddressHandleOptions {
    #[inline]
    pub fn dest_lid(&mut self, dest_lid: u16) -> &mut Self {
        self.attr.dlid = dest_lid;
        self
    }

    #[inline]
    pub fn service_level(&mut self, service_level: u8) -> &mut Self {
        self.attr.sl = service_level;
        self
    }

    #[inline]
    pub fn port_num(&mut self, port_num: u8) -> &mut Self {
        self.attr.port_num = port_num;
        self
    }
}