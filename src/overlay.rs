use std::sync::mpsc::Receiver;

use skulpin::skia_safe;
use skulpin::{CoordinateSystemHelper, LogicalSize};
use skulpin::winit;
use skulpin::rafx::api::RafxExtents2D;

use winit::event::Event::WindowEvent;
use winit::event::WindowEvent::MouseInput;
use winit::event::MouseButton;

use crate::iracing::Update;

pub trait Drawable {
    fn draw(self: &Self, canvas: &mut skia_safe::Canvas, coord: &CoordinateSystemHelper);
}

pub trait StateUpdater {
    fn update_state(self: &mut Self, update: &Update);
}

pub struct Overlay<T> where T: Drawable + StateUpdater {
    pub name: String,
    pub state_receiver: Receiver<Update>,
    pub overlay: T,
}

impl<T: Drawable + StateUpdater + 'static> Overlay<T> {
    pub fn create_window(mut self, width: f32, height: f32) {
        let event_loop = winit::event_loop::EventLoop::<()>::with_user_event();

        let logical_size = winit::dpi::LogicalSize::new(width, height);
        let window = winit::window::WindowBuilder::new()
            .with_title(self.name)
            .with_inner_size(logical_size)
            .with_decorations(false)
            .with_always_on_top(true)
            .with_transparent(true)
            .with_resizable(true)
            .build(&event_loop)
            .expect("Failed to create overlay window");

        let visible_range = skulpin::skia_safe::Rect {
            left: 0.0,
            right: logical_size.width as f32,
            top: 0.0,
            bottom: logical_size.height as f32,
        };
        let scale_to_fit = skulpin::skia_safe::matrix::ScaleToFit::Center;

        let window_size = window.inner_size();
        let window_extents = RafxExtents2D {
            width: window_size.width,
            height: window_size.height,
        };

        let mut renderer = skulpin::RendererBuilder::new()
            .coordinate_system(skulpin::CoordinateSystem::VisibleRange(
                visible_range,
                scale_to_fit,
            ))
            .build(&window, window_extents)
            .unwrap();

        event_loop.run(move |event, _window_target, control_flow| {
            match event {
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                } => *control_flow = winit::event_loop::ControlFlow::Exit,

                winit::event::Event::MainEventsCleared => {
                    while let Ok(new_state) = self.state_receiver.try_recv() {
                        self.overlay.update_state(&new_state);
                    }

                    window.request_redraw();
                },

                winit::event::Event::RedrawRequested(_window_id) => {
                    if let Err(e) = renderer.draw(window_extents, window.scale_factor(), |canvas, coordinate_system_helper| {
                        self.overlay.draw(canvas, &coordinate_system_helper);
                    }) {
                        println!("Error during draw: {:?}", e);
                        *control_flow = winit::event_loop::ControlFlow::Exit
                    }
                },

                WindowEvent { event: MouseInput { button: MouseButton::Left, .. }, .. } => {
                    window.drag_window();
                },

                _ => {}
            }
        });
    }
}
