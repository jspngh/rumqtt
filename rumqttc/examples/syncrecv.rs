use std::thread;
use std::time::Duration;

use rumqtt_bytes::LastWill;
use rumqttc::{Client, OptionBuilder, QoS};

fn main() {
    pretty_env_logger::init();

    let will = LastWill::new("hello/world", "good bye", QoS::AtMostOnce, false);
    let options = OptionBuilder::new_tcp("localhost", 1883)
        .client_id("test-1")
        .keep_alive(Duration::from_secs(5))
        .last_will(will)
        .finalize();

    let (client, mut connection) = Client::new(options, 10);
    thread::spawn(move || publish(client));

    if let Ok(notification) = connection.recv() {
        println!("Notification = {notification:?}");
    }

    if let Ok(notification) = connection.try_recv() {
        println!("Notification = {notification:?}");
    }

    if let Ok(notification) = connection.recv_timeout(Duration::from_secs(10)) {
        println!("Notification = {notification:?}");
    }
}

fn publish(client: Client) {
    client.subscribe("hello/+/world", QoS::AtMostOnce).unwrap();
    for i in 0..3 {
        let payload = vec![1; i];
        let topic = format!("hello/{i}/world");
        let qos = QoS::AtLeastOnce;

        client.publish(topic, qos, true, payload).unwrap();
    }

    thread::sleep(Duration::from_secs(1));
}
