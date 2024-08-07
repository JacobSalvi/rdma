use crate::ah::AddressHandleOptions;
use crate::bindings::{self as C, ibv_qp_create_send_ops_flags, ibv_qp_init_attr_mask};
use crate::cq::CompletionQueue;
use crate::ctx::Context;
use crate::device::Mtu;
use crate::error::{create_resource, from_errno, get_errno, set_errno};
use crate::mr::AccessFlags;
use crate::pd::ProtectionDomain;
use crate::qp_ex::QueuePairEx;
use crate::qp_ex;
use crate::srq::SharedReceiveQueue;
use crate::utils::{bool_to_c_int, c_uint_to_u32, ptr_as_mut, u32_as_c_uint};
use crate::utils::{usize_to_void_ptr, void_ptr_to_usize};
use crate::wr::{RecvRequest, SendRequest};

use std::mem::MaybeUninit;
use std::os::raw::{c_int, c_uint};
use std::ptr::{self, NonNull};
use std::sync::Arc;
use std::{io, mem};

#[derive(Clone)]
pub struct QueuePair(Arc<Owner>);

impl QueuePair {
    pub(crate) fn ffi_ptr(&self) -> *mut C::ibv_qp {
        self.0.ffi_ptr()
    }

    #[inline]
    #[must_use]
    pub fn options() -> QueuePairOptions {
        QueuePairOptions::default()
    }

    #[inline]
    pub fn create(ctx: &Context, mut options: QueuePairOptions) -> io::Result<Self> {
        // SAFETY: ffi
        let owner = unsafe {
            let context = ctx.ffi_ptr();
            let qp_attr = &mut options.attr;

            let qp = create_resource(
                || C::ibv_create_qp_ex(context, qp_attr),
                || "failed to create queue pair",
            )?;

            Arc::new(Owner {
                qp,
                _pd: options.pd,
                send_cq: options.send_cq,
                recv_cq: options.recv_cq,
                _srq: options.srq,
            })
        };
        Ok(Self(owner))
    }

    #[inline]
    #[must_use]
    pub fn qp_num(&self) -> u32 {
        let qp = self.ffi_ptr();
        // SAFETY: reading a immutable field of a concurrent ffi type
        unsafe { (*qp).qp_num }
    }

    #[inline]
    #[must_use]
    pub fn user_data(&self) -> usize {
        let qp = self.ffi_ptr();
        // SAFETY: reading a immutable field of a concurrent ffi type
        unsafe { void_ptr_to_usize((*qp).qp_context) }
    }

    /// # Safety
    /// TODO
    #[inline]
    pub unsafe fn post_send(&self, send_wr: &SendRequest) -> io::Result<()> {
        let qp = self.ffi_ptr();
        let wr: *mut C::ibv_send_wr = ptr_as_mut(send_wr).cast();
        let mut bad_wr: *mut C::ibv_send_wr = ptr::null_mut();
        set_errno(0);
        let ret = C::ibv_post_send(qp, wr, &mut bad_wr);
        if ret != 0 {
            let errno = get_errno();
            if errno != 0 {
                return Err(from_errno(errno));
            }
            return Err(from_errno(ret.abs()));
        }
        Ok(())
    }

    /// # Safety
    /// TODO
    #[inline]
    pub unsafe fn post_recv(&self, recv_wr: &RecvRequest) -> io::Result<()> {
        let qp = self.ffi_ptr();
        let wr: *mut C::ibv_recv_wr = ptr_as_mut(recv_wr).cast();
        let mut bad_wr: *mut C::ibv_recv_wr = ptr::null_mut();
        set_errno(0);
        let ret = C::ibv_post_recv(qp, wr, &mut bad_wr);
        if ret != 0 {
            let errno = get_errno();
            if errno != 0 {
                return Err(from_errno(errno));
            }
            return Err(from_errno(ret.abs()));
        }
        Ok(())
    }

    #[inline]
    pub fn modify(&self, mut options: ModifyOptions) -> io::Result<()> {
        let qp = self.ffi_ptr();
        // SAFETY: ffi
        unsafe {
            let attr_mask: c_int = mem::transmute(options.mask);
            let attr = options.attr.as_mut_ptr();
            let ret = C::ibv_modify_qp(qp, attr, attr_mask);
            if ret != 0 {
                return Err(from_errno(ret));
            }
            Ok(())
        }
    }

