use std::collections::HashMap;

use async_std::channel::{ Receiver };
use async_std::sync::Arc;
use async_std::sync::Mutex;

use skia_vulkan::skia_safe;
use skia_vulkan::winit;

use winit::event::Event::WindowEvent;
use winit::event::WindowEvent::MouseInput;
use winit::event::MouseButton;
use winit::window::Window;
use winit::window::WindowId;
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;

use crate::iracing::Update;
use crate::plot::PlotOverlay;
use crate::head2head::Head2HeadOverlay;
use crate::track::TrackOverlay;

use async_trait::async_trait;

pub trait Drawable {
    fn draw(&mut self, canvas: &mut skia_safe::Canvas, window_size: (u32, u32));
}

pub trait StateUpdater {
    fn set_state(&mut self, window: &Window);
}

pub struct WindowSpec {
    pub title: String,
    pub width: f32,
    pub height: f32,
}

pub trait Overlay: Drawable + StateUpdater {
    fn window_spec(&self) -> WindowSpec;
} 

pub struct OverlayImpl<'a> {
    pub overlay: Box<dyn Overlay>,
    renderer: skia_vulkan::WindowRenderer<'a>,
    window: &'a Window,
}

pub struct Overlays {
    windows: Vec<Window>,
    event_loop: EventLoop<()>,
    overlays: Vec<Box<dyn Overlay>>,
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
        // let (head2head_overlay, head2head_overlay_state) = Head2HeadOverlay::new();

        let state_updater = async_std::task::spawn(async move {
            let mut state_trackers: Vec<Arc<Mutex<dyn StateTracker + Send + Sync>>> = vec![
                Arc::new(Mutex::new(plot_overlay_state)),
                Arc::new(Mutex::new(track_overlay_state)),
                // Arc::new(Mutex::new(head2head_overlay_state),
            ];

            while let Ok(update) = state_receiver.recv().await {
                if let Update::Telemetry(telemetry) = &update {
                    let delay = telemetry.timestamp.elapsed().as_millis();
                    if delay > (10 as u128) {
                        warn!["Slow data processing. Delay: {}", delay];
                    }
                }

                let arc_update = Arc::new(update);
                let mut tasks = Vec::with_capacity(state_trackers.len());

                for state_tracker in &mut state_trackers {
                    let task_update = arc_update.clone();
                    let cloned_state_tracker = state_tracker.clone();

                    tasks.push(async_std::task::spawn(async move {
                        cloned_state_tracker.lock().await.process(&task_update).await;
                    }));
                }
                for task in tasks {
                    task.await;
                }
            }
        });

        let overlays: Vec<Box<dyn Overlay>> = vec![
            Box::new(plot_overlay),
            Box::new(track_overlay),
            // Box::new(head2head_overlay),
        ];
        let windows: Vec<_> = overlays.iter()
            .map(|overlay| {
                let window_spec = overlay.window_spec();
                create_window(&event_loop, &window_spec.title, window_spec.width, window_spec.height)
            })
            .collect();

        Overlays {
            windows,
            event_loop,
            overlays,
            state_updater,
        }
    }

    pub fn start_event_loop(mut self) {
        let vulkan = skia_vulkan::VulkanInstance::new();
        let static_resources = skia_vulkan::StaticWindowsResources::construct(&vulkan, &self.windows);
        let mut overlay_map: HashMap<_, _> = std::iter::zip(self.windows.iter(), self.overlays.into_iter())
            .map(|(window, overlay)| (window.id(), OverlayImpl::new(overlay, &static_resources, window)))
            .collect();

        self.event_loop.run_return(move |event, _window, control_flow| {
            match event {
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                } => *control_flow = winit::event_loop::ControlFlow::Exit,

                winit::event::Event::MainEventsCleared => {
                    for (_window_id, overlay) in &mut overlay_map {
                        overlay.overlay.set_state(&overlay.window);
                        overlay.window.request_redraw();
                    }
                },

                winit::event::Event::RedrawRequested(window_id) => {
                    if let Some(overlay) = overlay_map.get_mut(&window_id) {
                        let window_size = overlay.window.inner_size();

                        overlay.renderer.draw(window_size.into(), &mut |canvas| {
                            overlay.overlay.draw(canvas, window_size.into());
                        })
                    } else {
                        error!("Unknown window with id {:?}", window_id);
                    }
                },

                WindowEvent { window_id, event: MouseInput { button: MouseButton::Left, .. }, .. } => {
                    if let Some(overlay) = overlay_map.get(&window_id) {
                        overlay.window.drag_window().expect("Failed to drag window");
                    }
                },

                _ => {}
            }
        });
    }
}

fn create_window(event_loop: &EventLoop<()>, name: &str, width: f32, height: f32) -> Window {
    let logical_size = winit::dpi::LogicalSize::new(width, height);
    
    winit::window::WindowBuilder::new()
        .with_title(name)
        .with_inner_size(logical_size)
        .with_decorations(false)
        .with_always_on_top(true)
        .with_transparent(true)
        .with_resizable(true)
        .with_visible(false)
        .build(event_loop)
        .expect("Failed to create overlay window")
}

impl<'a> OverlayImpl<'a> {
    pub fn new(overlay: Box<dyn Overlay>, static_resources: &'a skia_vulkan::StaticWindowsResources, window: &'a Window) -> OverlayImpl<'a> {
        let renderer_config = skia_vulkan::WindowRendererConfigBuilder::default().build().unwrap();
        let renderer = skia_vulkan::WindowRenderer::construct(&static_resources, &window, renderer_config);

        OverlayImpl {
            window,
            renderer,
            overlay,
        }
    }
}
