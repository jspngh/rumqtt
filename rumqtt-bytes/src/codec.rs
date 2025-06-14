use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use super::{Error, Protocol};

/// A type that implements the [Encoder] and [Decoder] traits for MQTT packets.
#[derive(Debug, Clone)]
pub struct Codec<P: Protocol> {
    /// Maximum packet size allowed by client
    pub max_incoming_size: u32,
    /// Maximum packet size allowed by broker
    pub max_outgoing_size: u32,
    /// Phantom data to ensure the codec is generic over protocol
    protocol: std::marker::PhantomData<P>,
}

impl<P: Protocol> Codec<P> {
    /// Creates a new codec with specified maximum sizes
    pub fn new(max_incoming_size: u32, max_outgoing_size: u32) -> Self {
        Self {
            max_incoming_size,
            max_outgoing_size,
            protocol: std::marker::PhantomData,
        }
    }
}

impl<P: Protocol> Decoder for Codec<P> {
    type Item = P::Item;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match P::read(src, self.max_incoming_size) {
            Ok(packet) => Ok(Some(packet)),
            Err(Error::InsufficientBytes(b)) => {
                // Get more packets to construct the incomplete packet
                src.reserve(b);
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}

impl<P: Protocol<Item = Pkt>, Pkt> Encoder<Pkt> for Codec<P> {
    type Error = Error;

    fn encode(&mut self, item: Pkt, dst: &mut BytesMut) -> Result<(), Self::Error> {
        P::write(item, dst, self.max_outgoing_size)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use tokio_util::codec::Encoder;

    use super::Codec;
    use crate::{Error, Packet, Publish, QoS, V5};

    #[test]
    fn outgoing_max_packet_size_check() {
        let mut buf = BytesMut::new();
        let mut codec = Codec::<V5>::new(100, 200);

        let mut small_publish = Publish::new("hello/world", QoS::AtLeastOnce, vec![1; 100]);
        small_publish.pkid = 1;
        codec
            .encode(Packet::Publish(small_publish), &mut buf)
            .unwrap();

        let large_publish = Publish::new("hello/world", QoS::AtLeastOnce, vec![1; 265]);
        match codec.encode(Packet::Publish(large_publish), &mut buf) {
            Err(Error::OutgoingPacketTooLarge {
                pkt_size: 282,
                max: 200,
            }) => {}
            _ => unreachable!(),
        }
    }
}
