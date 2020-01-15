//! Various small utilities helping you to write clients

use std::{cell::RefCell, rc::Rc};

use wayland_client::{
    protocol::{wl_registry, wl_shm},
    Attached, DispatchData,
};

mod mempool;

pub use self::mempool::{DoubleMemPool, MemPool};
pub use wl_shm::Format;

pub struct ShmHandler {
    shm: Option<Attached<wl_shm::WlShm>>,
    formats: Rc<RefCell<Vec<wl_shm::Format>>>,
}

impl ShmHandler {
    pub fn new() -> ShmHandler {
        ShmHandler {
            shm: None,
            formats: Rc::new(RefCell::new(vec![])),
        }
    }
}

impl crate::environment::GlobalHandler<wl_shm::WlShm> for ShmHandler {
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        _version: u32,
        _: DispatchData,
    ) {
        // only shm verison 1 is supported
        let shm = registry.bind::<wl_shm::WlShm>(1, id);
        let my_formats = self.formats.clone();
        shm.quick_assign(move |_, event, _| match event {
            wl_shm::Event::Format { format } => {
                my_formats.borrow_mut().push(format);
            }
            _ => unreachable!(),
        });
        self.shm = Some((*shm).clone());
    }
    fn get(&self) -> Option<Attached<wl_shm::WlShm>> {
        self.shm.clone()
    }
}

pub trait ShmHandling {
    fn shm_formats(&self) -> Vec<wl_shm::Format>;
}

impl ShmHandling for ShmHandler {
    fn shm_formats(&self) -> Vec<wl_shm::Format> {
        self.formats.borrow().clone()
    }
}
