use std::error::Error;
use std::time::Duration;

use rumqtt_bytes::PublishProperties;
use rumqttc::{v5::AsyncClient, OptionBuilder, QoS};
use tokio::{task, time};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let options = OptionBuilder::new_tcp("localhost", 1884)
        .client_id("test-1")
        .keep_alive(Duration::from_secs(5))
        .topic_alias_max(10)
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
    client
        .subscribe("hello/world", QoS::AtMostOnce)
        .await
        .unwrap();

    client
        .subscribe("bye/world", QoS::AtMostOnce)
        .await
        .unwrap();

    let props = PublishProperties {
        topic_alias: Some(28),
        ..Default::default()
    };

    client
        .publish_with_properties(
            "hello/world",
            QoS::AtMostOnce,
            false,
            vec![3; 3],
            props.clone(),
        )
        .await
        .unwrap();

    let mut other_props = props.clone();
    other_props.topic_alias = Some(51);

    client
        .publish_with_properties(
            "bye/world",
            QoS::AtMostOnce,
            false,
            vec![3; 3],
            other_props.clone(),
        )
        .await
        .unwrap();

    for i in 1..=10 {
        // alternately choose the properties
        let properties = if i % 2 == 0 {
            other_props.clone()
        } else {
            props.clone()
        };

        // no need to specify topic as we are using topic alias
        client
            .publish_with_properties("", QoS::AtMostOnce, false, vec![1; i], properties)
            .await
            .unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }

    time::sleep(Duration::from_secs(120)).await;
}
