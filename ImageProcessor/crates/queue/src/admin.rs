use std::time::Duration;

use rdkafka::{
    admin::{AdminClient, AdminOptions, NewTopic, TopicReplication},
    client::DefaultClientContext,
    config::ClientConfig,
};

pub struct KafkaAdmin {
    admin: AdminClient<DefaultClientContext>,
}

impl KafkaAdmin {
    /// This code is responsible for creating a new admin client
    pub fn new(brokers: &str) -> Self {
        let admin_client: AdminClient<DefaultClientContext> = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .create()
            .unwrap();

        Self {
            admin: admin_client,
        }
    }
    
    /// This function is responsible for creating a topic
    pub async fn create_topic(&self, topic_name: &str, num_partitions: i32) -> Result<(), String> {
        let new_topic = NewTopic::new(topic_name, num_partitions, TopicReplication::Fixed(1));
        let admin_opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(10)));

        let result = self.admin.create_topics(&[new_topic], &admin_opts).await;
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to create topic: {}", e)),
        }
    }
}
