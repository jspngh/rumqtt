use std::error::Error;
use std::time::Duration;

use rumqtt_bytes::{properties, Property, VarInt};
use rumqttc::{v5::AsyncClient, OptionBuilder, QoS};
use tokio::{task, time};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let options = OptionBuilder::new_tcp("localhost", 1884)
        .client_id("test-1")
        .keep_alive(Duration::from_secs(5))
        .finalize();

    let (client, mut eventloop) = AsyncClient::new_v5(options, 10);
    task::spawn(async move {
        requests(client).await;
        time::sleep(Duration::from_secs(3)).await;
    });

    while let Ok(event) = eventloop.poll().await {
        println!("{event:?}");
    }

    Ok(())
}

async fn requests(client: AsyncClient) {
    let props = properties![Property::SubscriptionIdentifier(VarInt::constant(1))];

    client
        .subscribe_with_properties("hello/world", QoS::AtMostOnce, props)
        .await
        .unwrap();

    let props = properties![Property::SubscriptionIdentifier(VarInt::constant(2))];

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
    // because we unsubscribed to hello/# and then
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
