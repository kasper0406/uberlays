use std::collections::HashMap;

use async_std::channel;
use async_std::channel::{ Receiver, Sender };

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
use crate::track::TrackOverlay;

use async_trait::async_trait;

pub trait Drawable {
    fn draw(self: &mut Self, canvas: &mut skia_safe::Canvas, coord: &CoordinateSystemHelper);
}

pub trait StateUpdater {
    fn set_state(self: &mut Self);
}

pub struct WindowSpec {
    pub title: String,
    pub width: f32,
    pub height: f32,
}

pub trait Overlay: Drawable + StateUpdater {
    fn window_spec(&self) -> WindowSpec;
} 

pub struct OverlayImpl {
    pub overlay: Box<dyn Overlay>,
    renderer: skulpin::Renderer,
    window: Window,
}

pub struct Overlays {
    event_loop: EventLoop<()>,
    overlays: HashMap<WindowId, OverlayImpl>,
    state_updater: async_std::task::JoinHandle<()>,
}

#[async_trait]
pub trait StateTracker {
    async fn process(&mut self, update: &Update);
}

impl Overlays {
    pub fn new(state_receiver: Receiver<Update>) -> Overlays {
        let event_loop = EventLoop::<()>::with_user_event();

        let (plot_overlay, plot_overlay_state) = PlotOverlay::new();
        let (track_overlay, track_overlay_state) = TrackOverlay::new();
        let (head2head_overlay, head2head_overlay_state) = Head2HeadOverlay::new();

        let overlays: Vec<Box<dyn Overlay>> = vec![
            Box::new(plot_overlay),
            Box::new(track_overlay),
            Box::new(head2head_overlay),
        ];

        let state_updater = async_std::task::spawn(async move {
            let mut state_trackers: Vec<Box<dyn StateTracker + Send + Sync>> = vec![
                Box::new(plot_overlay_state),
                Box::new(track_overlay_state),
                Box::new(head2head_overlay_state),
            ];

            // TODO(knielsen): Do this in parallel!
            while let Ok(update) = state_receiver.recv().await {
                for state_tracker in &mut state_trackers {
                    let test = state_tracker.process(&update);
                    test.await;
                }
            }
        });

        let window_map: HashMap<_, _> = overlays.into_iter()
            .map(|overlay| {
                let window_spec = overlay.window_spec();
                let overlay_impl = OverlayImpl::new(&event_loop, &window_spec.title, window_spec.width, window_spec.height, overlay);
                (
                    overlay_impl.window.id(),
                    overlay_impl
                )
            })
            .collect();

        Overlays {
            event_loop,
            overlays: window_map,
            state_updater,
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
                    for (_window_id, mut overlay) in &mut self.overlays {
                        overlay.overlay.set_state();
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
            overlay,
        }
    }
}
