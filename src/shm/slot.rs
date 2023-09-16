//! A pool implementation based on buffer slots

use std::io;
use std::{
    os::unix::io::{AsRawFd, OwnedFd},
    sync::{
        atomic::{AtomicU8, AtomicUsize, Ordering},
        Arc, Mutex, Weak,
    },
};

use wayland_client::{
    protocol::{wl_buffer, wl_shm, wl_surface},
    Proxy,
};

use crate::{globals::ProvidesBoundGlobal, shm::raw::RawPool, shm::CreatePoolError};

#[derive(Debug, thiserror::Error)]
pub enum CreateBufferError {
    /// Slot creation error.
    #[error(transparent)]
    Io(#[from] io::Error),

    /// Pool mismatch.
    #[error("Incorrect pool for slot")]
    PoolMismatch,

    /// Slot size mismatch
    #[error("Requested buffer size is too large for slot")]
    SlotTooSmall,
}

#[derive(Debug, thiserror::Error)]
pub enum ActivateSlotError {
    /// Buffer was already active
    #[error("Buffer was already active")]
    AlreadyActive,
}

#[derive(Debug)]
pub struct SlotPool {
    pub(crate) inner: RawPool,
    free_list: Arc<Mutex<Vec<FreelistEntry>>>,
}

#[derive(Debug)]
struct FreelistEntry {
    offset: usize,
    len: usize,
}

/// A chunk of memory allocated from a [SlotPool]
///
/// Retaining this object is only required if you wish to resize or change the buffer's format
/// without changing the contents of the backing memory.
#[derive(Debug)]
pub struct Slot {
    inner: Arc<SlotInner>,
}

#[derive(Debug)]
struct SlotInner {
    free_list: Weak<Mutex<Vec<FreelistEntry>>>,
    offset: usize,
    len: usize,
    active_buffers: AtomicUsize,
    /// Count of all "real" references to this slot.  This includes all Slot objects and any
    /// BufferData object that is not in the DEAD state.  When this reaches zero, the memory for
    /// this slot will return to the free_list.  It is not possible for it to reach zero and have a
    /// Slot or Buffer referring to it.
    all_refs: AtomicUsize,
}

/// A wrapper around a [`wl_buffer::WlBuffer`] which has been allocated via a [SlotPool].
///
/// When this object is dropped, the buffer will be destroyed immediately if it is not active, or
/// upon the server's release if it is.
#[derive(Debug)]
pub struct Buffer {
    buffer: wl_buffer::WlBuffer,
    height: i32,
    stride: i32,
    slot: Slot,
}

/// ObjectData for the WlBuffer
#[derive(Debug)]
struct BufferData {
    inner: Arc<SlotInner>,
    state: AtomicU8,
}

// These constants define the value of BufferData::state, since AtomicEnum does not exist.
impl BufferData {
    /// Buffer is counted in active_buffers list; will return to INACTIVE on Release.
    const ACTIVE: u8 = 0;

    /// Buffer is not counted in active_buffers list, but also has not been destroyed.
    const INACTIVE: u8 = 1;

    /// Buffer is counted in active_buffers list; will move to DEAD on Release
    const DESTROY_ON_RELEASE: u8 = 2;

    /// Buffer has been destroyed
    const DEAD: u8 = 3;

    /// Value that is ORed on buffer release to transition to the next state
    const RELEASE_SET: u8 = 1;

    /// Value that is ORed on buffer destroy to transition to the next state
    const DESTROY_SET: u8 = 2;

    /// Call after successfully transitioning the state to DEAD
    fn record_death(&self) {
        drop(Slot { inner: self.inner.clone() })
    }
}

impl SlotPool {
    pub fn new(
        len: usize,
        shm: &impl ProvidesBoundGlobal<wl_shm::WlShm, 1>,
    ) -> Result<Self, CreatePoolError> {
        let inner = RawPool::new(len, shm)?;
        let free_list = Arc::new(Mutex::new(vec![FreelistEntry { offset: 0, len: inner.len() }]));
        Ok(SlotPool { inner, free_list })
    }

    /// Create a new buffer in a new slot.
    ///
    /// This returns the buffer and the canvas.  The parameters are:
    ///
    /// - `width`: the width of this buffer (in pixels)
    /// - `height`: the height of this buffer (in pixels)
    /// - `stride`: distance (in bytes) between the beginning of a row and the next one
    /// - `format`: the encoding format of the pixels. Using a format that was not
    ///   advertised to the `wl_shm` global by the server is a protocol error and will
    ///   terminate your connection.
    ///
    /// The [Slot] for this buffer will have exactly the size required for the data.  It can be
    /// accessed via [Buffer::slot] to create additional buffers that point to the same data.  This
    /// is required if you wish to change formats, buffer dimensions, or attach a canvas to
    /// multiple surfaces.
    ///
    /// For more control over sizing, use [Self::new_slot] and [Self::create_buffer_in].
    pub fn create_buffer(
        &mut self,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
    ) -> Result<(Buffer, &mut [u8]), CreateBufferError> {
        let len = (height as usize) * (stride as usize);
        let slot = self.new_slot(len)?;
        let buffer = self.create_buffer_in(&slot, width, height, stride, format)?;
        let canvas = self.raw_data_mut(&slot);
        Ok((buffer, canvas))
    }

