use async_std::channel;
use async_std::channel::{ Sender, Receiver };
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
}

pub struct PlotStateTracker {
    sender: Sender<StateUpdate>,
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
            PlotStateTracker { sender },
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
        canvas.clear(skia_safe::Color::from_argb(100, 255, 255, 255));

        let now = Instant::now();
        for plot in &self.plots {
            let mut paint = skia_safe::Paint::new(plot.color, None);
                    paint.set_anti_alias(true);
                    paint.set_style(skia_safe::paint::Style::Stroke);
                    paint.set_stroke_width(2.0);

            for point in &plot.measurements {
                if now.duration_since(point.time) > self.duration {
                    continue;
                }

                let x = ((self.duration - now.duration_since(point.time)).as_secs_f32() / self.duration.as_secs_f32()) * (coord.window_logical_size().width as f32);
                let y = (1f32 - (point.value as f32)) * (coord.window_logical_size().height as f32);

                canvas.draw_circle(
                    skia_safe::Point::new(x, y),
                    2.0,
                    &paint,
                );
            }
        }
    }
}

#[async_trait]
impl StateTracker for PlotStateTracker {
    async fn process(&mut self, update: &Update) {
        match update {
            Update::Telemetry(new_state) => {
                self.sender.send(StateUpdate::AddMeasurement(new_state.clone())).await.unwrap();
            },
            _ => (),
        }
    }
}

impl StateUpdater for PlotOverlay {
    fn set_state(&mut self) {
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
                _ => ()
            }
        }
    }
}
