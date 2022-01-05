use std::time::Instant;

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
    pub r#break: f32,
    pub gear: u16,
    pub velocity: f32,
    pub deltas: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub name: String,
    pub drivers: Vec<Driver>,
}

#[derive(Debug, Clone)]
pub enum Update {
    Session(SessionInfo),
    Telemetry(Telemetry),
}
