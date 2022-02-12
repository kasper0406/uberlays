use async_std::channel;
use async_std::channel::{ Sender, Receiver };
use skulpin::winit::window::Window;
use std::time::{ Duration, Instant };
use std::collections::VecDeque;

use skulpin::skia_safe;


use crate::overlay::{ Overlay, Drawable, StateUpdater, StateTracker, WindowSpec };
use crate::iracing::{ Update, Telemetry };

use async_trait::async_trait;

struct PlotPoint {
    time: Instant,
    value: f64, // Asummed to range in [0, 1]
}

type Extractor = dyn Fn(&Telemetry) -> f64 + Send;

enum StateUpdate {
    AddMeasurement(Telemetry),
    WindowVisible(bool),
}

pub struct PlotStateTracker {
    sender: Sender<StateUpdate>,
    is_visible: bool,
}

struct Plot {
    measurements: VecDeque<PlotPoint>,
    color: skia_safe::Color4f,

    extractor: Box<Extractor>,
}

pub struct PlotOverlay {
    plots: Vec<Plot>,
    duration: Duration,
    receiver: Receiver<StateUpdate>,
}

impl<'a> PlotOverlay {
    pub fn new() -> (PlotOverlay, PlotStateTracker) {
        let (sender, receiver) = channel::unbounded();
        (
            PlotOverlay {
                duration: Duration::from_secs(15),
                plots: vec![
                    Plot {
                        measurements: VecDeque::new(),
                        color: skia_safe::Color4f::new(0.0, 1.0, 0.0, 1.0),
                        extractor: Box::new(|state| state.throttle as f64),
                    },
                    Plot {
                        measurements: VecDeque::new(),
                        color: skia_safe::Color4f::new(1.0, 0.0, 0.0, 1.0),
                        extractor: Box::new(|state| state.brake as f64),
                    },
                ],
                receiver,
            },
            PlotStateTracker {
                sender,
                is_visible: false,
            },
        )
    }
}

impl Overlay for PlotOverlay {
    fn window_spec(&self) -> WindowSpec {
        WindowSpec {
            title: "Plot".to_string(),
            width: 500.0,
            height: 90.0,
        }
    }
}

impl Drawable for PlotOverlay {
    fn draw(&mut self, canvas: &mut skia_safe::Canvas, coord: &skulpin::CoordinateSystemHelper) {
        let margin = 2;
        canvas.clear(skia_safe::Color::from_argb(100, 255, 255, 255));

        let now = Instant::now();
        for plot in &self.plots {
            let mut paint = skia_safe::Paint::new(plot.color, None);
            paint.set_anti_alias(true);
            paint.set_style(skia_safe::paint::Style::Stroke);
            paint.set_stroke_width(4.0);

            let mut prev_point = None;
            for point in &plot.measurements {
                if now.duration_since(point.time) > self.duration {
                    continue;
                }

                let x = ((self.duration - now.duration_since(point.time)).as_secs_f32() / self.duration.as_secs_f32()) * (coord.window_logical_size().width as f32);
                let y = (1f32 - (point.value as f32)) * ((coord.window_logical_size().height - (2 * margin)) as f32) + (margin as f32);
                let skia_point = skia_safe::Point::new(x, y);

                if let Some(prev_skia_point) = prev_point {
                    canvas.draw_line(prev_skia_point, skia_point, &paint);
                }

                prev_point = Some(skia_point);
            }
        }
    }
}

#[async_trait]
impl StateTracker for PlotStateTracker {
    async fn process(&mut self, update: &Update) {
        match update {
            Update::Telemetry(new_state) => {
                let measurement_sender = self.sender.send(StateUpdate::AddMeasurement(new_state.clone()));

                if self.is_visible != new_state.is_on_track {
                    self.sender.send(StateUpdate::WindowVisible(new_state.is_on_track)).await.unwrap();
                    self.is_visible = new_state.is_on_track;
                }
                
                measurement_sender.await.unwrap();
            },
            _ => (),
        }
    }
}

impl StateUpdater for PlotOverlay {
    fn set_state(&mut self, window: &Window) {
        if let Ok(update) = self.receiver.try_recv() {
            let now = Instant::now();
            for plot in &mut self.plots {
                while let Some(measurement) = plot.measurements.front() {
                    if now.duration_since(measurement.time) < self.duration {
                        break;
                    }
                    plot.measurements.pop_front();
                }
            }

            match update {
                StateUpdate::AddMeasurement(telemetry) => {
                    for plot in &mut self.plots {
                        plot.measurements.push_back(PlotPoint {
                            time: telemetry.timestamp,
                            value: (plot.extractor)(&telemetry),
                        });
                    }
                },
                StateUpdate::WindowVisible(visible) => window.set_visible(visible),
                _ => ()
            }
        }
    }
}
