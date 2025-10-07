use aws_sdk_s3::Client;
use db_utils::types::DBClient;
use queue::{ProducerClient, consumer::ConsumerClient};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ConsumerAppState {
    pub(crate) producer: Arc<ProducerClient>,
    pub(crate) consumer: Arc<ConsumerClient>,
    pub(crate) database: Arc<DBClient>,
    pub(crate) s3: Arc<Client>,
}
