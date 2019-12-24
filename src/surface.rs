use crate::output::{add_output_listener, with_output_info, OutputListener};
use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc, Mutex,
};
use wayland_client::protocol::{wl_compositor, wl_output, wl_surface};
use wayland_client::Main;

pub(crate) struct SurfaceUserData {
    dpi_factor: Arc<Mutex<i32>>,
    outputs: Vec<(wl_output::WlOutput, Arc<AtomicI32>, OutputListener)>,
    dpi_change_cb: Option<Arc<Mutex<dyn FnMut(i32, wl_surface::WlSurface) + Send + 'static>>>,
}

impl SurfaceUserData {
    fn new(
        dpi_change_cb: Option<Arc<Mutex<dyn FnMut(i32, wl_surface::WlSurface) + Send + 'static>>>,
    ) -> Self {
        SurfaceUserData {
            dpi_factor: Arc::new(Mutex::new(1)),
            outputs: Vec::new(),
            dpi_change_cb,
        }
    }

    pub(crate) fn enter(&mut self, output: wl_output::WlOutput, surface: wl_surface::WlSurface) {
        let dpi = with_output_info(&output, |info| info.scale_factor).unwrap_or(1);
        let arc = Arc::new(AtomicI32::new(dpi));
        let my_arc = arc.clone();
        let listener = if let Some(ref change_cb) = self.dpi_change_cb {
            let my_cb = change_cb.clone();
            let my_surface = surface.clone();
            let my_dpi = self.dpi_factor.clone();
            add_output_listener(&output, move |info| {
                if info.obsolete {
                    // an output that no longer exists is marked by a dpi factor of -1
                    my_arc.store(-1, Ordering::Release);
                } else {
                    let mut dpi = my_dpi.lock().unwrap();
                    my_arc.store(info.scale_factor, Ordering::Release);
                    // If this dpi change cause the effective scale factor for this window
                    // to inscrease, notify it. We don't notify about DPI decrease, because
                    // they are much less obvious to spot, and less visible to the user.
                    if *dpi < info.scale_factor {
                        *dpi = info.scale_factor;
                        (&mut *my_cb.lock().unwrap())(info.scale_factor, my_surface.clone());
                    }
                }
            })
        } else {
            add_output_listener(&output, move |info| {
                if info.obsolete {
                    // an output that no longer exists is marked by a dpi factor of -1
                    my_arc.store(-1, Ordering::Release);
                } else {
                    my_arc.store(info.scale_factor, Ordering::Release);
                }
            })
        };
        self.outputs.push((output, arc, listener));
        self.compute_dpi_factor(&surface);
    }

    pub(crate) fn leave(&mut self, output: &wl_output::WlOutput, surface: wl_surface::WlSurface) {
        self.outputs
            .retain(|(ref output2, _, _)| !output.as_ref().equals(output2.as_ref()));
        self.compute_dpi_factor(&surface);
    }

    fn compute_dpi_factor(&mut self, surface: &wl_surface::WlSurface) -> i32 {
        let mut scale_factor = 1;
        self.outputs.retain(|(_, dpi, _)| {
            let v = dpi.load(Ordering::Acquire);
            if v > 0 {
                scale_factor = ::std::cmp::max(scale_factor, v);
                true
            } else {
                // cleanup obsolete output
                false
            }
        });
        let mut dpi = self.dpi_factor.lock().unwrap();
        if *dpi != scale_factor {
            *dpi = scale_factor;
            if let Some(ref mut cb) = self.dpi_change_cb {
                (&mut *cb.lock().unwrap())(scale_factor, surface.clone());
            }
        }
        *dpi
    }
}

fn setup_surface<F>(
    surface: Main<wl_surface::WlSurface>,
    dpi_change: Option<F>,
) -> wl_surface::WlSurface
where
    F: FnMut(i32, wl_surface::WlSurface) + Send + 'static,
{
    surface.assign_mono(move |surface, event| {
        let mut user_data = surface
            .as_ref()
            .user_data()
            .get::<Mutex<SurfaceUserData>>()
            .unwrap()
            .lock()
            .unwrap();
        match event {
            wl_surface::Event::Enter { output } => {
                user_data.enter(output, (*surface).clone().detach());
            }
            wl_surface::Event::Leave { output } => {
                user_data.leave(&output, (*surface).clone().detach());
            }
            _ => unreachable!(),
        };
    });
    surface.as_ref().user_data().set_threadsafe(|| {
        Mutex::new(SurfaceUserData::new(
            dpi_change.map(|c| Arc::new(Mutex::new(c)) as Arc<Mutex<_>>),
        ))
    });
    (*surface).clone().detach()
}

impl<E: crate::environment::GlobalHandler<wl_compositor::WlCompositor>>
    crate::environment::Environment<E>
{
    /// Create a DPI-aware surface
    ///
    /// This surface will track the outputs it is being displayed on, and compute the
    /// optimal scale factor for these. You can access them using
    /// [`get_surface_scale_factor`](../fn.get_surface_scale_factor.html) and
    /// [`get_surface_outputs`](../fn.get_surface_outputs.html).
    pub fn create_surface(&self) -> wl_surface::WlSurface {
        let compositor = self.require_global::<wl_compositor::WlCompositor>();
        setup_surface(compositor.create_surface(), None::<fn(_, _)>)
    }

    /// Create a DPI-aware surface with callbacks
    ///
    /// This method is like `create_surface`, but the provided callback will also be
    /// notified whenever the DPI factor of this surface change, if you don't want to have to
    /// periodically check it.
    pub fn create_surface_with_dpi_callback<
        F: FnMut(i32, wl_surface::WlSurface) + Send + 'static,
    >(
        &self,
        f: F,
    ) -> wl_surface::WlSurface {
        let compositor = self.require_global::<wl_compositor::WlCompositor>();
        setup_surface(compositor.create_surface(), Some(f))
    }
}

/// Returns the current suggested dpi factor of a surface.
///
/// Panics if the surface was not created using `Environment::create_surface` or
/// `Environment::create_surface_with_dpi_callback`.
pub fn get_surface_scale_factor(surface: &wl_surface::WlSurface) -> i32 {
    let mut surface_data = surface
        .as_ref()
        .user_data()
        .get::<Mutex<SurfaceUserData>>()
        .expect("SCTK: Surface was not created by SCTK.")
        .lock()
        .unwrap();
    surface_data.compute_dpi_factor(surface)
}

/// Returns a list of outputs the surface is displayed on.
///
/// Panics if the surface was not created using `Environment::create_surface` or
/// `Environment::create_surface_with_dpi_callback`.
pub fn get_surface_outputs(surface: &wl_surface::WlSurface) -> Vec<wl_output::WlOutput> {
    surface
        .as_ref()
        .user_data()
        .get::<Mutex<SurfaceUserData>>()
        .expect("SCTK: Surface was not created by SCTK.")
        .lock()
        .unwrap()
        .outputs
        .iter()
        .map(|(ref output, _, _)| output.clone())
        .collect()
}
