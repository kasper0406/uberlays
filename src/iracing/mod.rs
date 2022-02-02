use yaml_rust::YamlLoader;

pub mod data_collector;

use std::time::Instant;
use std::convert::TryFrom;

#[derive(Debug, Clone)]
pub struct Driver {
    pub name: String,
    pub irating: u32,
    pub safety_rating: f32,
}

#[derive(Debug, Clone)]
pub struct Telemetry {
    pub timestamp: Instant,
    pub throttle: f32,
    pub brake: f32,
    pub gear: u16,
    pub velocity: f32,
    pub deltas: Vec<f32>,
    pub positions: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackSpec {
    pub name: String,
    pub configuration: String,
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    // pub name: String,
    // pub drivers: Vec<Driver>,

    pub track: TrackSpec,
}

impl TryFrom<&String> for SessionInfo {
    type Error = String;

    fn try_from(str: &String) -> Result<Self, Self::Error> {
        let parsed = &YamlLoader::load_from_str(str)
                .map_err(|err| format!("Failed to parse yaml: {:?}", err))?[0];

        let track_name = parsed["WeekendInfo"]["TrackName"].as_str()
                .ok_or("TrackName not found")?;
        let track_configuration = parsed["WeekendInfo"]["TrackConfigName"].as_str()
                .ok_or("TrackConfigName not found")?;

        Ok(SessionInfo {
            track: TrackSpec {
                name: track_name.to_string(),
                configuration: track_configuration.to_string(),
            }
        })
    }
}

#[derive(Debug, Clone)]
pub enum Update {
    Session(SessionInfo),
    Telemetry(Telemetry),
}