    #[inline]
    pub fn query(&self, options: QueryOptions) -> io::Result<QueuePairAttr> {
        let qp = self.ffi_ptr();
        // SAFETY: ffi
        unsafe {
            let attr_mask: c_int = mem::transmute(options.mask);
            let mut attr: QueuePairAttr = mem::zeroed();
            let mut init_attr: C::ibv_qp_init_attr = mem::zeroed();
            let ret = C::ibv_query_qp(qp, &mut attr.attr, attr_mask, &mut init_attr);
            if ret != 0 {
                return Err(from_errno(ret));
            }
            attr.mask = options.mask;
            Ok(attr)
        }
    }

    #[inline]
    #[must_use]
    pub fn send_cq(&self) -> Option<&CompletionQueue> {
        self.0.send_cq.as_ref()
    }

    #[inline]
    #[must_use]
    pub fn recv_cq(&self) -> Option<&CompletionQueue> {
        self.0.recv_cq.as_ref()
    }



    pub fn to_qp_ex(&self) -> io::Result<QueuePairEx> {
        let owner = unsafe {
            let qp_ex = create_resource(|| C::ibv_qp_to_qp_ex(self.0.qp.as_ptr()), 
                || "Failed to create qp_ex")?;
            
            Arc::new(qp_ex::Owner::new(qp_ex))
        };
       Ok(QueuePairEx::new(owner)) 
    }
}

struct Owner {
    qp: NonNull<C::ibv_qp>,

    _pd: Option<ProtectionDomain>,
    send_cq: Option<CompletionQueue>,
    recv_cq: Option<CompletionQueue>,
    _srq: Option<SharedReceiveQueue>,
}

/// SAFETY: owned type
unsafe impl Send for Owner {}
/// SAFETY: owned type
unsafe impl Sync for Owner {}

