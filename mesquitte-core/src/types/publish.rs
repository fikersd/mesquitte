use std::cmp;
use std::sync::Arc;
use std::time::SystemTime;

use mqtt_codec_kit::common::{QualityOfService, TopicName};
// #[cfg(feature = "v4")]
use mqtt_codec_kit::v4::{
    packet::connect::LastWill as V4LastWill, packet::PublishPacket as V4PublishPacket,
};
// #[cfg(feature = "v5")]
use mqtt_codec_kit::v5::{
    control::PublishProperties, packet::connect::LastWill as V5LastWill,
    packet::PublishPacket as V5PublishPacket,
};

use super::retain_content::RetainContent;

#[derive(Debug, Clone)]
pub struct PublishMessage {
    topic_name: TopicName,
    payload: Vec<u8>,
    qos: QualityOfService,
    retain: bool,
    dup: bool,
    properties: Option<PublishProperties>,
}

impl PublishMessage {
    pub fn topic_name(&self) -> &TopicName {
        &self.topic_name
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn qos(&self) -> QualityOfService {
        self.qos
    }

    pub fn dup(&self) -> bool {
        self.dup
    }

    pub fn set_dup(&mut self) {
        self.dup = true
    }

    pub fn retain(&self) -> bool {
        self.retain
    }

    pub fn properties(&self) -> Option<&PublishProperties> {
        self.properties.as_ref()
    }
}

impl From<V4PublishPacket> for PublishMessage {
    fn from(packet: V4PublishPacket) -> Self {
        let mut payload = vec![0u8; packet.payload().len()];
        payload.copy_from_slice(packet.payload());

        Self {
            topic_name: packet.topic_name().to_owned(),
            payload,
            qos: packet.qos().into(),
            retain: packet.retain(),
            dup: packet.dup(),
            properties: None,
        }
    }
}

impl From<V5PublishPacket> for PublishMessage {
    fn from(packet: V5PublishPacket) -> Self {
        let mut payload = vec![0u8; packet.payload().len()];
        payload.copy_from_slice(packet.payload());

        Self {
            topic_name: packet.topic_name().to_owned(),
            payload,
            qos: packet.qos().into(),
            retain: packet.retain(),
            dup: packet.dup(),
            properties: Some(packet.properties().to_owned()),
        }
    }
}

impl From<Arc<RetainContent>> for PublishMessage {
    fn from(packet: Arc<RetainContent>) -> Self {
        let mut payload = vec![0u8; packet.payload().len()];
        payload.copy_from_slice(packet.payload());

        Self {
            topic_name: packet.topic_name().to_owned(),
            payload,
            qos: packet.qos().to_owned(),
            retain: false,
            dup: false,
            properties: packet.properties().cloned(),
        }
    }
}

// #[cfg(feature = "v4")]
impl From<V4LastWill> for PublishMessage {
    fn from(value: V4LastWill) -> Self {
        let mut payload = vec![0u8; value.message().0.len()];
        payload.copy_from_slice(&value.message().0);

        Self {
            topic_name: value.topic().to_owned(),
            payload,
            qos: value.qos(),
            retain: value.retain(),
            properties: None,
            dup: false,
        }
    }
}

// #[cfg(feature = "v5")]
impl From<V5LastWill> for PublishMessage {
    fn from(value: V5LastWill) -> Self {
        let mut payload = vec![0u8; value.message().0.len()];
        payload.copy_from_slice(&value.message().0);

        let mut publish_properties = PublishProperties::default();
        let properties = value.properties();
        publish_properties.set_payload_format_indicator(properties.payload_format_indicator());
        publish_properties.set_message_expiry_interval(properties.message_expiry_interval());
        publish_properties.set_response_topic(properties.response_topic().clone());
        publish_properties.set_correlation_data(properties.correlation_data().clone().map(|v| v.0));
        for (key, value) in properties.user_properties() {
            publish_properties.add_user_property(key, value);
        }
        publish_properties.set_content_type(properties.content_type().clone());

        Self {
            topic_name: value.topic().to_owned(),
            payload,
            qos: value.qos(),
            retain: value.retain(),
            dup: false,
            properties: Some(publish_properties),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutgoingPublishPacket {
    packet_id: u16,
    subscribe_qos: QualityOfService,
    message: PublishMessage,
    added_at: u64,
    pubrec_at: Option<u64>,
    pubcomp_at: Option<u64>,
}

impl OutgoingPublishPacket {
    pub fn new(packet_id: u16, subscribe_qos: QualityOfService, message: PublishMessage) -> Self {
        Self {
            packet_id,
            message,
            subscribe_qos,
            added_at: get_unix_ts(),
            pubrec_at: None,
            pubcomp_at: None,
        }
    }

    pub fn packet_id(&self) -> u16 {
        self.packet_id
    }

    pub fn message(&self) -> &PublishMessage {
        &self.message
    }

    pub fn get_mut_message(&mut self) -> &mut PublishMessage {
        &mut self.message
    }

    pub fn subscribe_qos(&self) -> QualityOfService {
        self.subscribe_qos
    }

    pub fn final_qos(&self) -> QualityOfService {
        cmp::min(self.subscribe_qos, self.message.qos())
    }

    pub fn added_at(&self) -> u64 {
        self.added_at
    }

    pub fn pubrec_at(&self) -> Option<u64> {
        self.pubrec_at
    }

    pub fn renew_pubrec_at(&mut self) {
        self.pubrec_at = Some(get_unix_ts())
    }

    pub fn pubcomp_at(&self) -> Option<u64> {
        self.pubcomp_at
    }

    pub fn renew_pubcomp_at(&mut self) {
        self.pubcomp_at = Some(get_unix_ts())
    }
}

#[derive(Debug, Clone)]
pub struct IncomingPublishPacket {
    message: PublishMessage,
    packet_id: u16,
    receive_at: u64,
    deliver_at: Option<u64>,
}

impl IncomingPublishPacket {
    pub fn new(packet_id: u16, message: PublishMessage) -> Self {
        Self {
            message,
            packet_id,
            receive_at: get_unix_ts(),
            deliver_at: None,
        }
    }

    pub fn message(&self) -> &PublishMessage {
        &self.message
    }

    pub fn packet_id(&self) -> u16 {
        self.packet_id
    }

    pub fn receive_at(&self) -> u64 {
        self.receive_at
    }

    pub fn deliver_at(&self) -> Option<u64> {
        self.deliver_at
    }

    pub fn renew_deliver_at(&mut self) {
        self.deliver_at = Some(get_unix_ts())
    }
}

/// Unix timestamp as seconds
pub fn get_unix_ts() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}
