use tokio::sync::broadcast::Receiver;
use std::time::{ Duration, Instant };
use std::collections::VecDeque;

use skulpin::skia_safe;
use skulpin::LogicalSize;

use crate::overlay::{ Overlay, Drawable, StateUpdater };
use crate::iracing::{ Update, Telemetry };

struct PlotPoint {
    time: Instant,
    value: f64, // Asummed to range in [0, 1]
}

struct Plot {
    measurements: VecDeque<PlotPoint>,
    color: skia_safe::Color4f,

    extractor: Box<dyn Fn(&Telemetry) -> f64>,
}

pub struct PlotOverlay {
    plots: Vec<Plot>,
    duration: Duration,
}

impl<'a> PlotOverlay {
    pub fn new() -> PlotOverlay {
        PlotOverlay {
            duration: Duration::from_secs(10),
            plots: vec![
                Plot {
                    measurements: VecDeque::new(),
                    color: skia_safe::Color4f::new(0.0, 1.0, 0.0, 1.0),
                    extractor: Box::new(|state| state.throttle as f64),
                },
                Plot {
                    measurements: VecDeque::new(),
                    color: skia_safe::Color4f::new(1.0, 0.0, 0.0, 1.0),
                    extractor: Box::new(|state| state.r#break as f64),
                },
            ],
        }
    }
}

impl Overlay for PlotOverlay {}

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

impl StateUpdater for PlotOverlay {
    fn update_state(self: &mut Self, update: &Update) {
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
            Update::Telemetry(new_state) => {
                for plot in &mut self.plots {
                    plot.measurements.push_back(PlotPoint {
                        time: new_state.timestamp.clone(),
                        value: (plot.extractor)(&new_state),
                    });
                }
            },
            _ => ()
        }
    }
}
