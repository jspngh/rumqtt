use std::error::Error;
use std::time::Duration;

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
        requests(&client).await;
        time::sleep(Duration::from_secs(15)).await;
    });

    loop {
        let event = eventloop.poll().await;
        match &event {
            Ok(v) => {
                println!("Event = {v:?}");
            }
            Err(e) => {
                println!("Error = {e:?}");
                return Ok(());
            }
        }
    }
}

async fn requests(client: &AsyncClient) {
    client
        .subscribe("hello/world", QoS::AtMostOnce)
        .await
        .unwrap();

    for i in 1..=10 {
        client
            .publish("hello/world", QoS::ExactlyOnce, false, vec![1; i])
            .await
            .unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }
}
