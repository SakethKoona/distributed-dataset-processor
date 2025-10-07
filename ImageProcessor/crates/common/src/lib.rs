use uuid::Uuid;

// ============================================================================
// SHARED TYPES
// ============================================================================

#[derive(serde::Serialize, Debug, Clone, serde::Deserialize)]
pub enum ImageOperation {
    Resize { scaling_factor: f32 },
    GrayScale,
    Noise { noise_level: f32 },
    InvertColors,
}

// ============================================================================
// KAFKA MESSAGE TYPES
// These structs should only have information that Kafka and our image processing
// workers need to know about. Any additional metadata should be stored in the database.
// ============================================================================

/// Represents a high-level job to process a dataset with multiple operations
/// This is typically the initial message sent to Kafka to start processing.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DatasetProcessingJob {
    pub batch_id: Option<uuid::Uuid>, // A unique ID, generated server-side, to track the entire batch
    pub dataset_key: String,          // Key of the dataset zip folder inside of s3
    pub operations: Vec<ImageOperation>, // A list of the different operations to be applied
}

/// Represents a single dataset processing task (one operation on a dataset)
/// Generated from DatasetProcessingJob and sent to Kafka consumers
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct DatasetProcessingTask {
    pub dataset_key: String, // The S3 key of the dataset to be processed, inherited from the parent job
    pub task_id: uuid::Uuid, // Unique ID for this specific task,  generated server-side
    pub batch_id: uuid::Uuid, // Inherited from the parent job
    pub operation: ImageOperation, // The operation to be performed on the dataset
    pub depends_on: Option<Uuid>, // The ID of the task this task depends on, if it exists
    pub stage: u32,
}

/// Represents an individual image processing task (smallest unit of work)
/// Generated from DatasetProcessingTask for each image in the dataset
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ImageTask {
    pub s3_key: String,              // The S3 key of the image to be processed
    pub dataset_id: uuid::Uuid,      // The ID of the dataset this image belongs to
    pub batch_id: uuid::Uuid,        // The ID of the batch this image belongs to
    pub task_id: Option<uuid::Uuid>, // The ID of the task, if it exists
    pub depends_on: Option<Uuid>,    // The ID of the task this task depends on, if it exists
    pub dependency_dataset_task_id: Option<Uuid>, // The ID of the dataset task this task depends on, if it exists
    pub operation: ImageOperation,                // The operation to be performed on the image
}

// ============================================================================
// API RESPONSE TYPES
// ============================================================================

/// Result structure for batch processing operations
/// Used to track successes and failures when sending tasks to Kafka
pub struct SendDataResult {
    pub batch_id: Uuid,
    pub successes: Vec<DatasetProcessingTask>,
    pub failures: Vec<DatasetProcessingTask>,
}

// ============================================================================
// TRAITS FOR TYPE CONVERSION
// ============================================================================

/// Trait for converting types into individual image tasks
pub trait IntoImageTasks {
    fn into_individual_tasks(self) -> Vec<ImageTask>;
}

/// Trait for converting types into dataset processing tasks
pub trait IntoDatasetTasks {
    fn into_dataset_tasks(self) -> Vec<DatasetProcessingTask>;
}

// ============================================================================
// TRAIT IMPLEMENTATIONS
// ============================================================================

impl IntoDatasetTasks for DatasetProcessingJob {
    fn into_dataset_tasks(self) -> Vec<DatasetProcessingTask> {
        let batch_id = self.batch_id.unwrap_or(Uuid::new_v4());

        self.operations
            .into_iter()
            .scan((None, 0u32), |state, op| {
                let (prev_task_id, stage_counter) = state;
                let task_id = Uuid::new_v4();

                let task = DatasetProcessingTask {
                    dataset_key: self.dataset_key.clone(),
                    task_id,
                    batch_id,
                    operation: op,
                    depends_on: *prev_task_id,
                    stage: *stage_counter,
                };

                *prev_task_id = Some(task_id);
                *stage_counter += 1;
                Some(task)
            })
            .collect()
    }
}

impl std::fmt::Display for DatasetProcessingTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DatasetProcessingTask\ndataset_key: {}\ntask_id: {}\nbatch_id: {}\nOperation: {:?}\ndepends_on: {:?}",
            self.dataset_key, self.task_id, self.batch_id, self.operation, self.depends_on
        )
    }
}
