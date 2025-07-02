use std::thread;
use std::time::Duration;

use rumqtt_bytes::{LastWill, V5};
use rumqttc::{Client, ConnectionError, OptionsBuilder, QoS};

fn main() {
    pretty_env_logger::init();

    let will = LastWill::new("hello/world", "good bye", QoS::AtMostOnce, false);
    let options = OptionsBuilder::new_tcp("localhost", 1884)
        .client_id("test-1")
        .keep_alive(Duration::from_secs(5))
        .last_will(will)
        .finalize();

    let (client, mut connection) = Client::new_v5(options, 10);
    thread::spawn(move || publish(client));

    for (i, notification) in connection.iter().enumerate() {
        match notification {
            Err(ConnectionError::Io(error))
                if error.kind() == std::io::ErrorKind::ConnectionRefused =>
            {
                println!("Failed to connect to the server. Make sure correct client is configured properly!\nError: {error:?}");
                return;
            }
            _ => {}
        }
        println!("{i}. Notification = {notification:?}");
    }

    println!("Done with the stream!!");
}

fn publish(client: Client<V5>) {
    client.subscribe("hello/+/world", QoS::AtMostOnce).unwrap();
    for i in 0..10_usize {
        let payload = vec![1; i];
        let topic = format!("hello/{i}/world");
        let qos = QoS::AtLeastOnce;

        let _ = client.publish(topic, qos, true, payload);
    }

    thread::sleep(Duration::from_secs(1));
}
