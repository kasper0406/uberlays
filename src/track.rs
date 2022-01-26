use async_std::channel::Receiver;
use std::time::{ Duration, Instant };
use std::collections::VecDeque;

use skulpin::skia_safe;
use skulpin::skia_safe::Point;
use skulpin::skia_safe::Path;
use skulpin::skia_safe::ContourMeasure;
use skulpin::skia_safe::ContourMeasureIter;

use crate::overlay::{ Overlay, Drawable, StateUpdater };
use crate::iracing::{ Update, Telemetry };

pub struct TrackOverlay {
    position: f32,
}

impl TrackOverlay {
    pub fn new() -> TrackOverlay {
        TrackOverlay {
            position: 0.0,
        }
    }
}

impl Overlay for TrackOverlay {}

impl Drawable for TrackOverlay {
    fn draw(&mut self, canvas: &mut skia_safe::Canvas, coord: &skulpin::CoordinateSystemHelper) {
        canvas.clear(skia_safe::Color::from_argb(0, 0, 0, 0));

        let mut paint = skia_safe::Paint::new(skia_safe::Color4f::new(0.0, 0.0, 0.0, 1.0), None);
        paint.set_anti_alias(true);
        paint.set_style(skia_safe::paint::Style::Stroke);
        paint.set_stroke_width(2.0);

        let mut path = Path::new();
        path.move_to(Point::new(10.0, 10.0));
        path.quad_to(Point::new(256.0, 64.0), Point::new(128.0, 128.0));
        path.quad_to(Point::new(10.0, 192.0), Point::new(250.0, 250.0));

        let mut measures = ContourMeasureIter::from_path(&path, false, 1.0);
        if let Some(measure) = measures.next() {
            let length = measure.length();
            if let Some((point, _tangent)) = measure.pos_tan(self.position * length) {
                canvas.draw_circle(point, 10.0, &paint);
            }
        }

        canvas.draw_path(&path, &paint);
    }
}

impl StateUpdater for TrackOverlay {
    fn update_state(self: &mut Self, update: &Update) {
        self.position = (self.position + 0.002) % 1.0;
    }
}
