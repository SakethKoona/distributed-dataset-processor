use chrono::Utc;
use common::{DatasetProcessingJob, DatasetProcessingTask, ImageTask};
use mongodb::{
    Client,
    bson::{Bson, doc},
    results::{InsertManyResult, InsertOneResult},
};
pub mod types;

use types::*;

impl DBClient {
    pub async fn new(db_name: &str) -> Self {
        let clnt = Client::with_uri_str("mongodb://mongodb:27017")
            .await
            .expect("Failed to connect to MongoDB");
        let db = clnt.database(db_name);

        Self {
            image_tasks: db.collection::<DBImageTask>("image_tasks"),
            dataset_tasks: db.collection::<DBDatasetTask>("dataset_tasks"),
            dataset_batch_tasks: db.collection::<DBDatasetProcessingJob>("dataset_batch_tasks"),
            mappings: db.collection::<DBMapping>("mappings"),
        }
    }

    pub async fn create_mapping(
        &self,
        dataset_task_id: uuid::Uuid,
        image_filename: &str,
        image_task_id: uuid::Uuid,
    ) -> Result<InsertOneResult, String> {
        // first, we want to create the actual struct
        let data = DBMapping {
            id: None,
            dataset_task_id: dataset_task_id,
            image_filename: image_filename.to_string(),
            image_task_id: image_task_id,
        };

        self.mappings
            .insert_one(data, None)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn query_mappings(
        &self,
        dataset_task_id: &uuid::Uuid,
        image_filename: &str,
    ) -> Option<uuid::Uuid> {
        
        let filter = doc! {
            "dataset_task_id": mongodb::bson::to_bson(&dataset_task_id).unwrap(),
            "image_filename": Bson::String(image_filename.to_string()),
        };

        let res_document = self.mappings.find_one(filter, None).await.ok()?;

        println!("{:?}", res_document);

        let result = res_document.map(|map| map.image_task_id);
        // println!("{:?}", result);

        result
    }

    pub async fn db_add_task(&self, task: &ImageTask) -> Result<InsertOneResult, String> {
        self.image_tasks
            .insert_one(<&ImageTask as Into<DBImageTask>>::into(task), None)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn add_multi_operation_dataset(
        &self,
        ds_task: &DatasetProcessingJob,
    ) -> Result<InsertOneResult, String> {
        // First, we convert the DatasetProcessingJob into a dataset batch task

        let db_task = DBDatasetProcessingJob {
            //TODO: Find better approach for batch id here
            batch_id: ds_task
                .batch_id
                .expect("FATAL ERROR: Dataset did not have a batch task"),
            id: None,
            time_created: Utc::now(),
            time_completed: None,
            status: TaskStatus::Waiting,

            dataset_key: ds_task.dataset_key.clone(),
            operations: ds_task.operations.clone(),
        };

        self.dataset_batch_tasks
            .insert_one(db_task, None)
            .await
            .map_err(|e| e.to_string())
    }

    /// Adds a list of dataset processing tasks to the database.
    ///
    /// This asynchronous function takes a vector of `DatasetProcessingTask` items,
    /// converts them into database-compatible entries (`DBDatasetTask`), and inserts
    /// them into the `dataset_tasks` collection.
    ///
    /// # Arguments
    ///
    /// * `task` - A vector of `DatasetProcessingTask` structs representing the tasks to be stored.
    ///
    /// # Returns
    ///
    /// * `Ok(InsertManyResult)` on successful insertion.
    /// * `Err(String)` if the insertion fails, containing the error message.
    ///
    /// # Errors
    ///
    /// Returns an error if the database insertion fails.
    pub async fn add_datasets(
        &self,
        task: &Vec<DatasetProcessingTask>,
    ) -> Result<InsertManyResult, String> {
        let db_entries: Vec<DBDatasetTask> =
            task.iter().map(|el| DBDatasetTask::from(el)).collect();
        self.dataset_tasks
            .insert_many(db_entries, None)
            .await
            .map_err(|e| e.to_string())
    }
}

impl From<&DatasetProcessingTask> for DBDatasetTask {
    fn from(value: &DatasetProcessingTask) -> Self {
        DBDatasetTask {
            id: None,
            task_id: value.task_id,
            batch_id: value.batch_id,
            dataset_key: value.dataset_key.clone(),
            depends_on: value.depends_on,
            operation: value.operation.clone(),

            time_created: Utc::now(),
            time_completed: None,
            status: {
                match value.depends_on {
                    Some(_) => TaskStatus::Waiting,
                    None => TaskStatus::Ready,
                }
            },
        }
    }
}

impl From<&ImageTask> for DBImageTask {
    fn from(task: &ImageTask) -> Self {
        DBImageTask {
            id: None,
            s3_key: task.s3_key.clone(),
            dataset_id: task.dataset_id,
            batch_id: task.batch_id,
            operation: task.operation.clone(),
            task_id: task.task_id,
            time_created: Utc::now(),
            time_completed: None,
            status: TaskStatus::Waiting,
            depends_on: None,
            dependency_dataset_task_id: task.dependency_dataset_task_id,
        }
    }
}
