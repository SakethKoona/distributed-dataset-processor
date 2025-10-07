use common::{
    DatasetProcessingJob, DatasetProcessingTask, ImageTask, IntoDatasetTasks, SendDataResult,
};
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
};
use serde_json;
pub mod admin;
pub mod consumer;

#[derive(Clone)]
pub struct ProducerClient {
    producer: FutureProducer,
    topic: String,
}

impl ProducerClient {
    pub fn new(brokers: &str, topic: &str) -> Self {
        let config = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .create()
            .expect("Failed to create new ClientConfig");

        Self {
            producer: config,
            topic: topic.to_string(),
        }
    }

    pub async fn send_image_task(&self, initial_task: ImageTask) -> Result<ImageTask, String> {
        // Generate a new task ID if not provided, otherwise just return the task that we do have
        // already
        if let Some(_) = initial_task.task_id { return Ok(initial_task); }
        let task = ImageTask {
            task_id: Some(uuid::Uuid::new_v4()),
            ..initial_task
        };

        // Serialize the task to JSON
        let json_payload = serde_json::to_string(&task).unwrap();
        let rec: FutureRecord<String, String> =
            FutureRecord::to(&self.topic).payload(&json_payload);

        // Send the task to the Kafka topic
        let result = self.producer.send(rec, Timeout::Never).await;

        // Handle the result of sending the task
        return {
            match result {
                Ok(_) => Ok(task),
                Err(_) => Err("Failed to upload to queue".to_string()),
            }
        };
    }

    // TODO: add retry capability here for any failed tasks
    pub async fn send_dataset(
        &self,
        initial_dataset_task: DatasetProcessingJob,
    ) -> Result<SendDataResult, String> {
        let batch_id = initial_dataset_task.batch_id;

        let tasks = initial_dataset_task.into_dataset_tasks();

        let mut failed: Vec<DatasetProcessingTask> = vec![];
        let mut success: Vec<DatasetProcessingTask> = vec![];

        for task in tasks {
            // Add to first queue
            let json_payload = serde_json::to_string(&task).map_err(|_| {
                "Failed to Serialize Task, please check the structure of the task".to_string()
            })?;

            let result = {
                let rec: FutureRecord<String, String> =
                    FutureRecord::to(&self.topic).payload(&json_payload);
                self.producer.send(rec, Timeout::Never).await
            };

            match result {
                Ok(_) => {
                    success.push(task);
                }
                Err(_) => {
                    failed.push(task);
                }
            };
        }

        Ok(SendDataResult {
            batch_id: batch_id.expect("FATAL ERROR: Dataset Not identifiable"),
            successes: success,
            failures: failed,
        })
    }
}
