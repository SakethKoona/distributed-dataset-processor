use chrono::{DateTime, Utc};
use common::ImageOperation;
use mongodb::{
    Collection,
    bson::{doc, oid::ObjectId},
};
use serde::{Deserialize, Serialize};

// ============================================================================
// SHARED ENUMS
// ============================================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TaskStatus {
    Waiting,
    Success,
    Failure,
    Running,
    Ready,
}

// ============================================================================
// DATABASE DOCUMENT TYPES
// These structs represent documents stored in MongoDB collections
// ============================================================================

/// Database representation of a batch processing job
/// Stores high-level information about a dataset processing batch
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DBDatasetProcessingJob {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>, // Internal MongoDB ID

    pub batch_id: uuid::Uuid, // A unique ID, copied straight from the Kafka job
    pub dataset_key: String, // Key of the dataset zip folder inside of s3
    pub operations: Vec<ImageOperation>, // A list of the different operations to be applied
    
    // Additional metadata for the database
    pub time_created: DateTime<Utc>,
    pub time_completed: Option<DateTime<Utc>>,
    pub status: TaskStatus,
}

/// Database representation of a dataset processing task
/// Stores information about a single operation on a dataset
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DBDatasetTask {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub task_id: uuid::Uuid,
    pub batch_id: uuid::Uuid,
    pub dataset_key: String,
    pub depends_on: Option<uuid::Uuid>,
    pub operation: ImageOperation,

    pub time_created: DateTime<Utc>,
    pub time_completed: Option<DateTime<Utc>>,
    pub status: TaskStatus,
}

/// Database representation of an individual image processing task
/// Stores information about processing a single image
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DBImageTask {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub s3_key: String,
    pub dataset_id: uuid::Uuid,
    pub batch_id: uuid::Uuid,
    pub task_id: Option<uuid::Uuid>,
    pub depends_on: Option<uuid::Uuid>,
    pub dependency_dataset_task_id: Option<uuid::Uuid>,
    pub operation: ImageOperation,

    pub time_created: DateTime<Utc>,
    pub time_completed: Option<DateTime<Utc>>,
    pub status: TaskStatus,
}

// ============================================================================
// MAPPING TYPES
// These structs handle relationships between different entities
// ============================================================================

/// Database representation of the mapping between dataset tasks and image tasks
/// Links dataset processing tasks to their constituent image tasks
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DBMapping {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub dataset_task_id: uuid::Uuid,
    pub image_filename: String,
    pub image_task_id: uuid::Uuid,
}

// ============================================================================
// DATABASE CLIENT
// Provides access to MongoDB collections
// ============================================================================
pub struct DBClient {
    pub image_tasks: Collection<DBImageTask>,
    pub dataset_tasks: Collection<DBDatasetTask>,
    pub dataset_batch_tasks: Collection<DBDatasetProcessingJob>,
    pub mappings: Collection<DBMapping>,
}
