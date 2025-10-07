use aws_config;

use aws_sdk_s3::{Client, presigning::PresigningConfig};

use axum::{
    Extension, Router,
    response::Json,
    response::{IntoResponse, Response},
    routing::{get, post},
};

use std::{env, sync::Arc, time::Duration};

use tokio::net::TcpListener;

use common::DatasetProcessingJob;
use db_utils::types::DBClient;
use queue::{ProducerClient, admin::KafkaAdmin};
mod utils;
use crate::utils::{APIError, DatasetUploadResponse, UploadRequest};

const S3_BUCKET: &str = "rust-backend-proj-bucket";

async fn get_s3_client() -> Client {
    let config = aws_config::load_from_env().await;
    Client::new(&config)
}

/// Handles the creation of a presigned URL for dataset uploads.
///
/// This endpoint validates the file extension of the uploaded dataset file,
/// then generates and returns a presigned S3 URL for clients to use to upload
/// the dataset directly to S3.
///
/// # Arguments
/// - `state`: Shared application state containing the S3 client.
/// - `request`: The upload request payload, including filename and dataset name.
///
/// # Returns
/// - `200 OK` with a `DatasetUploadResponse` containing the presigned URL and dataset key if successful.
/// - `400 Bad Request` if the file extension is not supported or URL generation fails
#[axum::debug_handler]
async fn create_dataset_upload(
    Extension(state): Extension<utils::AppState>,
    Json(request): Json<UploadRequest>,
) -> Result<Json<DatasetUploadResponse>, Response> {
    // First, we validate the content type
    let valid_ext = ["jpg", "png", "bmp", "tiff", "tif", "zip"];
    let ext = request.filename.split(".").last().unwrap_or("");

    if !valid_ext.contains(&ext) {
        return Err(APIError::UploadError("Wrong File type".to_string()).into_response());
    }

    // Otherwise, we generate a presigned url for the client to use
    let s3_key = format!("uploads/{}/input.zip", request.dataset_name);
    let dur = Duration::from_secs(900);
    let conf = PresigningConfig::expires_in(dur).map_err(|_| {
        APIError::UploadError("Failed to generate presigned URL".to_string()).into_response()
    })?;

    let url = state
        .s3_client
        .put_object()
        .bucket(S3_BUCKET)
        .key(&s3_key)
        .presigned(conf)
        .await
        .map_err(|_| {
            APIError::UploadError("Failed to generate presigned URL".to_string()).into_response()
        })?;

    Ok(Json(DatasetUploadResponse {
        dataset_key: s3_key,
        presigned_url: url.uri().into(),
    }))
}

#[axum::debug_handler]
async fn handle_dataset_task(
    Extension(state): Extension<utils::AppState>,
    Json(mut request): Json<DatasetProcessingJob>,
) -> Result<Json<utils::TaskDispatchResult>, Response> {
    // First, we send the initial batch dataset task to the db before splitting it
    request.batch_id = Some(uuid::Uuid::new_v4());

    if let Err(_) = state.db.add_multi_operation_dataset(&request).await {
        return Err(
            APIError::DatabaseError("Failed to send batched data into DB".to_string())
                .into_response(),
        );
    }

    // First, we add the dataset to the kafka queue, and see our results
    let insertions = match state.kafka_client.send_dataset(request).await {
        Ok(passed) => Ok(passed),
        Err(_) => {
            Err(APIError::SendTaskError("Failed to send task to Queue".to_string()).into_response())
        }
    }?;

    // Next, we insert into the database, but only the successful ones
    let _db_insert_result = match state.db.add_datasets(&insertions.successes).await {
        Ok(res) => Ok(res),
        Err(_) => Err(APIError::DatabaseError("Failed to send to DB".to_string()).into_response()),
    }?;

    Ok(Json(utils::TaskDispatchResult {
        batch_id: insertions.batch_id,
        task_ids: insertions
            .successes
            .into_iter()
            .map(|task| task.task_id)
            .collect(),
        message: "Tasks successfully dispatched".to_string(),
    }))
}

#[tokio::main]
async fn main() {
    println!("Starting server...");

    // Load environment variables
    let broker = env::var("KAFKA_BROKER").expect("Faield to receive variable from environment.");

    // First, we want to make sure that the kafka topic exists, so we can create an admin client
    {
        let admin_client = KafkaAdmin::new(&broker);
        admin_client
            .create_topic("dataset-tasks", 3)
            .await
            .expect("Failed to create topic");
        admin_client
            .create_topic("image-tasks", 3)
            .await
            .expect("Failed to create image topic");
    }

    // Initialize clients
    let db_client = DBClient::new("img-processing-server").await;
    let s3_client = get_s3_client().await;
    let kafka_client = ProducerClient::new(&broker, "dataset-tasks"); // This producer is responsible
    // for sending datasets and
    // datasets only to kafka.

    // Create application state
    let app_state = utils::AppState {
        db: Arc::new(db_client),
        kafka_client: Arc::new(kafka_client),
        s3_client: s3_client,
    };

    // Setup router
    let mut app = Router::new()
        .route("/upload_dataset", post(create_dataset_upload))
        .route("/send_task", post(handle_dataset_task))
        .layer(Extension(app_state));

    app = app.route("/info", get(|| async { "Hello There".to_string() }));

    let listener = TcpListener::bind("0.0.0.0:3030").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
