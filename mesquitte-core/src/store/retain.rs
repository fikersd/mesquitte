use std::future::Future;

use mqtt_codec_kit::common::{QualityOfService, TopicFilter, TopicName};
// #[cfg(feature = "v5")]
use mqtt_codec_kit::v5::control::PublishProperties;

#[derive(Clone)]
pub struct RetainContent {
    // the publisher client id
    client_id: String,
    topic_name: TopicName,
    payload: Vec<u8>,
    // #[cfg(feature = "v5")]
    properties: Option<PublishProperties>,
    qos: QualityOfService,
}

pub trait Retain {
    type Error;

    fn matches(
        &self,
        topic_filter: &TopicFilter,
    ) -> impl Future<Output = Result<Vec<RetainContent>, Self::Error>>;

    fn insert(
        &self,
        content: RetainContent,
    ) -> impl Future<Output = Result<Option<RetainContent>, Self::Error>>;

    fn remove(
        &self,
        topic_name: &TopicName,
    ) -> impl Future<Output = Result<Option<RetainContent>, Self::Error>>;
}
