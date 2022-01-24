use async_std::channel::Receiver;
use std::time::{ Duration, Instant };
use std::collections::VecDeque;

use skulpin::skia_safe;

use crate::overlay::{ Overlay, Drawable, StateUpdater };
use crate::iracing::{ Update, Telemetry };

pub struct Head2HeadOverlay {
    font: skia_safe::Font,
}

impl Head2HeadOverlay {
    pub fn new() -> Head2HeadOverlay {
        let mut font_collection = skia_safe::textlayout::FontCollection::new();
        font_collection.set_default_font_manager(skia_safe::FontMgr::new(), None);
        // let typeface = font_collection.default_fallback().unwrap();

        let style= skia_safe::FontStyle::normal();
        let families = vec!["Monaco"];
        let typeface = font_collection.find_typefaces(&families, style).pop().unwrap();

        let mut font = skia_safe::Font::new(typeface, None);
        font.set_subpixel(true);

        Head2HeadOverlay {
            font
        }
    }
}

impl Overlay for Head2HeadOverlay {}

impl Drawable for Head2HeadOverlay {
    fn draw(&mut self, canvas: &mut skia_safe::Canvas, coord: &skulpin::CoordinateSystemHelper) {
        canvas.clear(skia_safe::Color::from_argb(100, 100, 255, 255));

        let mut paint = skia_safe::Paint::new(skia_safe::Color4f::new(0.0, 0.0, 0.0, 1.0), None);
                    // paint.set_anti_alias(true);
                    //paint.set_style(skia_safe::paint::Style::Stroke);
                    // paint.set_stroke_width(1.0);

        canvas.draw_str(
            // "Hello 😀",
            "Hello",
            skia_safe::Point::new(50.0, 50.0),
            &self.font,
            &paint
        );
    }
}

impl StateUpdater for Head2HeadOverlay {
    fn update_state(self: &mut Self, update: &Update) {
    }
}
