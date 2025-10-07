use crate::utils::ConsumerAppState;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use common::{DatasetProcessingTask, ImageTask};
use db_utils::types::DBClient;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use queue::consumer::ConsumerClient;
use queue::ProducerClient;
use std::env;
use std::error::Error;
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::Arc;
use tokio;
use tokio::task::JoinHandle;
use zip::ZipArchive;
mod utils;

async fn process_zip(
    msg: DatasetProcessingTask,
    state: Arc<ConsumerAppState>,
    bucket: &str,
    zip_key: &str,
    valid_extensions: &Vec<&str>,
) -> Result<(), Box<dyn Error>> {
    let zip_arc = Arc::new(zip_key.to_string());
    let resp = state
        .s3
        .get_object()
        .bucket(bucket)
        .key(zip_key)
        .send()
        .await
        .map_err(|_| "Failed to get object from S3")?;

    let stage = msg.stage;
    let data = resp
        .body
        .collect()
        .await
        .map_err(|_| "Failed to collect S3 body")?
        .into_bytes();
    let bufreader = Cursor::new(&data);

    let mut zip_contents = ZipArchive::new(bufreader).map_err(|_| "Failed to read zip archive")?;
    let mut tasks_in_queue: FuturesUnordered<JoinHandle<Result<(), _>>> = FuturesUnordered::new();

    for i in 0..zip_contents.len() {
        let mut file = zip_contents
            .by_index(i)
            .map_err(|_| "Failed to get file from zip")?;
        let filename = file.name().to_string();

        let is_valid_image: bool = filename
            .rsplit('.')
            .next()
            .map(|ext| valid_extensions.iter().any(|&valid| valid == ext))
            .unwrap_or(false);

        if file.is_dir() || !is_valid_image {
            continue; // Skip that image and move to the next
        }

        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).is_err() {
            return Err("Failed to read image from zip".into());
        }

        // Otherwise, we can create that image task, and also send the image key back to s3.
        let s3 = state.s3.clone();
        let bucket = bucket.to_string();
        let database = state.database.clone();
        let operation = msg.operation.clone();
        let producer = state.producer.clone();
        let zip_arc = Arc::clone(&zip_arc);

        tasks_in_queue.push(tokio::spawn(async move { // Each thread will process one image
            // First, we add the image to s3
            let zk = zip_arc;
            let dataset_name: Vec<String> = zk.split("/").map(|s| s.to_string()).collect();

            let s3_put_res = s3
                .put_object()
                .bucket(bucket)
                .key(format!("{}/{}/{}", &dataset_name[1], stage, &filename))
                .body(ByteStream::from(buf))
                .send()
                .await;

            let task_to_send = match s3_put_res {
                Ok(_) => {
                    // Create the initial image task
                    let mut image_task = ImageTask {
                        s3_key: format!("stages/{}/{}", msg.stage, &filename), //TODO: match this to s3 saving scheme
                        dataset_id: msg.task_id,
                        batch_id: msg.batch_id,
                        task_id: Some(uuid::Uuid::new_v4()),
                        operation: operation,
                        depends_on: None,
                        dependency_dataset_task_id: {
                            match msg.depends_on {
                                Some(depends_on) => Some(depends_on),
                                None => None,
                            }
                        },
                    };

                    let _ = database.create_mapping(image_task.dataset_id, &filename, image_task.task_id.expect("Line 110")).await;

                    // Here, we query our mappings to see if the dependency image task already
                    // exists
                    if let Some(val) = &image_task.dependency_dataset_task_id {
                        let depends_on_image =
                            database.query_mappings(val, &filename).await;
                        image_task.depends_on = depends_on_image;
                    }

                    image_task
                }
                Err(_) => {
                    return Err("Failed to send task to queue.");
                }
            };

            let _ = database.db_add_task(&task_to_send).await;

            if let Some(_) = task_to_send.depends_on {
                match producer.send_image_task(task_to_send).await {
                    Ok(_) => Ok(()),
                    Err(_) => Err("Failed to send task to Kafka"),
                }?;
            }

            Ok(())
        }));
    }

    while let Some(result) = tasks_in_queue.next().await {
        match result {
            Ok(inner_result) => {
                if let Err(e) = inner_result {
                    return Err(e.into());
                }
            }
            Err(join_err) => {
                return Err(format!("Join error: {}", join_err).into());
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let broker = env::var("KAFKA_BROKER").expect("CONSUMER: Failed to get env variable");

    let producer = ProducerClient::new(&broker, "image-tasks");
    let db_client = DBClient::new("img-processing-server").await;
    let decomposer_consumer = ConsumerClient::new(&broker, "decompose-tasks", &["dataset-tasks"]);

    let app_state = Arc::new(ConsumerAppState {
        producer: Arc::new(producer),
        consumer: Arc::new(decomposer_consumer),
        database: Arc::new(db_client),
        s3: Arc::new({
            let config = aws_config::load_from_env().await;
            Client::new(&config)
        }),
    });

    let consumer = Arc::clone(&app_state).consumer.clone();

    consumer
        .start_consuming({
            let app_state = Arc::clone(&app_state);
            move |msg: DatasetProcessingTask| {
                let app_state = Arc::clone(&app_state);
                async move {
                    let valid_image_extensions = vec!["png", "jpg", "tiff"];

                    let ext = Path::new(&msg.dataset_key)
                        .extension()
                        .and_then(|e| e.to_str());

                    let key = msg.dataset_key.clone();
                    match ext {
                        Some("zip") => {
                            match process_zip(
                                msg,
                                app_state,
                                "rust-backend-proj-bucket",
                                &key,
                                &valid_image_extensions,
                            )
                            .await
                            {
                                Ok(_) => {
                                    println!("Successfully processed task");
                                }
                                Err(e) => {
                                    println!("Failed to process this task: {}", e);
                                }
                            };
                        }
                        Some(ext) if valid_image_extensions.contains(&ext) => {
                            println!("Single image file received: {}", msg.dataset_key);
                            // TODO: Handle single image
                        }
                        Some(ext) => {
                            eprintln!("Unsupported file extension: {}", ext);
                        }
                        None => {
                            eprintln!(
                                "Could not determine file extension for key: {}",
                                msg.dataset_key
                            );
                        }
                    }
                }
            }
        })
        .await;
}
