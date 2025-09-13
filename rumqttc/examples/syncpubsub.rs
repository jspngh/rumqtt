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

    for (i, notification) in connection.iter().enumerate() {
        match notification {
            Ok(notif) => {
                println!("{i}. Notification = {notif:?}");
            }
            Err(error) => {
                println!("{i}. Notification = {error:?}");
                return;
            }
        }
    }

    println!("Done with the stream!!");
}

fn publish(client: Client) {
    thread::sleep(Duration::from_secs(1));
    client.subscribe("hello/+/world", QoS::AtMostOnce).unwrap();
    for i in 0..10_usize {
        let payload = vec![1; i];
        let topic = format!("hello/{i}/world");
        let qos = QoS::AtLeastOnce;

        client.publish(topic, qos, true, payload).unwrap();
    }

    thread::sleep(Duration::from_secs(1));
}
