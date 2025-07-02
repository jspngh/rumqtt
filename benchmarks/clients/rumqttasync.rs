use rumqttc::{AsyncClient, Event, Incoming, OptionBuilder, QoS};

use std::error::Error;
use std::time::{Duration, Instant};

use tokio::task;
use tokio::time;

mod common;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    // pretty_env_logger::init();
    let guard = pprof::ProfilerGuard::new(100).unwrap();
    start("rumqtt-async", 100, 1_000_000).await.unwrap();
    common::profile("bench.pb", guard);
}

pub async fn start(id: &str, payload_size: usize, count: usize) -> Result<(), Box<dyn Error>> {
    let options = OptionBuilder::new_tcp("localhost", 1883)
        .client_id(id)
        .keep_alive(Duration::from_secs(20))
        .receive_maximum(100)
        .finalize();

    let (client, mut eventloop) = AsyncClient::new(options, 10);
    task::spawn(async move {
        for _i in 0..count {
            let payload = vec![0; payload_size];
            let qos = QoS::AtLeastOnce;
            client
                .publish("hello/benchmarks/world", qos, false, payload)
                .await
                .unwrap();
        }

        time::sleep(Duration::from_secs(10)).await;
    });

    let mut acks_count = 0;
    let start = Instant::now();
    loop {
        if let Event::Incoming(Incoming::PubAck(_)) = eventloop.poll().await? {
            acks_count += 1;
            if acks_count == count {
                break;
            }
        }
    }

    let elapsed_ms = start.elapsed().as_millis();
    let throughput = acks_count / elapsed_ms as usize;
    let throughput = throughput * 1000;

    let print = common::Print {
        id: id.to_owned(),
        messages: count,
        payload_size,
        throughput,
    };

    println!("{}", serde_json::to_string_pretty(&print).unwrap());
    Ok(())
}
