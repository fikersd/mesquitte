use std::{collections::VecDeque, sync::Arc};

use mqtt_codec_kit::v4::packet::{
    suback::SubscribeReturnCode, SubackPacket, SubscribePacket, UnsubackPacket, UnsubscribePacket,
    VariablePacket,
};

use crate::{server::state::GlobalState, store::queue::Queue, types::session::Session};

use super::publish::receive_outgoing_publish;

pub(super) fn handle_subscribe<Q>(
    session: &mut Session,
    packet: SubscribePacket,
    global: Arc<GlobalState<Q>>,
) -> Vec<VariablePacket>
where
    Q: Queue,
{
    log::debug!(
        r#"client#{} received a subscribe packet:
packet id : {}
   topics : {:?}"#,
        session.client_id(),
        packet.packet_identifier(),
        packet.subscribes(),
    );
    let mut return_codes = Vec::with_capacity(packet.subscribes().len());
    let mut retain_packets: Vec<VariablePacket> = Vec::new();
    for (filter, subscribe_qos) in packet.subscribes() {
        if filter.is_shared() {
            log::warn!("mqtt v3.x don't support shared subscription");
            return_codes.push(SubscribeReturnCode::Failure);
            continue;
        }

        // TODO: granted max qos from config
        let granted_qos = subscribe_qos.to_owned();
        session.subscribe(filter.clone());
        global.subscribe(filter, session.client_id(), granted_qos);

        for msg in global.retain_table().get_matches(filter) {
            let mut packet = receive_outgoing_publish(session, granted_qos, msg.into());
            packet.set_retain(true);

            retain_packets.push(packet.into());
        }

        return_codes.push(granted_qos.into());
    }

    let mut queue: VecDeque<VariablePacket> = VecDeque::from(retain_packets);
    queue.push_front(SubackPacket::new(packet.packet_identifier(), return_codes).into());
    queue.into()
}

pub(super) fn handle_unsubscribe<Q>(
    session: &mut Session,
    packet: &UnsubscribePacket,
    global: Arc<GlobalState<Q>>,
) -> UnsubackPacket
where
    Q: Queue,
{
    log::debug!(
        r#"client#{} received a unsubscribe packet:
packet id : {}
   topics : {:?}"#,
        session.client_id(),
        packet.packet_identifier(),
        packet.subscribes(),
    );
    for filter in packet.subscribes() {
        global.unsubscribe(filter, session.client_id());
        session.unsubscribe(filter);
    }

    UnsubackPacket::new(packet.packet_identifier())
}