    /// Get the bytes corresponding to a given slot or buffer if drawing to the slot is permitted.
    ///
    /// Returns `None` if there are active buffers in the slot or if the slot does not correspond
    /// to this pool.
    pub fn canvas(&mut self, key: &impl CanvasKey) -> Option<&mut [u8]> {
        key.canvas(self)
    }

    /// Returns the size, in bytes, of this pool.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Resizes the memory pool, notifying the server the pool has changed in size.
    ///
    /// This is an optimization; the pool automatically resizes when you allocate new slots.
    pub fn resize(&mut self, size: usize) -> io::Result<()> {
        let old_len = self.inner.len();
        self.inner.resize(size)?;
        let new_len = self.inner.len();
        if old_len == new_len {
            return Ok(());
        }
        // add the new memory to the freelist
        let mut free = self.free_list.lock().unwrap();
        if let Some(FreelistEntry { offset, len }) = free.last_mut() {
            if *offset + *len == old_len {
                *len += new_len - old_len;
                return Ok(());
            }
        }
        free.push(FreelistEntry { offset: old_len, len: new_len - old_len });
        Ok(())
    }

    fn alloc(&mut self, size: usize) -> io::Result<usize> {
        let mut free = self.free_list.lock().unwrap();
        for FreelistEntry { offset, len } in free.iter_mut() {
            if *len >= size {
                let rv = *offset;
                *len -= size;
                *offset += size;
                return Ok(rv);
            }
        }
        let mut rv = self.inner.len();
        let mut pop_tail = false;
        if let Some(FreelistEntry { offset, len }) = free.last() {
            if offset + len == self.inner.len() {
                rv -= len;
                pop_tail = true;
            }
        }
        // resize like Vec::reserve, always at least doubling
        let target = std::cmp::max(rv + size, self.inner.len() * 2);
        self.inner.resize(target)?;
        // adjust the end of the freelist here
        if pop_tail {
            free.pop();
        }
        if target > rv + size {
            free.push(FreelistEntry { offset: rv + size, len: target - rv - size });
        }
        Ok(rv)
    }

    fn free(free_list: &Mutex<Vec<FreelistEntry>>, mut offset: usize, mut len: usize) {
        let mut free = free_list.lock().unwrap();
        let mut nf = Vec::with_capacity(free.len() + 1);
        for &FreelistEntry { offset: ioff, len: ilen } in free.iter() {
            if ioff + ilen == offset {
                offset = ioff;
                len += ilen;
                continue;
            }
            if ioff == offset + len {
                len += ilen;
                continue;
            }
            if ioff > offset + len && len != 0 {
                nf.push(FreelistEntry { offset, len });
                len = 0;
            }
            if ilen != 0 {
                nf.push(FreelistEntry { offset: ioff, len: ilen });
            }
        }
        if len != 0 {
            nf.push(FreelistEntry { offset, len });
        }
        *free = nf;
    }

    /// Create a new slot with the given size in bytes.
    pub fn new_slot(&mut self, mut len: usize) -> io::Result<Slot> {
        len = (len + 63) & !63;
        let offset = self.alloc(len)?;

        Ok(Slot {
            inner: Arc::new(SlotInner {
                free_list: Arc::downgrade(&self.free_list),
                offset,
                len,
                active_buffers: AtomicUsize::new(0),
                all_refs: AtomicUsize::new(1),
            }),
        })
    }

    /// Get the bytes corresponding to a given slot.
    ///
    /// Note: prefer using [Self::canvas], which will prevent drawing to a buffer that has not been
    /// released by the server.
    ///
    /// Returns an empty buffer if the slot does not belong to this pool.
    pub fn raw_data_mut(&mut self, slot: &Slot) -> &mut [u8] {
        if slot.inner.free_list.as_ptr() == Arc::as_ptr(&self.free_list) {
            &mut self.inner.mmap()[slot.inner.offset..][..slot.inner.len]
        } else {
            &mut []
        }
    }