impl Owner {
    fn ffi_ptr(&self) -> *mut C::ibv_qp {
        self.qp.as_ptr()
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        // SAFETY: ffi
        unsafe {
            let qp: *mut C::ibv_qp = self.ffi_ptr();
            let ret = C::ibv_destroy_qp(qp);
            assert_eq!(ret, 0);
        }
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct QueuePairCapacity {
    pub max_send_wr: u32,
    pub max_recv_wr: u32,
    pub max_send_sge: u32,
    pub max_recv_sge: u32,
    pub max_inline_data: u32,
}

impl Default for QueuePairCapacity {
    #[inline]
    fn default() -> Self {
        // SAFETY: POD ffi type
        unsafe { mem::zeroed() }
    }
}

impl QueuePairCapacity {
    fn into_ctype(self) -> C::ibv_qp_cap {
        // SAFETY: same repr
        unsafe { mem::transmute(self) }
    }
    fn from_ctype_ref(cap: &C::ibv_qp_cap) -> &Self {
        // SAFETY: same repr
        unsafe { mem::transmute(cap) }
    }
}

pub struct QueuePairOptions {
    attr: C::ibv_qp_init_attr_ex,

    send_cq: Option<CompletionQueue>,
    recv_cq: Option<CompletionQueue>,
    pd: Option<ProtectionDomain>,
    srq: Option<SharedReceiveQueue>,
}

// SAFETY: owned type
unsafe impl Send for QueuePairOptions {}
// SAFETY: owned type
unsafe impl Sync for QueuePairOptions {}

impl Default for QueuePairOptions {
    #[inline]
    fn default() -> Self {
        Self {
            // SAFETY: POD ffi type
            attr: unsafe { mem::zeroed() },
            send_cq: None,
            recv_cq: None,
            pd: None,
            srq: None,
        }
    }
}

impl QueuePairOptions {
    #[inline]
    pub fn user_data(&mut self, user_data: usize) -> &mut Self {
        self.attr.qp_context = usize_to_void_ptr(user_data);
        self
    }

    #[inline]
    pub fn send_cq(&mut self, send_cq: &CompletionQueue) -> &mut Self {
        self.attr.send_cq = C::ibv_cq_ex_to_cq(send_cq.ffi_ptr());
        self.send_cq = Some(send_cq.clone());
        self
    }

    #[inline]
    pub fn recv_cq(&mut self, recv_cq: &CompletionQueue) -> &mut Self {
        if self.srq.take().is_some() {
            self.attr.srq = ptr::null_mut();
        }
        self.attr.recv_cq = C::ibv_cq_ex_to_cq(recv_cq.ffi_ptr());
        self.recv_cq = Some(recv_cq.clone());
        self
    }

    #[inline]
    pub fn qp_type(&mut self, qp_type: QueuePairType) -> &mut Self {
        self.attr.qp_type = qp_type.to_c_uint();
        self
    }

    #[inline]
    pub fn sq_sig_all(&mut self, sq_sig_all: bool) -> &mut Self {
        self.attr.sq_sig_all = bool_to_c_int(sq_sig_all);
        self
    }

    #[inline]
    pub fn cap(&mut self, cap: QueuePairCapacity) -> &mut Self {
        self.attr.cap = cap.into_ctype();
        self
    }

    #[inline]
    pub fn pd(&mut self, pd: &ProtectionDomain) -> &mut Self {
        self.attr.pd = pd.ffi_ptr();
        self.attr.comp_mask |= C::IBV_QP_INIT_ATTR_PD;
        self.pd = Some(pd.clone());
        self
    }

    #[inline]
    pub fn srq(&mut self, srq: &SharedReceiveQueue) -> &mut Self {
        if self.recv_cq.take().is_some() {
            self.attr.recv_cq = ptr::null_mut();
        }
        self.attr.srq = srq.ffi_ptr();
        self.srq = Some(srq.clone());
        self
    }

    #[inline]
    pub fn comp_mask(&mut self, mask: ibv_qp_init_attr_mask) -> &mut Self{
        self.attr.comp_mask = mask;
        self
    }

    #[inline]
    pub fn send_ops_flags(&mut self, flags: ibv_qp_create_send_ops_flags) -> &mut Self{
        self.attr.send_ops_flags = u64::from(flags);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QueuePairType {
    RC = c_uint_to_u32(C::IBV_QPT_RC),
    UC = c_uint_to_u32(C::IBV_QPT_UC),
    UD = c_uint_to_u32(C::IBV_QPT_UD),
    Driver = c_uint_to_u32(C::IBV_QPT_DRIVER),
    XrcRecv = c_uint_to_u32(C::IBV_QPT_XRC_RECV),
    XrcSend = c_uint_to_u32(C::IBV_QPT_XRC_SEND),
}

impl QueuePairType {
    fn to_c_uint(self) -> c_uint {
        #[allow(clippy::as_conversions)]
        u32_as_c_uint(self as u32)
    }
}

#[repr(C)]
pub struct ModifyOptions {
    mask: C::ibv_qp_attr_mask,
    attr: MaybeUninit<C::ibv_qp_attr>,
}

// SAFETY: owned type
unsafe impl Send for ModifyOptions {}
// SAFETY: owned type
unsafe impl Sync for ModifyOptions {}

impl Default for ModifyOptions {
    #[inline]
    fn default() -> Self {
        Self {
            mask: 0,
            attr: MaybeUninit::uninit(),
        }
    }
}

macro_rules! modify_option {
    ($mask: ident, $field: ident, $ty: ty, $($cvt:tt)+) => {
        #[inline]
        pub fn $field(&mut self, $field: $ty) -> &mut Self {
            // SAFETY: write uninit field
            unsafe {
                let attr = self.attr.as_mut_ptr();
                let p = ptr::addr_of_mut!((*attr).$field);
                p.write($($cvt)+);
            }
            self.mask |= C::$mask;
            self
        }
    };
}

impl ModifyOptions {
    modify_option!(IBV_QP_STATE, qp_state, QueuePairState, qp_state.to_c_uint());
    modify_option!(IBV_QP_PKEY_INDEX, pkey_index, u16, pkey_index);
    modify_option!(IBV_QP_PORT, port_num, u8, port_num);
    modify_option!(IBV_QP_QKEY, qkey, u32, qkey);
    modify_option!(
        IBV_QP_ACCESS_FLAGS,
        qp_access_flags,
        AccessFlags,
        qp_access_flags.to_c_uint()
    );
    modify_option!(IBV_QP_PATH_MTU, path_mtu, Mtu, path_mtu.to_c_uint());
    modify_option!(IBV_QP_DEST_QPN, dest_qp_num, u32, dest_qp_num);
    modify_option!(IBV_QP_RQ_PSN, rq_psn, u32, rq_psn);
    modify_option!(
        IBV_QP_MAX_DEST_RD_ATOMIC,
        max_dest_rd_atomic,
        u8,
        max_dest_rd_atomic
    );
    modify_option!(IBV_QP_MIN_RNR_TIMER, min_rnr_timer, u8, min_rnr_timer);
    modify_option!(
        IBV_QP_AV,
        ah_attr,
        AddressHandleOptions,
        ah_attr.into_ctype()
    );
    modify_option!(IBV_QP_TIMEOUT, timeout, u8, timeout);
    modify_option!(IBV_QP_RETRY_CNT, retry_cnt, u8, retry_cnt);
    modify_option!(IBV_QP_RNR_RETRY, rnr_retry, u8, rnr_retry);
    modify_option!(IBV_QP_SQ_PSN, sq_psn, u32, sq_psn);
    modify_option!(IBV_QP_MAX_QP_RD_ATOMIC, max_rd_atomic, u8, max_rd_atomic);
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct QueryOptions {
    mask: C::ibv_qp_attr_mask,
}

impl Default for QueryOptions {
    #[inline]
    fn default() -> Self {
        // SAFETY: POD ffi type
        unsafe { mem::zeroed() }
    }
}

impl QueryOptions {
    #[inline]
    pub fn cap(&mut self) -> &mut Self {
        self.mask |= C::IBV_QP_CAP;
        self
    }

    #[inline]
    pub fn qp_state(&mut self) -> &mut Self {
        self.mask |= C::IBV_QP_STATE;
        self
    }
}

#[repr(C)]
pub struct QueuePairAttr {
    mask: C::ibv_qp_attr_mask,
    attr: C::ibv_qp_attr,
}

// SAFETY: owned type
unsafe impl Send for QueuePairAttr {}
// SAFETY: owned type
unsafe impl Sync for QueuePairAttr {}

impl QueuePairAttr {
    #[inline]
    #[must_use]
    pub fn cap(&self) -> Option<&QueuePairCapacity> {
        (self.mask & C::IBV_QP_CAP != 0).then(|| QueuePairCapacity::from_ctype_ref(&self.attr.cap))
    }

    #[inline]
    #[must_use]
    pub fn qp_state(&self) -> Option<QueuePairState> {
        (self.mask & C::IBV_QP_STATE != 0).then(|| QueuePairState::from_c_uint(self.attr.qp_state))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QueuePairState {
    Reset = c_uint_to_u32(C::IBV_QPS_RESET),
    Initialize = c_uint_to_u32(C::IBV_QPS_INIT),
    ReadyToReceive = c_uint_to_u32(C::IBV_QPS_RTR),
    ReadyToSend = c_uint_to_u32(C::IBV_QPS_RTS),
    SendQueueDrained = c_uint_to_u32(C::IBV_QPS_SQD),
    SendQueueError = c_uint_to_u32(C::IBV_QPS_SQE),
    Error = c_uint_to_u32(C::IBV_QPS_ERR),
    Unknown = c_uint_to_u32(C::IBV_QPS_UNKNOWN), // ASK: what is this
}

impl QueuePairState {
    fn from_c_uint(val: c_uint) -> Self {
        match val {
            C::IBV_QPS_RESET => Self::Reset,
            C::IBV_QPS_INIT => Self::Initialize,
            C::IBV_QPS_RTR => Self::ReadyToReceive,
            C::IBV_QPS_RTS => Self::ReadyToSend,
            C::IBV_QPS_SQD => Self::SendQueueDrained,
            C::IBV_QPS_SQE => Self::SendQueueError,
            C::IBV_QPS_ERR => Self::Error,
            _ => panic!("unexpected queue pair state"),
        }
    }

    fn to_c_uint(self) -> c_uint {
        #[allow(clippy::as_conversions)]
        u32_as_c_uint(self as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rust_utils::offset_of;

    #[test]
    fn qp_cap_layout() {
        assert_eq!(
            mem::size_of::<QueuePairCapacity>(),
            mem::size_of::<C::ibv_qp_cap>()
        );
        assert_eq!(
            mem::align_of::<QueuePairCapacity>(),
            mem::align_of::<C::ibv_qp_cap>()
        );

        assert_eq!(
            offset_of!(QueuePairCapacity, max_send_wr),
            offset_of!(C::ibv_qp_cap, max_send_wr)
        );
        assert_eq!(
            offset_of!(QueuePairCapacity, max_recv_wr),
            offset_of!(C::ibv_qp_cap, max_recv_wr)
        );
        assert_eq!(
            offset_of!(QueuePairCapacity, max_send_sge),
            offset_of!(C::ibv_qp_cap, max_send_sge)
        );
        assert_eq!(
            offset_of!(QueuePairCapacity, max_recv_sge),
            offset_of!(C::ibv_qp_cap, max_recv_sge)
        );
        assert_eq!(
            offset_of!(QueuePairCapacity, max_inline_data),
            offset_of!(C::ibv_qp_cap, max_inline_data)
        );
    }
}
