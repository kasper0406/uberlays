use tokio::sync::broadcast::Receiver;
use std::time::{ Duration, Instant };
use std::collections::VecDeque;

use skulpin::skia_safe;

use crate::overlay::{ Overlay, Drawable, StateUpdater };
use crate::iracing::{ Update, Telemetry };

pub struct Head2HeadOverlay {
    
}

impl Head2HeadOverlay {
    pub fn new() -> Head2HeadOverlay {
        Head2HeadOverlay {}
    }
}

impl Overlay for Head2HeadOverlay {}

impl Drawable for Head2HeadOverlay {
    fn draw(&mut self, canvas: &mut skia_safe::Canvas, coord: &skulpin::CoordinateSystemHelper) {
        canvas.clear(skia_safe::Color::from_argb(100, 255, 255, 255));

    }
}

impl StateUpdater for Head2HeadOverlay {
    fn update_state(self: &mut Self, update: &Update) {
    }
}
