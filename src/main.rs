use chrono::{Datelike, FixedOffset, Timelike, Utc};
use dotenv::dotenv;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use std::env::var;
use std::time::Duration;
use thirtyfour::prelude::*;
use thirtyfour::{By, WebDriver};
use tokio;
use tokio::sync::broadcast;
use tokio::time::sleep;

const TOPIC_CAM_0: &str = "aidot/get/cam0";
const TOPIC_CAM_1: &str = "aidot/get/cam1";

const PATH_CAM_0: &str = "/home/lucas/home-assistant/data/cam0/";
const PATH_CAM_1: &str = "/home/lucas/home-assistant/data/cam1/";

#[tokio::main]
async fn main() {
    dotenv().ok();
    let mut mqttoptions = MqttOptions::new("test-client2", "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    // Create the client and event loop
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    if let Err(e) = client.subscribe(TOPIC_CAM_0, QoS::AtMostOnce).await {
        eprintln!("Failed to subscribe: {:?}", e);
        return;
    }

    if let Err(e) = client.subscribe(TOPIC_CAM_1, QoS::AtMostOnce).await {
        eprintln!("Failed to subscribe: {:?}", e);
        return;
    }

    let user = var("AIDOT_USER").expect("Missing AIDOT_USER in environment");
    let pass = var("AIDOT_PASSWORD").expect("Missing AIDOT_PASSWORD in environment");
    let url_cam_0 = var("URL_CAM_0").expect("Missing URL_CAM_0 in environment");
    let url_cam_1 = var("URL_CAM_1").expect("Missing URL_CAM_1 in environment");

    let mut caps = DesiredCapabilities::chrome();

    /* chrome args */
    // caps.add_chrome_arg("--headless").unwrap();
    // caps.add_chrome_arg("--disable-setuid-sandbox").unwrap();
    caps.add_chrome_arg("--use-fake-ui-for-media-stream").unwrap();
    caps.add_chrome_arg("--use-fake-device-for-media-stream").unwrap();
    // caps.add_chrome_arg("--allow-file-access-from-files").unwrap();
    caps.add_chrome_arg("--allow-insecure-localhost").unwrap();
    //caps.add_chrome_arg("--no-sandbox").unwrap();
    // caps.add_chrome_arg("--disable-web-security").unwrap();
    caps.add_chrome_arg("--disable-features=IsolateOrigins,site-per-process").unwrap();

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

    let caps_clone = caps.clone();
    let user_clone = user.clone();
    let pass_clone = pass.clone();
    // task 0
    let mut rx_a = tx.subscribe();
    tokio::spawn(async move {
        println!("Spawn task 0");
        let driver = WebDriver::new("http://localhost:9515", caps_clone)
            .await
            .unwrap();

        driver_sign_in(&driver, &user_clone, &pass_clone).await;

        println!("✅ Login exitoso");
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Ir a la vista de la cámara
        // Suponiendo que `driver` ya está inicializado
        driver.goto(url_cam_0).await.unwrap();

        loop {
            if !rx_a.is_empty() {
                if let Ok((topic, payload)) = rx_a.recv().await {
                    if topic == TOPIC_CAM_0 {
                        println!("Task 0: {:?}", String::from_utf8_lossy(&payload));

                        take_picture(&driver, &PATH_CAM_0.to_string()).await;
                    }
                }
            }
            let mut i: i32 = 0;
            while wait_for_video(&driver).await.is_none() {
                sleep(Duration::from_secs(1)).await;
                i += 1;
                if i == 30 {
                    driver.refresh().await.unwrap();
                }
            }
        }
    });

    // task 1
    let mut rx_a = tx.subscribe();
    tokio::spawn(async move {
        println!("Spawn task 1");
        let driver = WebDriver::new("http://localhost:9515", caps).await.unwrap();

        driver_sign_in(&driver, &user, &pass).await;

        println!("✅ Login exitoso");
        tokio::time::sleep(Duration::from_secs(5)).await;

        driver.goto(url_cam_1).await.unwrap();

        loop {
            if !rx_a.is_empty() {
                if let Ok((topic, payload)) = rx_a.recv().await {
                    if topic == TOPIC_CAM_1 {
                        println!("Task 1: {:?}", String::from_utf8_lossy(&payload));

                        take_picture(&driver, &PATH_CAM_1.to_string()).await;
                    }
                }
            }
            let mut i: i32 = 0;
            while wait_for_video(&driver).await.is_none() {
                sleep(Duration::from_secs(1)).await;
                i += 1;
                if i == 30 {
                    driver.refresh().await.unwrap();
                }
            }
        }
    });

    loop {
        client
            .publish(TOPIC_CAM_0, QoS::AtMostOnce, false, "")
            .await
            .unwrap();
        client
            .publish(TOPIC_CAM_1, QoS::AtMostOnce, false, "")
            .await
            .unwrap();
        
        let offset = FixedOffset::west_opt(3 * 3600).expect("Offset inválido");
        let datetime = Utc::now().with_timezone(&offset);

        println!(
            "TICK {:04}{:02}{:02}{:02}{:02}{:02}",
            datetime.year(),
            datetime.month(),
            datetime.day(),
            datetime.hour(),
            datetime.minute(),
            datetime.second()
        );

        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

async fn driver_sign_in(driver: &WebDriver, user: &String, pass: &String) {
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
}

// Función para esperar a que el video tenga dimensiones válidas y devolver el elemento
async fn wait_for_video(driver: &WebDriver) -> Option<thirtyfour::WebElement> {
    let script = r#"
        let video = document.querySelector('video');
        return video && video.videoWidth > 0 && video.videoHeight > 0;
    "#;

    match driver.execute(script, vec![]).await {
        Ok(result) => {
            if result.json().as_bool().unwrap_or(false) {
                // Obtener el elemento <video> cuando esté listo
                let video_elem = driver.query(By::Css("video")).first().await.unwrap();
                return Some(video_elem);
            }
        }
        Err(e) => {
            println!("Error al ejecutar script: {:?}", e);
        }
    }

    None
}

async fn take_picture(driver: &WebDriver, path: &String) {
    let offset = FixedOffset::west_opt(3 * 3600).expect("Offset inválido");

    match &wait_for_video(&driver).await {
        Some(video_elem) => {
            println!("✅ Video cargado con dimensiones válidas");

            // Tomar captura de pantalla solo del elemento <video>
            println!("📸 Tomando captura del elemento <video>...");
            let screenshot = video_elem.screenshot_as_png().await.unwrap();
            let datetime = Utc::now().with_timezone(&offset);
            let filename = format!(
                "{}{:04}{:02}{:02}{:02}{:02}{:02}.png",
                path,
                datetime.year(),
                datetime.month(),
                datetime.day(),
                datetime.hour(),
                datetime.minute(),
                datetime.second(),
            );
            std::fs::write(format!("{}now.png",path), &screenshot).unwrap();
            std::fs::write(&filename, &screenshot).unwrap();
            println!("✅ Captura guardada como {}", filename);
        }
        None => {
            println!("❌ No se pudo cargar el video con dimensiones válidas");
            driver.refresh().await.unwrap();
            while wait_for_video(&driver).await.is_none() {
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
