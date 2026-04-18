#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;
use std::path::PathBuf;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct GitHubRelease {
    tag_name: String,
}

/// Detecta la carpeta .minecraft dependiendo del sistema operativo
fn get_minecraft_path() -> PathBuf {
    let home = dirs::home_dir().expect("No se pudo obtener el directorio HOME");
    if cfg!(target_os = "windows") {
        home.join("AppData").join("Roaming").join(".minecraft")
    } else {
        // En Linux usualmente es ~/.minecraft
        home.join(".minecraft")
    }
}

/// Obtiene el separador de classpath adecuado (Windows utiliza ; y Linux :)
fn get_classpath_separator() -> &'static str {
    if cfg!(target_os = "windows") { ";" } else { ":" }
}

#[tauri::command]
async fn check_github_updates() -> Result<String, String> {
    let client = reqwest::Client::new();
    // Al ser público, ya no necesitamos el token para consultar las releases
    let url = "https://api.github.com/repos/TU_USUARIO/launcher_efic/releases/latest";

    let response = client.get(url)
        .header("User-Agent", "rust-minecraft-launcher")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let release: GitHubRelease = response.json().await.map_err(|e| e.to_string())?;
        Ok(release.tag_name)
    } else {
        Err("No se pudo conectar con GitHub para buscar actualizaciones".to_string())
    }
}

#[tauri::command]
fn launch_minecraft(username: String, version: String) -> Result<String, String> {
    if username.trim().is_empty() {
        return Err("El nombre de usuario es obligatorio".to_string());
    }

    let mc_path = get_minecraft_path();
    let sep = get_classpath_separator();
    
    // Ruta al archivo JAR del cliente
    let client_jar = mc_path.join("versions").join(&version).join(format!("{}.jar", version));
    
    if !client_jar.exists() {
        return Err(format!("No se encontró el archivo JAR para la versión {}. Asegúrate de que esté descargada en {}", version, mc_path.display()));
    }

    // Construcción básica del comando Java
    // Nota: Un comando real de Minecraft requiere muchas más librerías en el classpath
    let classpath = format!("{}{}{}", client_jar.display(), sep, mc_path.join("libraries/*").display());

    let status = Command::new("java")
        .arg("-Xmx2G")
        .arg("-cp")
        .arg(classpath)
        .arg("net.minecraft.client.main.Main")
        .arg("--username").arg(&username)
        .arg("--version").arg(&version)
        .arg("--gameDir").arg(mc_path.display().to_string())
        .arg("--assetsDir").arg(mc_path.join("assets").display().to_string())
        .arg("--uuid").arg(uuid::Uuid::new_v4().to_string())
        .arg("--accessToken").arg("0") // No-premium usa 0 o cualquier valor
        .arg("--userType").arg("legacy")
        .spawn();

    match status {
        Ok(_) => Ok(format!("Iniciando Minecraft {} como {}", version, username)),
        Err(e) => Err(format!("Error al ejecutar Java: {}. ¿Está Java instalado y en el PATH?", e)),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            check_github_updates,
            launch_minecraft
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}