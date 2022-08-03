use skia_vulkan::skia_safe;
use skia_vulkan::winit::window::Window;

use crate::overlay::{ Overlay, Drawable, StateUpdater, StateTracker, WindowSpec };
use crate::iracing::{ Update };

use async_trait::async_trait;

pub struct Head2HeadOverlay {
    font: skia_safe::Font,
}

pub struct Head2HeadStateTracker {

}

impl Head2HeadOverlay {
    pub fn new() -> (Head2HeadOverlay, Head2HeadStateTracker) {
        let mut font_collection = skia_safe::textlayout::FontCollection::new();
        font_collection.set_default_font_manager(skia_safe::FontMgr::new(), None);
        // let typeface = font_collection.default_fallback().unwrap();

        let style= skia_safe::FontStyle::normal();
        let families = vec!["Monaco"];
        let typeface = font_collection.find_typefaces(&families, style).pop().unwrap();

        let mut font = skia_safe::Font::new(typeface, None);
        font.set_subpixel(true);

        (
            Head2HeadOverlay {
                font
            },
            Head2HeadStateTracker { },
        )
    }
}

impl Overlay for Head2HeadOverlay {
    fn window_spec(&self) -> WindowSpec {
        WindowSpec {
            title: "Head <-> Head".to_string(),
            width: 300.0,
            height: 600.0,
        }
    }
}

impl Drawable for Head2HeadOverlay {
    fn draw(&mut self, canvas: &mut skia_safe::Canvas, _window_size: (u32, u32)) {
        canvas.clear(skia_safe::Color::from_argb(100, 100, 255, 255));

        let paint = skia_safe::Paint::new(skia_safe::Color4f::new(0.0, 0.0, 0.0, 1.0), None);
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

#[async_trait]
impl StateTracker for Head2HeadStateTracker {
    async fn process(&mut self, _update: &Update) {
        // TODO
    }
}

impl StateUpdater for Head2HeadOverlay {
    fn set_state(&mut self, window: &Window) {
        // TODO
    }
}
