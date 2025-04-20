use rumqttc::{MqttOptions, AsyncClient, QoS, Event, Incoming};
use std::time::Duration;
use tokio::time;

async fn mqtt() {
    let mut mqttoptions = MqttOptions::new("test-client2", "192.168.100.2", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    // Create the client and event loop
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    if let Err(e) = client.subscribe("test/topic", QoS::AtLeastOnce).await {
        eprintln!("Failed to subscribe: {:?}", e);
        return;
    }

    if let Err(e) = client
        .publish("test/topic", QoS::AtLeastOnce, false, "Hola desde Rust")
        .await
    {
        eprintln!("Failed to publish: {:?}", e);
        return;
    }

    println!("Esperando mensajes...");

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                println!("Connected to broker!");
            }
            Ok(Event::Incoming(Incoming::Publish(p))) => {
                let payload = if let Ok(s) = String::from_utf8(p.payload.to_vec()) {
                    s
                } else {
                    format!("(non-UTF-8 data: {} bytes)", p.payload.len())
                };
                println!(">> Mensaje recibido: {} = {}", p.topic, payload);
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Easdrror: {:?}", e);
                time::sleep(Duration::from_secs(5)).await;
                // Add reconnection logic if needed
            }
        }
    }
}
