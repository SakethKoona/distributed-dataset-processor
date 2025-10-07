use std::sync::Arc;

use aws_sdk_s3::Client; // Add this import
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use db_utils::types::DBClient;
use queue::ProducerClient;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadRequest {
    pub dataset_name: String,
    pub filename: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub image_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetUploadResponse {
    pub dataset_key: String,
    pub presigned_url: String,
}

#[derive(serde::Serialize)]
pub struct SingleTaskResult {
    pub task_id: Option<uuid::Uuid>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(serde::Serialize)]
pub struct TaskDispatchResult {
    pub batch_id: uuid::Uuid,
    pub task_ids: Vec<uuid::Uuid>,
    pub message: String,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DBClient>,
    pub kafka_client: Arc<ProducerClient>,
    pub s3_client: Client, // Add this field
}

#[derive(Debug, Error)]
pub enum APIError {
    #[error("Failed to send task to Producer Queue")]
    SendTaskError(String),

    #[error("Database Error: {0}")]
    DatabaseError(String),

    #[error("Failed to upload image to S3")]
    UploadError(String),
}

impl IntoResponse for APIError {
    fn into_response(self) -> Response {
        let res = match self {
            APIError::SendTaskError(message) => {
                (StatusCode::INTERNAL_SERVER_ERROR, message.to_string())
            }
            APIError::DatabaseError(message) => {
                (StatusCode::INTERNAL_SERVER_ERROR, message.to_string())
            }
            APIError::UploadError(message) => {
                (StatusCode::INTERNAL_SERVER_ERROR, message.to_string())
            }
        };

        res.into_response()
    }
}
