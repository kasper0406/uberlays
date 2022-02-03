use async_std::channel;
use async_std::channel::{ Sender, Receiver };
use std::time::{ Duration, Instant };
use std::collections::VecDeque;

use skulpin::skia_safe;
use skulpin::skia_safe::Point;
use skulpin::skia_safe::Path;
use skulpin::skia_safe::ContourMeasure;
use skulpin::skia_safe::ContourMeasureIter;

use crate::overlay::{ Overlay, Drawable, StateUpdater, StateTracker, WindowSpec };
use crate::iracing::{ Update, Telemetry, TrackSpec };

use async_std::fs::File;
use async_std::prelude::*;
use prost::Message;

use async_trait::async_trait;

#[derive(Clone)]
pub struct State {
    positions: Vec<f32>,
    track: Option<Track>,
}

pub struct TrackOverlay {
    state: State,
    receiver: Receiver<State>,
}

pub struct TrackOverlayState {
    current_state: State,
    last_seen_track: Option<TrackSpec>,
    sender: Sender<State>
}

impl TrackOverlay {
    pub fn new() -> (TrackOverlay, TrackOverlayState) {
        let (sender, receiver) = channel::unbounded();
        let start_state = State {
            positions: vec![],
            track: None,
        };

        (
            TrackOverlay {
                state: start_state.clone(),
                receiver,
            },
            TrackOverlayState {
                sender,
                current_state: start_state.clone(),
                last_seen_track: None,
            }
        )
    }
}

impl Overlay for TrackOverlay {
    fn window_spec(&self) -> WindowSpec {
        WindowSpec {
            title: "Track".to_string(),
            width: 300.0,
            height: 300.0,
        }
    }
}

fn scale(x: f64, coord: &skulpin::CoordinateSystemHelper) -> f32 {
    let scale = 0.95;
    let logical_size = coord.window_logical_size();
    let min_size = logical_size.width.min(logical_size.height) as f64;
    (x * min_size * scale + ((1.0 - scale) / 2.0) * min_size) as f32
}

impl Drawable for TrackOverlay {
    fn draw(&mut self, canvas: &mut skia_safe::Canvas, coord: &skulpin::CoordinateSystemHelper) {
        canvas.clear(skia_safe::Color::from_argb(0, 0, 0, 0));

        let mut track_paint = skia_safe::Paint::new(skia_safe::Color4f::new(0.5, 0.5, 0.5, 0.8), None);
        track_paint.set_anti_alias(true);
        track_paint.set_style(skia_safe::paint::Style::Stroke);
        track_paint.set_stroke_width(7.0);

        if let Some(track) = &self.state.track {
            if track.curve.len() == 0 {
                return;
            }

            let mut path = Path::new();
            let mut prev_point = &track.curve[0];

            path.move_to(Point::new(
                scale(prev_point.control.as_ref().unwrap().x, coord),
                scale(1.0 - prev_point.control.as_ref().unwrap().y, coord)));
            for i in 1..track.curve.len() {
                let next_point = &track.curve[i];

                // TODO(knielsen): Consider what to do if Bezier curve is written in the reverse direction
                path.cubic_to(
                    Point::new(
                        scale(prev_point.handle_right.as_ref().unwrap().x, coord),
                        scale(1.0 - prev_point.handle_right.as_ref().unwrap().y, coord)
                    ),
                    Point::new(
                        scale(next_point.handle_left.as_ref().unwrap().x, coord),
                        scale(1.0 - next_point.handle_left.as_ref().unwrap().y, coord)
                    ),
                    Point::new(
                        scale(next_point.control.as_ref().unwrap().x, coord),
                        scale(1.0 - next_point.control.as_ref().unwrap().y, coord)
                    ));
                
                prev_point = &next_point;
            }
            canvas.draw_path(&path, &track_paint);

            let mut measures = ContourMeasureIter::from_path(&path, false, 1.0);
            if let Some(measure) = measures.next() {
                let length = measure.length();

                for position in &self.state.positions {
                    if let Some((point, _tangent)) = measure.pos_tan((1.0 - position) * length) {
                        let mut car_paint = skia_safe::Paint::new(skia_safe::Color4f::new(0.2, 0.2, 1.0, 1.0), None);
                        car_paint.set_anti_alias(true);
                        canvas.draw_circle(point, 4.0, &car_paint);
                    }
                }
            }
        }
    }
}

#[async_trait]
impl StateTracker for TrackOverlayState {
    async fn process(&mut self, update: &Update) {
        let mut new_state = self.current_state.clone();
        match update {
            Update::Telemetry(telemetry) => {
                new_state.positions = telemetry.positions.clone();
            },
            Update::Session(session_info) => {
                let track_info_clone = Some(session_info.track.clone());
                if track_info_clone != self.last_seen_track {
                    self.last_seen_track = track_info_clone;
                    match load_track(&session_info.track.name, &session_info.track.configuration).await {
                        Ok(track) => new_state.track = Some(track),
                        Err(err) => error!["Failed to load track: {}", err],
                    }
                }
            }
        }

        self.current_state = new_state.clone();
        self.sender.send(new_state).await.unwrap();
    }
}

impl StateUpdater for TrackOverlay {
    fn set_state(self: &mut Self) {
        if let Ok(new_state) = self.receiver.try_recv() {
            self.state = new_state;
        }
    }
}

pub mod track {
    include!(concat!(env!("OUT_DIR"), "/overlay.track.rs"));
}
use track::Track;

async fn load_track(track: &str, layout: &str) -> Result<Track, String> {
    let path = format!["media/tracks/{}/{}.dat", track, layout];
    info!["Loading track file {}", path];

    let path_clone = path.clone();
    let mut track_file = File::open(path).await
        .map_err(|_err| format!["Could not find map file {}", path_clone])?;
    let mut buffer = vec![];
    track_file.read_to_end(&mut buffer).await
        .map_err(|_err| format!["Failed to read file contents of {}", path_clone])?;
    Track::decode(&*buffer)
        .map_err(|_err| format!["Map file is in wrong format!"])
}
