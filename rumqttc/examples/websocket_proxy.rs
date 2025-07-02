use std::{error::Error, time::Duration};

use rumqttc::{v5::AsyncClient, OptionBuilder, Proxy, ProxyAuth, ProxyType, QoS};
use tokio::{task, time};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    // port parameter is ignored when scheme is websocket
    let options = OptionBuilder::new_ws("ws://broker.mqttdashboard.com:8000/mqtt", 8000)
        .client_id("clientId-aSziq39Bp3")
        .keep_alive(Duration::from_secs(60))
        // Presumes that there is a proxy server already set up listening on 127.0.0.1:8100
        .proxy(Proxy {
            ty: ProxyType::Http,
            auth: ProxyAuth::None,
            addr: "127.0.0.1".into(),
            port: 8100,
        })
        .finalize();

    let (client, mut eventloop) = AsyncClient::new_v5(options, 10);
    task::spawn(async move {
        requests(client).await;
        time::sleep(Duration::from_secs(3)).await;
    });

    loop {
        let event = eventloop.poll().await;
        match event {
            Ok(notif) => {
                println!("Event = {notif:?}");
            }
            Err(err) => {
                println!("Error = {err:?}");
                return Ok(());
            }
        }
    }
}

async fn requests(client: AsyncClient) {
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

    time::sleep(Duration::from_secs(120)).await;
}
