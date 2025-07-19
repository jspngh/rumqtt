use crate::{reason, Error, Properties};

pub(crate) mod v4;
pub(crate) mod v5;

/// Disconnect notification
///
/// The final MQTT packet sent from the client or the server.
/// It indicates the reason why the network connection is being closed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Disconnect {
    /// Disconnect Reason Code
    pub reason_code: DisconnectReasonCode,
    /// Disconnect Properties
    pub properties: Properties,
}

impl Disconnect {
    pub fn new() -> Self {
        Self {
            reason_code: DisconnectReasonCode::NormalDisconnection,
            properties: Properties::new(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DisconnectReasonCode {
    #[default]
    /// Close the connection normally. Do not send the Will Message.
    NormalDisconnection = reason::NORMAL_DISCONNECTION,
    /// The Client wishes to disconnect but requires that the Server also publishes its Will Message.
    DisconnectWithWillMessage = reason::DISCONNECT_WITH_WILL_MESSAGE,
    /// The Connection is closed but the sender either does not wish to reveal the reason, or none of the other Reason Codes apply.
    UnspecifiedError = reason::UNSPECIFIED_ERROR,
    /// The received packet does not conform to this specification.
    MalformedPacket = reason::MALFORMED_PACKET,
    /// An unexpected or out of order packet was received.
    ProtocolError = reason::PROTOCOL_ERROR,
    /// The packet received is valid but cannot be processed by this implementation.
    ImplementationSpecificError = reason::IMPLEMENTATION_SPECIFIC_ERROR,
    /// The request is not authorized.
    NotAuthorized = reason::NOT_AUTHORIZED,
    /// The Server is busy and cannot continue processing requests from this Client.
    ServerBusy = reason::SERVER_BUSY,
    /// The Server is shutting down.
    ServerShuttingDown = reason::SERVER_SHUTTING_DOWN,
    /// The Connection is closed because no packet has been received for 1.5 times the Keepalive time.
    KeepAliveTimeout = reason::KEEP_ALIVE_TIMEOUT,
    /// Another Connection using the same ClientID has connected causing this Connection to be closed.
    SessionTakenOver = reason::SESSION_TAKEN_OVER,
    /// The Topic Filter is correctly formed, but is not accepted by this Sever.
    TopicFilterInvalid = reason::TOPIC_FILTER_INVALID,
    /// The Topic Name is correctly formed, but is not accepted by this Client or Server.
    TopicNameInvalid = reason::TOPIC_NAME_INVALID,
    /// The Client or Server has received more than Receive Maximum publication for which it has not sent PUBACK or PUBCOMP.
    ReceiveMaximumExceeded = reason::RECEIVE_MAXIMUM_EXCEEDED,
    /// The Client or Server has received a PUBLISH packet containing a Topic Alias which is greater than
    /// the Maximum Topic Alias it sent in the CONNECT or CONNACK packet.
    TopicAliasInvalid = reason::TOPIC_ALIAS_INVALID,
    /// The packet size is greater than Maximum Packet Size for this Client or Server.
    PacketTooLarge = reason::PACKET_TOO_LARGE,
    /// The received data rate is too high.
    MessageRateTooHigh = reason::MESSAGE_RATE_TOO_HIGH,
    /// An implementation or administrative imposed limit has been exceeded.
    QuotaExceeded = reason::QUOTA_EXCEEDED,
    /// The Connection is closed due to an administrative action.
    AdministrativeAction = reason::ADMINISTRATIVE_ACTION,
    /// The payload format does not match the one specified by the Payload Format Indicator.
    PayloadFormatInvalid = reason::PAYLOAD_FORMAT_INVALID,
    /// The Server has does not support retained messages.
    RetainNotSupported = reason::RETAIN_NOT_SUPPORTED,
    /// The Client specified a QoS greater than the QoS specified in a Maximum QoS in the CONNACK.
    QoSNotSupported = reason::QOS_NOT_SUPPORTED,
    /// The Client should temporarily change its Server.
    UseAnotherServer = reason::USE_ANOTHER_SERVER,
    /// The Server is moved and the Client should permanently change its server location.
    ServerMoved = reason::SERVER_MOVED,
    /// The Server does not support Shared Subscriptions.
    SharedSubscriptionNotSupported = reason::SHARED_SUBSCRIPTIONS_NOT_SUPPORTED,
    /// This connection is closed because the connection rate is too high.
    ConnectionRateExceeded = reason::CONNECTION_RATE_EXCEEDED,
    /// The maximum connection time authorized for this connection has been exceeded.
    MaximumConnectTime = reason::MAXIMUM_CONNECT_TIME,
    /// The Server does not support Subscription Identifiers; the subscription is not accepted.
    SubscriptionIdentifiersNotSupported = reason::SUBSCRIPTION_IDENTIFIERS_NOT_SUPPORTED,
    /// The Server does not support Wildcard subscription; the subscription is not accepted.
    WildcardSubscriptionsNotSupported = reason::WILDCARD_SUBSCRIPTIONS_NOT_SUPPORTED,
}

impl TryFrom<u8> for DisconnectReasonCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let rc = match value {
            reason::NORMAL_DISCONNECTION => Self::NormalDisconnection,
            reason::DISCONNECT_WITH_WILL_MESSAGE => Self::DisconnectWithWillMessage,
            reason::UNSPECIFIED_ERROR => Self::UnspecifiedError,
            reason::MALFORMED_PACKET => Self::MalformedPacket,
            reason::PROTOCOL_ERROR => Self::ProtocolError,
            reason::IMPLEMENTATION_SPECIFIC_ERROR => Self::ImplementationSpecificError,
            reason::NOT_AUTHORIZED => Self::NotAuthorized,
            reason::SERVER_BUSY => Self::ServerBusy,
            reason::SERVER_SHUTTING_DOWN => Self::ServerShuttingDown,
            reason::KEEP_ALIVE_TIMEOUT => Self::KeepAliveTimeout,
            reason::SESSION_TAKEN_OVER => Self::SessionTakenOver,
            reason::TOPIC_FILTER_INVALID => Self::TopicFilterInvalid,
            reason::TOPIC_NAME_INVALID => Self::TopicNameInvalid,
            reason::RECEIVE_MAXIMUM_EXCEEDED => Self::ReceiveMaximumExceeded,
            reason::TOPIC_ALIAS_INVALID => Self::TopicAliasInvalid,
            reason::PACKET_TOO_LARGE => Self::PacketTooLarge,
            reason::MESSAGE_RATE_TOO_HIGH => Self::MessageRateTooHigh,
            reason::QUOTA_EXCEEDED => Self::QuotaExceeded,
            reason::ADMINISTRATIVE_ACTION => Self::AdministrativeAction,
            reason::PAYLOAD_FORMAT_INVALID => Self::PayloadFormatInvalid,
            reason::RETAIN_NOT_SUPPORTED => Self::RetainNotSupported,
            reason::QOS_NOT_SUPPORTED => Self::QoSNotSupported,
            reason::USE_ANOTHER_SERVER => Self::UseAnotherServer,
            reason::SERVER_MOVED => Self::ServerMoved,
            reason::SHARED_SUBSCRIPTIONS_NOT_SUPPORTED => Self::SharedSubscriptionNotSupported,
            reason::CONNECTION_RATE_EXCEEDED => Self::ConnectionRateExceeded,
            reason::MAXIMUM_CONNECT_TIME => Self::MaximumConnectTime,
            reason::SUBSCRIPTION_IDENTIFIERS_NOT_SUPPORTED => {
                Self::SubscriptionIdentifiersNotSupported
            }
            reason::WILDCARD_SUBSCRIPTIONS_NOT_SUPPORTED => Self::WildcardSubscriptionsNotSupported,
            other => return Err(Error::InvalidConnectReturnCode(other)),
        };

        Ok(rc)
    }
}
