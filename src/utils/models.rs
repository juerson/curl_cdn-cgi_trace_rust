use serde::{ Deserialize, Serialize };
use std::time;

#[derive(Debug, Deserialize, Clone)]
pub struct Airport {
    pub iata: String,
    pub cca2: String,
    pub region: String,
    pub city: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Record {
    pub ip: String,
    pub colo: String,
    pub country: String,
    pub region: String,
    pub city: String,
    pub delay: time::Duration,
    pub is_jetbrains: bool, // 是否为JetBrains的License服务器
    pub http_status_code: String,
}
