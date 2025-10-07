use rdkafka::{
    Message,
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
};
use serde::de::DeserializeOwned;
use futures::StreamExt;
pub struct ConsumerClient {
    pub consumer: StreamConsumer,
}

impl ConsumerClient {
    pub fn new(brokers: &str, group_id: &str, topics: &[&str]) -> Self {
        let consumer: StreamConsumer<rdkafka::consumer::DefaultConsumerContext> =
            ClientConfig::new()
                .set("group.id", group_id)
                .set("bootstrap.servers", brokers)
                .set("enable.partition.eof", "false")
                .set("auto.offset.reset", "earliest")
                .create()
                .unwrap();

        consumer
            .subscribe(topics)
            .expect("Failed to create consumer");

        Self { consumer }
    }

    pub async fn start_consuming<F, Fut, I>(&self, mut handler: F)
    where
        F: FnMut(I) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send,
        I: DeserializeOwned + Send + 'static + Clone,
    {
        let mut message_stream = self.consumer.stream();

        while let Some(result) = message_stream.next().await {
            match result {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        let data: I = serde_json::from_slice(payload)
                            .expect("addd actual error handling later, this is just for fixing");
                        
                        handler(data.clone()).await;
                    }
                }
                Err(e) => {
                    println!("Error occurred while consuming messages: {}", e);
                }
            }
        }
    }
}