    /// Create a new buffer corresponding to a slot.
    ///
    /// The parameters are:
    ///
    /// - `width`: the width of this buffer (in pixels)
    /// - `height`: the height of this buffer (in pixels)
    /// - `stride`: distance (in bytes) between the beginning of a row and the next one
    /// - `format`: the encoding format of the pixels. Using a format that was not
    ///   advertised to the `wl_shm` global by the server is a protocol error and will
    ///   terminate your connection
    pub fn create_buffer_in(
        &mut self,
        slot: &Slot,
        width: i32,
        height: i32,
        stride: i32,
        format: wl_shm::Format,
    ) -> Result<Buffer, CreateBufferError> {
        let offset = slot.inner.offset as i32;
        let len = (height as usize) * (stride as usize);
        if len > slot.inner.len {
            return Err(CreateBufferError::SlotTooSmall);
        }

        if slot.inner.free_list.as_ptr() != Arc::as_ptr(&self.free_list) {
            return Err(CreateBufferError::PoolMismatch);
        }

        let slot = slot.clone();
        // take a ref for the BufferData, which will be destroyed by BufferData::record_death
        slot.inner.all_refs.fetch_add(1, Ordering::Relaxed);
        let data = Arc::new(BufferData {
            inner: slot.inner.clone(),
            state: AtomicU8::new(BufferData::INACTIVE),
        });
        let buffer = self.inner.create_buffer_raw(offset, width, height, stride, format, data);
        Ok(Buffer { buffer, height, stride, slot })
    }
}

impl Clone for Slot {
    fn clone(&self) -> Self {
        let inner = self.inner.clone();
        inner.all_refs.fetch_add(1, Ordering::Relaxed);
        Slot { inner }
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        if self.inner.all_refs.fetch_sub(1, Ordering::Relaxed) == 1 {
            if let Some(free_list) = self.inner.free_list.upgrade() {
                SlotPool::free(&free_list, self.inner.offset, self.inner.len);
            }
        }
    }
}

impl Drop for SlotInner {
    fn drop(&mut self) {
        debug_assert_eq!(*self.all_refs.get_mut(), 0);
    }
}

/// A helper trait for [SlotPool::canvas].
pub trait CanvasKey {
    fn canvas<'pool>(&self, pool: &'pool mut SlotPool) -> Option<&'pool mut [u8]>;
}

impl Slot {
    /// Return true if there are buffers referencing this slot whose contents are being accessed
    /// by the server.
    pub fn has_active_buffers(&self) -> bool {
        self.inner.active_buffers.load(Ordering::Relaxed) != 0
    }

    /// Returns the size, in bytes, of this slot.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.inner.len
    }

    /// Get the bytes corresponding to a given slot if drawing to the slot is permitted.
    ///
    /// Returns `None` if there are active buffers in the slot or if the slot does not correspond
    /// to this pool.
    pub fn canvas<'pool>(&self, pool: &'pool mut SlotPool) -> Option<&'pool mut [u8]> {
        if self.has_active_buffers() {
            return None;
        }
        if self.inner.free_list.as_ptr() == Arc::as_ptr(&pool.free_list) {
            Some(&mut pool.inner.mmap()[self.inner.offset..][..self.inner.len])
        } else {
            None
        }
    }
}

impl CanvasKey for Slot {
    fn canvas<'pool>(&self, pool: &'pool mut SlotPool) -> Option<&'pool mut [u8]> {
        self.canvas(pool)
    }
}

impl Buffer {
    /// Attach a buffer to a surface.
    ///
    /// This marks the slot as active until the server releases the buffer, which will happen
    /// automatically assuming the surface is committed without attaching a different buffer.
    ///
    /// Note: if you need to ensure that [`canvas()`](Buffer::canvas) calls never return data that
    /// could be attached to a surface in a multi-threaded client, make this call while you have
    /// exclusive access to the corresponding [`SlotPool`].
    pub fn attach_to(&self, surface: &wl_surface::WlSurface) -> Result<(), ActivateSlotError> {
        self.activate()?;
        surface.attach(Some(&self.buffer), 0, 0);
        Ok(())
    }

