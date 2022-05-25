use crate::error::create_resource;
use crate::query::DeviceAttr;
use crate::query::PortAttr;
use crate::resource::Resource;
use crate::resource::ResourceOwner;
use crate::CompChannel;
use crate::CompletionQueue;
use crate::CompletionQueueOptions;
use crate::Device;
use crate::GidEntry;
use crate::ProtectionDomain;
use crate::QueuePair;
use crate::QueuePairOptions;

use rdma_sys::ibv_context;
use rdma_sys::{ibv_close_device, ibv_open_device};

use std::cell::UnsafeCell;
use std::io;
use std::ptr::NonNull;

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
    pub fn create_cq(&self, options: CompletionQueueOptions) -> io::Result<CompletionQueue> {
        CompletionQueue::create(self, options)
    }

    #[inline]
    pub fn query_device(&self) -> io::Result<DeviceAttr> {
        DeviceAttr::query(self)
    }

    #[inline]
    pub fn query_port(&self, port_num: u32) -> io::Result<PortAttr> {
        PortAttr::query(self, port_num)
    }

    #[inline]
    pub fn query_gid_entry(&self, port_num: u32, gid_index: u32) -> io::Result<GidEntry> {
        GidEntry::query(self, port_num, gid_index)
    }

    #[inline]
    pub fn create_qp(&self, options: QueuePairOptions) -> io::Result<QueuePair> {
        QueuePair::create(self, options)
    }
}

pub(crate) struct ContextOwner {
    ctx: NonNull<UnsafeCell<ibv_context>>,
}

/// SAFETY: owned type
unsafe impl Send for ContextOwner {}
/// SAFETY: owned type
unsafe impl Sync for ContextOwner {}

/// SAFETY: resource owner
unsafe impl ResourceOwner for ContextOwner {
    type Ctype = ibv_context;

    fn ctype(&self) -> *mut Self::Ctype {
        self.ctx.as_ptr().cast()
    }
}

impl ContextOwner {
    fn open(device: &Device) -> io::Result<Self> {
        // SAFETY: ffi
        unsafe {
            let ctx = create_resource(
                || ibv_open_device(device.ffi_ptr()),
                || "failed to open device",
            )?;
            Ok(Self { ctx: ctx.cast() })
        }
    }
}

impl Drop for ContextOwner {
    fn drop(&mut self) {
        // SAFETY: ffi
        let ret = unsafe { ibv_close_device(self.ctype()) };
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
