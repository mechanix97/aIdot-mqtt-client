use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use std::env::var;
use std::time::Duration;
use thirtyfour::prelude::*;
use thirtyfour::{By, WebDriver};
use tokio;
use tokio::time;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let mut mqttoptions = MqttOptions::new("test-client2", "192.168.100.2", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    // Create the client and event loop
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    if let Err(e) = client.subscribe("aidot/get/cam1", QoS::AtLeastOnce).await {
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


    let user = var("AIDOT_USER").expect("Missing ENCRYPTION_KEY in environment");
    let pass = var("AIDOT_PASSWORD").expect("Missing ENCRYPTION_KEY in environment");

    // Inicializar WebDriver (Chrome)
    let mut caps = DesiredCapabilities::chrome();

    caps.add_chrome_arg("--headless").unwrap();
    caps.add_chrome_arg("--no-sandbox").unwrap();
    caps.add_chrome_arg("--disable-setuid-sandbox").unwrap();

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

    // caps.add_chrome_option("profile.default_content_setting_values.media_stream_mic", json!(1)).unwrap();

    let web_driver = WebDriver::new("http://localhost:9515", caps).await.unwrap();
    

    println!("Esperando mensajes...");

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                println!("Connected to broker!");
            }
            Ok(Event::Incoming(Incoming::Publish(_p))) => {
                let driver = web_driver.clone();
                // Ir a la página de login
                driver.goto("https://app.aidot.com").await.unwrap();

                let current_url = driver.current_url().await.unwrap().to_string();
                if current_url.contains("/SignIn") {
                    // Rellenar campos de login
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

                    let submit_btn = driver
                        .query(By::Css("button[type='button'].MuiButton-root"))
                        .first()
                        .await
                        .unwrap();
                    submit_btn.click().await.unwrap();
                }

                // Esperar que cambie la URL (hasta 20s) o esperar algún elemento post-login
                let mut retries = 20;
                loop {
                    let current_url = driver.current_url().await.unwrap().to_string();
                    if !current_url.contains("/SignIn") {
                        break;
                    }
                    if retries == 0 {
                        panic!("Timeout esperando redirección después del login.");
                    }
                    retries -= 1;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }

                println!("✅ Login exitoso");

                // Ir a la vista de la cámara
                // Suponiendo que `driver` ya está inicializado
                driver
                    .goto("https://app.aidot.com/live/b228e9b618e541479f9b4a636bdf52e0")
                    .await
                    .unwrap();

                // Esperar a que el video tenga dimensiones válidas
                println!("⏳ Esperando a que el video cargue...");
                let video_element = wait_for_video(&driver).await.unwrap();

                match video_element {
                    Some(video_elem) => {
                        println!("✅ Video cargado con dimensiones válidas");

                        // Tomar captura de pantalla solo del elemento <video>
                        println!("📸 Tomando captura del elemento <video>...");
                        let screenshot = video_elem.screenshot_as_png().await.unwrap();
                        std::fs::write("~/home-assistant/data/captura_video.png", screenshot)
                            .unwrap();
                        println!("✅ Captura guardada como captura_video.png");
                    }
                    None => {
                        println!("❌ No se pudo cargar el video con dimensiones válidas");
                    }
                }

                // Cerrar el navegador
                driver.quit().await.unwrap();
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

// Función para esperar a que el video tenga dimensiones válidas y devolver el elemento
async fn wait_for_video(driver: &WebDriver) -> WebDriverResult<Option<thirtyfour::WebElement>> {
    let script = r#"
     let video = document.querySelector('video');
     return video && video.videoWidth > 0 && video.videoHeight > 0;
 "#;

    for i in 0..30 {
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
