use std::time::Instant;

#[derive(Debug)]
pub struct Driver {
    pub name: String,
    pub irating: u32,
    pub safety_rating: f32,
}

#[derive(Debug)]
pub struct Telemetry {
    pub timestamp: Instant,
    pub throttle: f32,
    pub r#break: f32,
    pub gear: u16,
    pub velocity: f32,
    pub deltas: Vec<f32>,
}

#[derive(Debug)]
pub struct SessionInfo {
    pub name: String,
    pub drivers: Vec<Driver>,
}

#[derive(Debug)]
pub enum Update {
    Session(SessionInfo),
    Telemetry(Telemetry),
}
