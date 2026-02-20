use crate::wifi::error::WifiError;
use actix_web::{HttpResponse, ResponseError};
use serde_json::json;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Wifi error: {0}")]
    Wifi(#[from] WifiError),

    #[error("Job not found")]
    JobNotFound,

    #[error("All recorder slots are full")]
    NoAvailableSlots,

    #[error("Invalid settings: {0}")]
    InvalidSettings(String),

    #[error("Job is not idle")]
    JobNotIdle,

    #[error("Job is not running")]
    JobNotRunning,

    #[error("Process error: {0}")]
    Process(#[from] io::Error),

    #[error("Internal server error")]
    Internal,
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let body = json!({
            "error": self.to_string()
        });

        match self {
            ApiError::Wifi(WifiError::NotFound(_))
            | ApiError::JobNotFound
            | ApiError::NoAvailableSlots
            | ApiError::InvalidSettings(_) => HttpResponse::BadRequest().json(body),

            ApiError::JobNotIdle | ApiError::JobNotRunning => HttpResponse::Conflict().json(body),

            ApiError::Wifi(_) | ApiError::Process(_) | ApiError::Internal => {
                HttpResponse::InternalServerError().json(body)
            }
        }
    }
}
