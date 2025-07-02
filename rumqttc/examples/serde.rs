use std::convert::TryFrom;
use std::thread;
use std::time::{Duration, SystemTime};

use bincode::ErrorKind;
use bytes::Bytes;
use rumqttc::{Client, Event, Incoming, OptionBuilder, QoS};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    i: usize,
    time: SystemTime,
}

impl From<&Message> for Bytes {
    fn from(value: &Message) -> Self {
        Bytes::from(bincode::serialize(value).unwrap())
    }
}

impl From<Message> for Bytes {
    fn from(value: Message) -> Self {
        Bytes::from(bincode::serialize(&value).unwrap())
    }
}

impl TryFrom<&[u8]> for Message {
    type Error = Box<ErrorKind>;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        bincode::deserialize(value)
    }
}

fn main() {
    let options = OptionBuilder::new_tcp("localhost", 1883)
        .client_id("test-1")
        .finalize();

    let (client, mut connection) = Client::new(options, 10);
    client.subscribe("hello/rumqtt", QoS::AtMostOnce).unwrap();
    thread::spawn(move || {
        for i in 0..10 {
            let message = Message {
                i,
                time: SystemTime::now(),
            };

            client
                .publish("hello/rumqtt", QoS::AtLeastOnce, false, message)
                .unwrap();
            thread::sleep(Duration::from_millis(100));
        }
    });

    // Iterate to poll the eventloop for connection progress
    for notification in connection.iter().flatten() {
        if let Event::Incoming(Incoming::Publish(packet)) = notification {
            match Message::try_from(packet.payload.as_ref()) {
                Ok(message) => println!("Payload = {message:?}"),
                Err(error) => println!("Error = {error}"),
            }
        }
    }
}
