use super::*;

pub(super) fn parse_message_status(status: &str) -> Result<MessageStatus, TwilioError> {
    match status.to_lowercase().as_str() {
        "queued" => Ok(MessageStatus::Queued),
        "sending" => Ok(MessageStatus::Sending),
        "sent" => Ok(MessageStatus::Sent),
        "delivered" => Ok(MessageStatus::Delivered),
        "undelivered" => Ok(MessageStatus::Undelivered),
        "failed" => Ok(MessageStatus::Failed),
        "receiving" => Ok(MessageStatus::Receiving),
        "received" => Ok(MessageStatus::Received),
        "accepted" => Ok(MessageStatus::Accepted),
        "scheduled" => Ok(MessageStatus::Scheduled),
        "read" => Ok(MessageStatus::Read),
        "partially_delivered" => Ok(MessageStatus::PartiallyDelivered),
        "canceled" => Ok(MessageStatus::Canceled),
        _ => Err(TwilioError::ParseError(format!("Unknown status: {}", status))),
    }
}
pub(super) fn parse_direction(direction: &str) -> MessageDirection {
    match direction.to_lowercase().as_str() {
        "inbound" => MessageDirection::Inbound,
        "outbound-api" => MessageDirection::OutboundApi,
        "outbound-call" => MessageDirection::OutboundCall,
        "outbound-reply" => MessageDirection::OutboundReply,
        _ => MessageDirection::OutboundApi,
    }
}
pub(super) fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
