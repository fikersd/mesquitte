use std::{fmt::Debug, future::Future};

use mqtt_codec_kit::common::QualityOfService;

use crate::types::publish::{IncomingPublishPacket, OutgoingPublishPacket, PublishMessage};

pub trait Queue: Sized + Send + Sync {
    type Error: Debug;

    /// Push a incoming packet into queue, return if the queue is full.
    fn push_incoming(
        &self,
        client_id: &str,
        packet_id: u16,
        message: PublishMessage,
    ) -> impl Future<Output = Result<bool, Self::Error>>;

    /// Push a outgoing packet into queue, return if the queue is full.
    fn push_outgoing(
        &self,
        client_id: &str,
        packet_id: u16,
        subscribe_qos: QualityOfService,
        message: PublishMessage,
    ) -> impl Future<Output = Result<bool, Self::Error>>;

    fn pubrec(
        &self,
        client_id: &str,
        target_pid: u16,
    ) -> impl Future<Output = Result<bool, Self::Error>>;

    fn puback(
        &self,
        client_id: &str,
        target_pid: u16,
    ) -> impl Future<Output = Result<bool, Self::Error>>;

    fn pubcomp(
        &self,
        client_id: &str,
        target_pid: u16,
    ) -> impl Future<Output = Result<bool, Self::Error>>;

    fn clean_incoming(&self, client_id: &str) -> impl Future<Output = Result<(), Self::Error>>;

    fn clean_outgoing(&self, client_id: &str) -> impl Future<Output = Result<(), Self::Error>>;

    fn get_ready_incoming_packets(
        &self,
        client_id: &str,
    ) -> impl Future<Output = Result<Option<Vec<IncomingPublishPacket>>, Self::Error>>;

    fn get_unsent_outgoing_packets(
        &self,
        client_id: &str,
    ) -> impl Future<Output = Result<Option<Vec<OutgoingPublishPacket>>, Self::Error>>;

    fn remove(&self, client_id: &str) -> impl Future<Output = Result<(), Self::Error>>;
}
