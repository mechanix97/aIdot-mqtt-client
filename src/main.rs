use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use std::env::var;
use std::time::Duration;
use thirtyfour::prelude::*;
use thirtyfour::{By, WebDriver};
use tokio;
use dotenv::dotenv;
use tokio::time::sleep;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let mut mqttoptions = MqttOptions::new("test-client2", "192.168.100.2", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    // Create the client and event loop
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    if let Err(e) = client.subscribe("aidot/get/cam1", QoS::AtLeastOnce).await {
        eprintln!("Failed to subscribe: {:?}", e);
        return;
    }

    if let Err(e) = client.subscribe("aidot/get/cam2", QoS::AtLeastOnce).await {
        eprintln!("Failed to subscribe: {:?}", e);
        return;
    }

    let user = var("AIDOT_USER").expect("Missing AIDOT_USER in environment");
    let pass = var("AIDOT_PASSWORD").expect("Missing AIDOT_PASSWORD in environment");

    let mut caps = DesiredCapabilities::chrome();

    /* chrome args */
    caps.add_chrome_arg("--headless").unwrap();
    caps.add_chrome_arg("--no-sandbox").unwrap();
    caps.add_chrome_arg("--disable-setuid-sandbox").unwrap();
    caps.add_chrome_arg("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.0.0 Safari/537.36").unwrap();

    caps.add_chrome_arg("--use-fake-ui-for-media-stream")
        .unwrap();
    caps.add_chrome_arg("--use-fake-device-for-media-stream")
        .unwrap();
    caps.add_chrome_arg("--allow-file-access-from-files")
        .unwrap();
    caps.add_chrome_arg("--allow-insecure-localhost").unwrap();
    caps.add_chrome_arg("--no-sandbox").unwrap();
    caps.add_chrome_arg("--disable-web-security").unwrap();
    caps.add_chrome_arg("--disable-features=IsolateOrigins,site-per-process")
        .unwrap();

    let (tx, _) = broadcast::channel::<(String, Vec<u8>)>(32);

    // Task de recepción MQTT
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("Connected to broker!");
                }
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    let _ = tx_clone.send((p.topic.clone(), p.payload.to_vec()));
                }
                Ok(_) => {}
                Err(_e) => {}
            }
        }
    });

    // task 1
    let mut rx_a = tx.subscribe();
    tokio::spawn(async move {
        println!("Spawn task 1");
        let driver = WebDriver::new("http://localhost:9515", caps.clone()).await.unwrap();
        driver.goto("https://app.aidot.com/SignIn").await.unwrap();
        let current_url = driver.current_url().await.unwrap().to_string();
        if current_url.contains("/SignIn") {
            // Rellenar campos de login
            tokio::time::sleep(Duration::from_secs(3)).await;

            let username = driver
            .query(By::Css("input[placeholder='User Name']"))
            .first()
            .await
            .unwrap();
            username.send_keys(&user).await.unwrap();

            let password = driver
                .query(By::Css("input[placeholder='Password']"))
                .first()
                .await
                .unwrap();
            password.send_keys(&pass).await.unwrap();
            tokio::time::sleep(Duration::from_millis(500)).await;

            let submit_btn = driver
                .query(By::Css("button[type='button'].MuiButton-root"))
                .first()
                .await
                .unwrap();
            submit_btn.click().await.unwrap();
        }
        println!("✅ Login exitoso");
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Ir a la vista de la cámara
        // Suponiendo que `driver` ya está inicializado
        driver
            .goto("https://app.aidot.com/live/b228e9b618e541479f9b4a636bdf52e0")
            .await
            .unwrap();

        // Esperar a que el video tenga dimensiones válidas
        println!("⏳ Esperando a que el video cargue...");
        let video_element: Option<WebElement> = wait_for_video(&driver).await.unwrap();

        while let Ok((topic, payload)) = rx_a.recv().await {
            if topic == "aidot/get/cam1" {
                println!("Task A: {:?}", String::from_utf8_lossy(&payload));
                match &video_element {
                    Some(video_elem) => {
                        println!("✅ Video cargado con dimensiones válidas");

                        // Tomar captura de pantalla solo del elemento <video>
                        println!("📸 Tomando captura del elemento <video>...");
                        let screenshot = video_elem.screenshot_as_png().await.unwrap();
                        std::fs::write("/home/lucas/home-assistant/data/captura_video.png", screenshot)
                            .unwrap();
                        println!("✅ Captura guardada como captura_video.png");
                    }
                    None => {
                        println!("❌ No se pudo cargar el video con dimensiones válidas");
                    }
                }
            }
        }
    });

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }


    // loop {
    //     match eventloop.poll().await {
    //         Ok(Event::Incoming(Incoming::ConnAck(_))) => {
    //             println!("Connected to broker!");
    //         }
    //         Ok(Event::Incoming(Incoming::Publish(_p))) => {
    //             let driver = WebDriver::new("http://localhost:9515", caps.clone()).await.unwrap();
                
    //             // Ir a la página de login
    //             driver.goto("https://app.aidot.com").await.unwrap();

    //             let current_url = driver.current_url().await.unwrap().to_string();
    //             if current_url.contains("/SignIn") {
    //                 // Rellenar campos de login
    //                 let username = driver
    //                     .query(By::Css("input[placeholder='User Name']"))
    //                     .first()
    //                     .await
    //                     .unwrap();
    //                 username.send_keys(&user).await.unwrap();

    //                 let password = driver
    //                     .query(By::Css("input[placeholder='Password']"))
    //                     .first()
    //                     .await
    //                     .unwrap();
    //                 password.send_keys(&pass).await.unwrap();

    //                 let submit_btn = driver
    //                     .query(By::Css("button[type='button'].MuiButton-root"))
    //                     .first()
    //                     .await
    //                     .unwrap();
    //                 submit_btn.click().await.unwrap();
    //             }

    //             // Esperar que cambie la URL (hasta 20s) o esperar algún elemento post-login
    //             let mut retries = 20;
    //             loop {
    //                 let current_url = driver.current_url().await.unwrap().to_string();
    //                 if !current_url.contains("/SignIn") {
    //                     break;
    //                 }
    //                 if retries == 0 {
    //                     panic!("Timeout esperando redirección después del login.");
    //                 }
    //                 retries -= 1;
    //                 tokio::time::sleep(Duration::from_secs(1)).await;
    //             }

    //             println!("✅ Login exitoso");

    //             // Ir a la vista de la cámara
    //             // Suponiendo que `driver` ya está inicializado
    //             driver
    //                 .goto("https://app.aidot.com/live/b228e9b618e541479f9b4a636bdf52e0")
    //                 .await
    //                 .unwrap();

    //             // Esperar a que el video tenga dimensiones válidas
    //             println!("⏳ Esperando a que el video cargue...");
    //             let video_element = wait_for_video(&driver).await.unwrap();

    //             match video_element {
    //                 Some(video_elem) => {
    //                     println!("✅ Video cargado con dimensiones válidas");

    //                     // Tomar captura de pantalla solo del elemento <video>
    //                     println!("📸 Tomando captura del elemento <video>...");
    //                     let screenshot = video_elem.screenshot_as_png().await.unwrap();
    //                     std::fs::write("/home/lucas/home-assistant/data/captura_video.png", screenshot)
    //                         .unwrap();
    //                     println!("✅ Captura guardada como captura_video.png");
    //                 }
    //                 None => {
    //                     println!("❌ No se pudo cargar el video con dimensiones válidas");
    //                 }
    //             }

    //             // Cerrar el navegador
    //             //driver.quit().await.unwrap();
    //         }
    //         Ok(_) => {}
    //         Err(e) => {
    //             eprintln!("Easdrror: {:?}", e);
    //             time::sleep(Duration::from_secs(5)).await;
    //             // Add reconnection logic if needed
    //         }
    //     }
    // }
}

// Función para esperar a que el video tenga dimensiones válidas y devolver el elemento
async fn wait_for_video(driver: &WebDriver) -> WebDriverResult<Option<thirtyfour::WebElement>> {
    let script = r#"
     let video = document.querySelector('video');
     return video && video.videoWidth > 0 && video.videoHeight > 0;
 "#;

    for i in 0..100 {
        println!("Intento {}: Verificando video...", i);
        match driver.execute(script, vec![]).await {
            Ok(result) => {
                if result.json().as_bool().unwrap_or(false) {
                    // Obtener el elemento <video> cuando esté listo
                    let video_elem = driver.query(By::Css("video")).first().await.unwrap();
                    return Ok(Some(video_elem));
                }
            }
            Err(e) => {
                println!("Error al ejecutar script: {:?}", e);
            }
        }
        sleep(Duration::from_secs(1)).await;
    }

    Ok(None)
}
