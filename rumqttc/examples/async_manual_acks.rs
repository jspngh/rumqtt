use std::error::Error;
use std::time::Duration;

use rumqttc::{AsyncClient, Event, EventLoop, Incoming, OptionBuilder, QoS};
use tokio::{task, time};

fn create_conn() -> (AsyncClient, EventLoop) {
    let options = OptionBuilder::new_tcp("localhost", 1883)
        .client_id("test-1")
        .keep_alive(Duration::from_secs(5))
        .manual_acks(true)
        .clean_start(false)
        .finalize();

    AsyncClient::new(options, 10)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    // create mqtt connection with clean_session = false and manual_acks = true
    let (client, mut eventloop) = create_conn();

    // subscribe example topic
    client
        .subscribe("hello/world", QoS::AtLeastOnce)
        .await
        .unwrap();

    task::spawn(async move {
        // send some messages to example topic and disconnect
        requests(&client).await;
        client.disconnect().await.unwrap();
    });

    loop {
        // get subscribed messages without acking
        let event = eventloop.poll().await;
        match &event {
            Ok(notif) => {
                println!("Event = {notif:?}");
            }
            Err(error) => {
                println!("Error = {error:?}");
                break;
            }
        }
    }

    // create new broker connection but do not start a clean session
    let (client, mut eventloop) = create_conn();

    loop {
        // previously published messages should be republished after reconnection.
        let event = eventloop.poll().await;
        match &event {
            Ok(notif) => {
                println!("Event = {notif:?}");
            }
            Err(error) => {
                println!("Error = {error:?}");
                return Ok(());
            }
        }

        if let Ok(Event::Incoming(Incoming::Publish(publish))) = event {
            // this time we will ack incoming publishes.
            // Its important not to block eventloop as this can cause deadlock.
            let c = client.clone();
            tokio::spawn(async move {
                c.ack(&publish).await.unwrap();
            });
        }
    }
}

async fn requests(client: &AsyncClient) {
    for i in 1..=10 {
        client
            .publish("hello/world", QoS::AtLeastOnce, false, vec![1; i])
            .await
            .unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }
}