    /// Get the inner buffer.
    pub fn wl_buffer(&self) -> &wl_buffer::WlBuffer {
        &self.buffer
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn stride(&self) -> i32 {
        self.stride
    }

    fn data(&self) -> Option<&BufferData> {
        self.buffer.object_data()?.downcast_ref()
    }

    /// Get the bytes corresponding to this buffer if drawing is permitted.
    ///
    /// This may be smaller than the canvas associated with the slot.
    pub fn canvas<'pool>(&self, pool: &'pool mut SlotPool) -> Option<&'pool mut [u8]> {
        let len = (self.height as usize) * (self.stride as usize);
        if self.slot.inner.active_buffers.load(Ordering::Relaxed) != 0 {
            return None;
        }
        if self.slot.inner.free_list.as_ptr() == Arc::as_ptr(&pool.free_list) {
            Some(&mut pool.inner.mmap()[self.slot.inner.offset..][..len])
        } else {
            None
        }
    }

    /// Get the slot corresponding to this buffer.
    pub fn slot(&self) -> Slot {
        self.slot.clone()
    }

    /// Manually mark a buffer as active.
    ///
    /// An active buffer prevents drawing on its slot until a Release event is received or until
    /// manually deactivated.
    pub fn activate(&self) -> Result<(), ActivateSlotError> {
        let data = self.data().expect("UserData type mismatch");

        // This bitwise AND will transition INACTIVE -> ACTIVE, or do nothing if the buffer was
        // already ACTIVE.  No other ordering is required, as the server will not send a Release
        // until we send our attach after returning Ok.
        match data.state.fetch_and(!BufferData::RELEASE_SET, Ordering::Relaxed) {
            BufferData::INACTIVE => {
                data.inner.active_buffers.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            BufferData::ACTIVE => Err(ActivateSlotError::AlreadyActive),
            _ => unreachable!("Invalid state in BufferData"),
        }
    }

    /// Manually mark a buffer as inactive.
    ///
    /// This should be used when the buffer was manually marked as active or when a buffer was
    /// attached to a surface but not committed.  Calling this function on a buffer that was
    /// committed to a surface risks making the surface contents undefined.
    pub fn deactivate(&self) -> Result<(), ActivateSlotError> {
        let data = self.data().expect("UserData type mismatch");

        // Same operation as the Release event, but we know the Buffer was not dropped.
        match data.state.fetch_or(BufferData::RELEASE_SET, Ordering::Relaxed) {
            BufferData::ACTIVE => {
                data.inner.active_buffers.fetch_sub(1, Ordering::Relaxed);
                Ok(())
            }
            BufferData::INACTIVE => Err(ActivateSlotError::AlreadyActive),
            _ => unreachable!("Invalid state in BufferData"),
        }
    }
}

impl CanvasKey for Buffer {
    fn canvas<'pool>(&self, pool: &'pool mut SlotPool) -> Option<&'pool mut [u8]> {
        self.canvas(pool)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if let Some(data) = self.data() {
            match data.state.fetch_or(BufferData::DESTROY_SET, Ordering::Relaxed) {
                BufferData::ACTIVE => {
                    // server is using the buffer, let ObjectData handle the destroy
                }
                BufferData::INACTIVE => {
                    data.record_death();
                    self.buffer.destroy();
                }
                _ => unreachable!("Invalid state in BufferData"),
            }
        }
    }
}

impl wayland_client::backend::ObjectData for BufferData {
    fn event(
        self: Arc<Self>,
        handle: &wayland_client::backend::Backend,
        msg: wayland_backend::protocol::Message<wayland_backend::client::ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn wayland_backend::client::ObjectData>> {
        debug_assert!(wayland_client::backend::protocol::same_interface(
            msg.sender_id.interface(),
            wl_buffer::WlBuffer::interface()
        ));
        debug_assert!(msg.opcode == 0);

        match self.state.fetch_or(BufferData::RELEASE_SET, Ordering::Relaxed) {
            BufferData::ACTIVE => {
                self.inner.active_buffers.fetch_sub(1, Ordering::Relaxed);
            }
            BufferData::INACTIVE => {
                // possible spurious release, or someone called deactivate incorrectly
                log::debug!("Unexpected WlBuffer::Release on an inactive buffer");
            }
            BufferData::DESTROY_ON_RELEASE => {
                self.record_death();
                self.inner.active_buffers.fetch_sub(1, Ordering::Relaxed);

                // The Destroy message is identical to Release message (no args, same ID), so just reply
                handle
                    .send_request(msg.map_fd(|x| x.as_raw_fd()), None, None)
                    .expect("Unexpected invalid ID");
            }
            BufferData::DEAD => {
                // no-op, this object is already unusable
            }
            _ => unreachable!("Invalid state in BufferData"),
        }

        None
    }

    fn destroyed(&self, _: wayland_backend::client::ObjectId) {}
}

impl Drop for BufferData {
    fn drop(&mut self) {
        let state = *self.state.get_mut();
        if state == BufferData::ACTIVE || state == BufferData::DESTROY_ON_RELEASE {
            // Release the active-buffer count
            self.inner.active_buffers.fetch_sub(1, Ordering::Relaxed);
        }

        if state != BufferData::DEAD {
            // nobody has ever transitioned state to DEAD, so we are responsible for freeing the
            // extra reference
            self.record_death();
        }
    }
}
