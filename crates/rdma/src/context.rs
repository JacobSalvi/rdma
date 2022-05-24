use crate::error::custom_error;
use crate::resource::Resource;
use crate::resource::ResourceOwner;
use crate::CompChannel;
use crate::CompletionQueue;
use crate::Device;
use crate::ProtectionDomain;

use std::io;
use std::ptr::NonNull;

use rdma_sys::*;

#[derive(Clone)]
pub struct Context(pub(crate) Resource<ContextOwner>);

impl Context {
    #[inline]
    pub fn open(device: &Device) -> io::Result<Self> {
        let owner = ContextOwner::open(device)?;
        Ok(Self(Resource::new(owner)))
    }

    #[inline]
    pub fn alloc_pd(&self) -> io::Result<ProtectionDomain> {
        ProtectionDomain::alloc(self)
    }

    #[inline]
    pub fn create_cc(&self) -> io::Result<CompChannel> {
        CompChannel::create(self)
    }

    #[inline]
    pub fn create_cq(&self, cqe: usize, user_data: usize) -> io::Result<CompletionQueue> {
        CompletionQueue::create(self, cqe, user_data)
    }

    #[inline]
    pub fn create_cq_with_cc(
        &self,
        cqe: usize,
        user_data: usize,
        cc: &CompChannel,
    ) -> io::Result<CompletionQueue> {
        CompletionQueue::create_with_cc(self, cqe, user_data, cc)
    }
}

pub(crate) struct ContextOwner {
    ctx: NonNull<ibv_context>,
}

/// SAFETY: owned type
unsafe impl Send for ContextOwner {}
/// SAFETY: owned type
unsafe impl Sync for ContextOwner {}

/// SAFETY: resource owner
unsafe impl ResourceOwner for ContextOwner {
    type Ctype = ibv_context;

    fn ctype(&self) -> *mut Self::Ctype {
        self.ctx.as_ptr()
    }
}

impl ContextOwner {
    fn open(device: &Device) -> io::Result<Self> {
        // SAFETY: ffi
        unsafe {
            let ctx = ibv_open_device(device.ffi_ptr());
            if ctx.is_null() {
                return Err(custom_error("failed to open device"));
            }
            let ctx = NonNull::new_unchecked(ctx);
            Ok(Self { ctx })
        }
    }
}

impl Drop for ContextOwner {
    fn drop(&mut self) {
        // SAFETY: ffi
        let ret = unsafe { ibv_close_device(self.ctx.as_ptr()) };
        assert_eq!(ret, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::utils::require_send_sync;

    #[test]
    fn marker() {
        require_send_sync::<Context>();
        require_send_sync::<ContextOwner>();
    }
}
