use std::collections::HashMap;

use tokio::sync::broadcast::Receiver;

use skulpin::skia_safe;
use skulpin::{CoordinateSystemHelper, LogicalSize};
use skulpin::winit;
use skulpin::rafx::api::RafxExtents2D;

use winit::event::Event::WindowEvent;
use winit::event::WindowEvent::MouseInput;
use winit::event::MouseButton;
use winit::window::Window;
use winit::window::WindowId;
use winit::event_loop::EventLoop;

use crate::iracing::Update;
use crate::plot::PlotOverlay;
use crate::head2head::Head2HeadOverlay;

pub trait Drawable {
    fn draw(self: &mut Self, canvas: &mut skia_safe::Canvas, coord: &CoordinateSystemHelper);
}

pub trait StateUpdater {
    fn update_state(self: &mut Self, update: &Update);
}

pub trait Overlay: Drawable + StateUpdater { } 

pub struct OverlayImpl {
    pub overlay: Box<dyn Overlay>,
    renderer: skulpin::Renderer,
    window: Window,
}

pub struct Overlays {
    event_loop: EventLoop<()>,
    overlays: HashMap<WindowId, OverlayImpl>,
    state_receiver: Receiver<Update>,
}

impl Overlays {
    pub fn new(state_receiver: Receiver<Update>) -> Overlays {
        let event_loop = EventLoop::<()>::with_user_event();

        let plot_overlay = OverlayImpl::new(&event_loop, "Plot", 800.0, 160.0, Box::new(PlotOverlay::new()));
        let head2head_overlay = OverlayImpl::new(&event_loop, "Head2Head", 300.0, 600.0, Box::new(Head2HeadOverlay::new()));

        let mut window_map = HashMap::new();
        window_map.insert(plot_overlay.window.id(), plot_overlay);
        window_map.insert(head2head_overlay.window.id(), head2head_overlay);

        Overlays {
            state_receiver,
            event_loop,
            overlays: window_map,
        }
    }

    pub fn start_event_loop(mut self) {
        self.event_loop.run(move |event, window, control_flow| {
            match event {
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                } => *control_flow = winit::event_loop::ControlFlow::Exit,

                winit::event::Event::MainEventsCleared => {
                    let mut updates = Vec::with_capacity(5);
                    while let Ok(new_state) = self.state_receiver.try_recv() {
                        updates.push(new_state);
                    }

                    for (_window_id, mut overlay) in &mut self.overlays {
                        for update in &updates {
                            overlay.overlay.update_state(update);
                        }
                        overlay.window.request_redraw();
                    }
                },

                winit::event::Event::RedrawRequested(window_id) => {
                    if let Some(overlay) = self.overlays.get_mut(&window_id) {
                        let window_size = overlay.window.inner_size();
                        let window_extents = RafxExtents2D {
                            width: window_size.width,
                            height: window_size.height,
                        };
                        let scale_factor = overlay.window.scale_factor();

                        if let Err(e) = overlay.renderer.draw(window_extents, scale_factor, |canvas, coordinate_system_helper| {
                            overlay.overlay.draw(canvas, &coordinate_system_helper);
                        }) {
                            error!("Error during draw: {:?}", e);
                            *control_flow = winit::event_loop::ControlFlow::Exit
                        }
                    } else {
                        error!("Unknown window with id {:?}", window_id);
                    }
                },

                WindowEvent { window_id, event: MouseInput { button: MouseButton::Left, .. }, .. } => {
                    if let Some(overlay) = self.overlays.get(&window_id) {
                        overlay.window.drag_window().expect("Failed to drag window");
                    }
                },

                _ => {}
            }
        });
    }
}

impl OverlayImpl {
    pub fn new(event_loop: &EventLoop<()>, name: &str, width: f32, height: f32, overlay: Box<dyn Overlay>) -> OverlayImpl {
        let logical_size = winit::dpi::LogicalSize::new(width, height);
        let window = winit::window::WindowBuilder::new()
            .with_title(name)
            .with_inner_size(logical_size)
            .with_decorations(false)
            .with_always_on_top(true)
            .with_transparent(true)
            .with_resizable(true)
            .build(&event_loop)
            .expect("Failed to create overlay window");

        let window_size = window.inner_size();
        let window_extents = RafxExtents2D {
            width: window_size.width,
            height: window_size.height,
        };

        let mut renderer = skulpin::RendererBuilder::new()
            .coordinate_system(skulpin::CoordinateSystem::Logical)
            .build(&window, window_extents)
            .unwrap();

        OverlayImpl {
            window,
            renderer,
            overlay
        }
    }
}
