use std::error::Error;
use std::time::Duration;

use rumqtt_bytes::v5::SubscribeProperties;
use rumqtt_bytes::{QoS, VarInt};
use rumqttc::v5::{AsyncClient, MqttOptions};
use tokio::{task, time};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let mut mqttoptions = MqttOptions::new("test-1", "localhost", 1884);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    task::spawn(async move {
        requests(client).await;
        time::sleep(Duration::from_secs(3)).await;
    });

    while let Ok(event) = eventloop.poll().await {
        println!("{:?}", event);
    }

    Ok(())
}

async fn requests(client: AsyncClient) {
    let props = SubscribeProperties {
        subscription_id: Some(VarInt::constant(1)), // TODO
        user_properties: vec![],
    };

    client
        .subscribe_with_properties("hello/world", QoS::AtMostOnce, props)
        .await
        .unwrap();

    let props = SubscribeProperties {
        subscription_id: Some(VarInt::new(2).unwrap()), // TODO
        user_properties: vec![],
    };

    client
        .subscribe_with_properties("hello/#", QoS::AtMostOnce, props)
        .await
        .unwrap();

    time::sleep(Duration::from_millis(500)).await;
    // we will receive two publishes
    // one due to hello/world and other due to hello/#
    // both will have respective subscription ids
    client
        .publish(
            "hello/world",
            QoS::AtMostOnce,
            false,
            "both having subscription IDs!",
        )
        .await
        .unwrap();

    time::sleep(Duration::from_millis(500)).await;
    client.unsubscribe("hello/#").await.unwrap();
    client.subscribe("hello/#", QoS::AtMostOnce).await.unwrap();
    time::sleep(Duration::from_millis(500)).await;

    // we will receive two publishes
    // but only one will have subscription ID
    // cuz we unsubscribed to hello/# and then
    // subscribed without properties!
    client
        .publish(
            "hello/world",
            QoS::AtMostOnce,
            false,
            "Only one with subscription ID!",
        )
        .await
        .unwrap();
}
