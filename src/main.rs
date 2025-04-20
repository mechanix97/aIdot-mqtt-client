use thirtyfour::prelude::*;
use tokio;
use std::time::Duration;
use thirtyfour::{By, WebDriver};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> WebDriverResult<()> {
    // Leer usuario y contraseña de variables de entorno
    let user = "";
    let pass = "";

    // Inicializar WebDriver (Chrome)
    let mut caps = DesiredCapabilities::chrome();

    caps.add_chrome_arg("--headless")?;
    caps.add_chrome_arg("--no-sandbox")?;
    caps.add_chrome_arg("--disable-setuid-sandbox")?;

    caps.add_chrome_arg("--use-fake-ui-for-media-stream")?;
    caps.add_chrome_arg("--use-fake-device-for-media-stream")?;
    caps.add_chrome_arg("--allow-file-access-from-files")?;
    caps.add_chrome_arg("--allow-insecure-localhost")?;
    caps.add_chrome_arg("--no-sandbox")?;
    caps.add_chrome_arg("--disable-web-security")?;
    caps.add_chrome_arg("--disable-features=IsolateOrigins,site-per-process")?;

    // caps.add_chrome_option("profile.default_content_setting_values.media_stream_mic", json!(1))?;

    let driver = WebDriver::new("http://localhost:9515", caps).await?;

    // Ir a la página de login
    driver.goto("https://app.aidot.com").await?;

    let current_url = driver.current_url().await?.to_string();
    if current_url.contains("/SignIn") {
        // Rellenar campos de login
    let username = driver.query(By::Css("input[placeholder='User Name']")).first().await?;
    username.send_keys(&user).await?;

    let password = driver.query(By::Css("input[placeholder='Password']")).first().await?;
    password.send_keys(&pass).await?;

    let submit_btn = driver.query(By::Css("button[type='button'].MuiButton-root")).first().await?;
    submit_btn.click().await?;

    } 
    
    // Esperar que cambie la URL (hasta 20s) o esperar algún elemento post-login
    let mut retries = 20;
    loop {
        let current_url = driver.current_url().await?.to_string();
        if ! current_url.contains("/SignIn") {
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
    .await?;

 // Esperar a que el video tenga dimensiones válidas
 println!("⏳ Esperando a que el video cargue...");
 let video_element = wait_for_video(&driver).await?;

 match video_element {
     Some(video_elem) => {
         println!("✅ Video cargado con dimensiones válidas");

         // Tomar captura de pantalla solo del elemento <video>
         println!("📸 Tomando captura del elemento <video>...");
         let screenshot = video_elem.screenshot_as_png().await?;
         std::fs::write("captura_video.png", screenshot)?;
         println!("✅ Captura guardada como captura_video.png");
     }
     None => {
         println!("❌ No se pudo cargar el video con dimensiones válidas");
     }
 }

 // Cerrar el navegador
 driver.quit().await?;
 Ok(())
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
                 let video_elem = driver
                     .query(By::Css("video"))
                     .first()
                     .await?;
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
