use std::collections::VecDeque;

use hashbrown::HashMap;
use mqtt_codec_kit::common::QualityOfService;
use parking_lot::Mutex;

use crate::{
    store::queue::Queue,
    types::publish::{get_unix_ts, IncomingPublishPacket, OutgoingPublishPacket},
};

pub struct MemoryQueue {
    max_inflight: u16,
    // The ack packet timeout, when reached resent the packet
    timeout: u64,
    qos2_packets: Mutex<HashMap<String, VecDeque<IncomingPublishPacket>>>,
    outgoing_packets: Mutex<HashMap<String, VecDeque<OutgoingPublishPacket>>>,
}

impl MemoryQueue {
    pub fn new(max_inflight: u16, timeout: u64) -> Self {
        Self {
            max_inflight,
            timeout,
            qos2_packets: Default::default(),
            outgoing_packets: Default::default(),
        }
    }

    fn shrink_queue<P>(queue: &mut VecDeque<P>) {
        if queue.capacity() >= 16 && queue.capacity() >= (queue.len() << 2) {
            queue.shrink_to(queue.len() << 1);
        } else if queue.is_empty() {
            queue.shrink_to(0);
        }
    }
}

impl Queue for MemoryQueue {
    type Error = ();

    async fn push_qos2_back(
        &self,
        client_id: &str,
        packet_id: u16,
        message: crate::types::publish::PublishMessage,
    ) -> Result<bool, Self::Error> {
        let mut incoming_packets = self.qos2_packets.lock();
        let packets = incoming_packets
            .entry(client_id.to_string())
            .or_insert_with(VecDeque::new);

        if packets.len() >= self.max_inflight.into() {
            log::error!(
                "drop incoming packet {:?}, queue is full: {}",
                message,
                packets.len()
            );
            return Ok(true);
        }
        packets.push_back(IncomingPublishPacket::new(packet_id, message));
        Ok(false)
    }

    async fn push_outgoing_back(
        &self,
        client_id: &str,
        packet_id: u16,
        subscribe_qos: QualityOfService,
        message: crate::types::publish::PublishMessage,
    ) -> Result<bool, Self::Error> {
        let mut outgoing_packets = self.outgoing_packets.lock();
        let packets = outgoing_packets
            .entry(client_id.to_string())
            .or_insert_with(VecDeque::new);

        if packets.len() >= self.max_inflight.into() {
            log::error!(
                "drop outgoing packet {:?}, queue is full: {}",
                message,
                packets.len()
            );
            return Ok(true);
        }
        packets.push_back(OutgoingPublishPacket::new(
            packet_id,
            subscribe_qos,
            message,
        ));
        Ok(false)
    }

    async fn pubrec(&self, client_id: &str, target_pid: u16) -> Result<bool, Self::Error> {
        match self.outgoing_packets.lock().get_mut(client_id) {
            Some(queue) => {
                if let Some(pos) = queue.iter().position(|packet| {
                    packet.packet_id() == target_pid
                        && packet.message().qos() == QualityOfService::Level2
                        && packet.pubrec_at().is_none()
                        && packet.pubcomp_at().is_none()
                }) {
                    queue[pos].renew_pubrec_at();
                    queue[pos].get_mut_message().set_dup();
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            None => Ok(false),
        }
    }

    async fn puback(&self, client_id: &str, target_pid: u16) -> Result<bool, Self::Error> {
        match self.outgoing_packets.lock().get_mut(client_id) {
            Some(queue) => {
                if let Some(pos) = queue.iter().position(|packet| {
                    packet.packet_id() == target_pid
                        && packet.message().qos() == QualityOfService::Level1
                        && packet.pubcomp_at().is_none()
                }) {
                    queue[pos].renew_pubcomp_at();
                    queue[pos].get_mut_message().set_dup();
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            None => Ok(false),
        }
    }

    async fn pubcomp(&self, client_id: &str, target_pid: u16) -> Result<bool, Self::Error> {
        match self.outgoing_packets.lock().get_mut(client_id) {
            Some(queue) => {
                if let Some(pos) = queue.iter().position(|packet| {
                    packet.packet_id() == target_pid
                        && packet.message().qos() == QualityOfService::Level2
                        && packet.pubrec_at().is_some()
                }) {
                    queue[pos].renew_pubcomp_at();
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            None => Ok(false),
        }
    }

    async fn clean_incoming(&self, client_id: &str) -> Result<(), Self::Error> {
        if let Some(queue) = self.qos2_packets.lock().get_mut(client_id) {
            let mut changed = false;
            let now_ts = get_unix_ts();
            if let Some(pos) = queue.iter().position(|packet| {
                packet.deliver_at().is_some() || now_ts >= self.timeout + packet.receive_at()
            }) {
                changed = true;
                queue.remove(pos);
            }

            if changed {
                Self::shrink_queue(queue);
            }
        }

        Ok(())
    }

    async fn clean_outgoing(&self, client_id: &str) -> Result<(), Self::Error> {
        if let Some(queue) = self.outgoing_packets.lock().get_mut(client_id) {
            let mut changed = false;
            let now_ts = get_unix_ts();
            if let Some(pos) = queue.iter().position(|packet| {
                packet.pubcomp_at().is_some()
                    || now_ts >= self.timeout + packet.pubrec_at().unwrap_or(packet.added_at())
            }) {
                changed = true;
                queue.remove(pos);
            }

            if changed {
                Self::shrink_queue(queue);
            }
        }
        Ok(())
    }

    async fn get_ready_incoming_packets(
        &self,
        client_id: &str,
    ) -> Result<Option<Vec<IncomingPublishPacket>>, Self::Error> {
        match self.qos2_packets.lock().get_mut(client_id) {
            Some(queue) => {
                let now_ts = get_unix_ts();
                let mut ret = Vec::new();
                for packet in queue {
                    if packet.deliver_at().is_none() && now_ts <= self.timeout + packet.receive_at()
                    {
                        ret.push(packet.to_owned());
                    }
                }

                Ok(Some(ret))
            }
            None => Ok(None),
        }
    }

    async fn get_unsent_outgoing_packets(
        &self,
        client_id: &str,
    ) -> Result<Option<Vec<OutgoingPublishPacket>>, Self::Error> {
        match self.outgoing_packets.lock().get_mut(client_id) {
            Some(queue) => {
                let now_ts = get_unix_ts();
                let mut ret = Vec::new();
                for packet in queue {
                    if packet.pubcomp_at().is_none()
                        && packet.pubrec_at().is_none()
                        && now_ts <= self.timeout + packet.pubrec_at().unwrap_or(packet.added_at())
                    {
                        ret.push(packet.to_owned());
                    }
                }
                Ok(Some(ret))
            }
            None => Ok(None),
        }
    }

    async fn remove(&self, client_id: &str) -> Result<(), Self::Error> {
        self.qos2_packets.lock().remove(client_id);
        self.outgoing_packets.lock().remove(client_id);
        Ok(())
    }
}
