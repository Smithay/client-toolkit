use std::sync::{Arc, Mutex};
use wayland_client::Proxy;
use wayland_client::protocol::{wl_output, wl_surface};
use wayland_client::protocol::wl_compositor::{RequestsTrait as CompositorRequest};
use crate::env::Environment;
use crate::output::OutputMgr;

pub struct Surface {
    surface: Proxy<wl_surface::WlSurface>,
    inner: Arc<Mutex<InnerSurface>>
}

struct InnerSurface {
    output_manager: OutputMgr,
    outputs: Vec<Proxy<wl_output::WlOutput>>,
    scale_factor: i32,
    new_scale_factor: Option<i32>,
}

impl InnerSurface {
    pub fn new(env: &Environment) -> Self {
        InnerSurface {
            output_manager: env.outputs.clone(),
            outputs: Vec::new(),
            scale_factor: 1,
            new_scale_factor: None,
        }
    }

    pub fn scale_factor(&self) -> i32 {
        self.scale_factor
    }

    pub fn set_scale_factor(&mut self, scale_factor: i32) {
        if self.scale_factor != scale_factor {
            self.scale_factor = scale_factor;
            self.new_scale_factor = Some(scale_factor);
        }
    }
}

impl Surface {
    pub fn new(env: &Environment) -> Self {
        let inner = Arc::new(Mutex::new(InnerSurface::new(env)));
        let surface = env.compositor.create_surface(|surface| {
            let inner = inner.clone();
            surface.implement(move |event, _surface| match event {
                wl_surface::Event::Enter { output } => {
                    let mut state = inner.lock().unwrap();
                    let old_scale_factor = state.scale_factor();
                    let scale_factor = state.output_manager
                        .with_info(&output, |_id, info| {
                            std::cmp::max(info.scale_factor, old_scale_factor)
                        }).unwrap();
                    state.outputs.push(output);
                    state.set_scale_factor(scale_factor);
                },
                wl_surface::Event::Leave { output } => {
                    let mut state = inner.lock().unwrap();
                    state.outputs.retain(|output2| output.id() != output2.id());
                    let mut scale_factor = 1;
                    for output in &state.outputs {
                        let scale_factor2 = state.output_manager
                            .with_info(&output, |_id, info| info.scale_factor)
                            .unwrap();
                        scale_factor = std::cmp::max(scale_factor, scale_factor2);
                    }
                    state.set_scale_factor(scale_factor);
                },
            }, ())
        }).unwrap();
        Surface {
            surface,
            inner,
        }
    }

    pub fn scale_factor(&self) -> i32 {
        self.inner.lock().unwrap().scale_factor
    }

    pub fn poll_scale_factor_changed(&self) -> Option<i32> {
        self.inner.lock().unwrap().new_scale_factor.take()
    }
}
