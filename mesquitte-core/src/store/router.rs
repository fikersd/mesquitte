use std::future::Future;

use ahash::HashMap;
use mqtt_codec_kit::common::{QualityOfService, TopicFilter, TopicName};
use mqtt_codec_kit::v5::packet::subscribe::SubscribeOptions;

#[derive(Debug, Clone)]
pub enum RouteOptions {
    V4(QualityOfService),
    V5(SubscribeOptions),
}

pub struct RouteContent {
    topic_filter: TopicFilter,
    clients: HashMap<String, RouteOptions>,
    shared_clients: Option<HashMap<String, RouteOptions>>,
}

pub trait Router {
    type Error;

    fn matches(
        &self,
        topic_name: &TopicName,
    ) -> impl Future<Output = Result<Vec<RouteContent>, Self::Error>>;

    fn subscribe(
        &self,
        client_id: &str,
        topic_filter: &TopicFilter,
        options: RouteOptions,
    ) -> impl Future<Output = Result<bool, Self::Error>>;

    fn unsubscribe(
        &self,
        client_id: &str,
        topic_filter: &TopicFilter,
    ) -> impl Future<Output = Result<bool, Self::Error>>;

    async fn remove_client(&self, client_id: &str);
}
