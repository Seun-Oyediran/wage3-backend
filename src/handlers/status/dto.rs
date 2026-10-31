use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub payments: PaymentsStatus,
}

#[derive(Debug, Serialize)]
pub struct PaymentsStatus {
    pub enabled: bool,
    pub message: Option<String>, // only when paused
}
