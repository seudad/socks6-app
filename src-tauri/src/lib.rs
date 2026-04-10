mod profiles;
mod proxy;
mod system_proxy;

use profiles::ProfileStore;
use proxy::ProxyEngine;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub profiles: Mutex<ProfileStore>,
    pub proxy: Arc<ProxyEngine>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "socks6_app=info".parse().unwrap()),
        )
        .init();

    let profiles_store = ProfileStore::load().unwrap_or_default();
    let state = AppState {
        profiles: Mutex::new(profiles_store),
        proxy: Arc::new(ProxyEngine::new()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            list_profiles,
            save_profile,
            delete_profile,
            connect,
            disconnect,
            get_status,
            set_system_proxy,
            generate_keys,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ── Profile commands ────────────────────────────────────────────────────

#[tauri::command]
async fn list_profiles(state: tauri::State<'_, AppState>) -> Result<Vec<profiles::Profile>, String> {
    let store = state.profiles.lock().await;
    Ok(store.list())
}

#[tauri::command]
async fn save_profile(
    state: tauri::State<'_, AppState>,
    profile: profiles::Profile,
) -> Result<(), String> {
    let mut store = state.profiles.lock().await;
    store.upsert(profile);
    store.save().map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_profile(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut store = state.profiles.lock().await;
    store.remove(&id);
    store.save().map_err(|e| e.to_string())
}

// ── Proxy commands ──────────────────────────────────────────────────────

#[tauri::command]
async fn connect(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    profile_id: String,
) -> Result<(), String> {
    let profile = {
        let store = state.profiles.lock().await;
        store
            .get(&profile_id)
            .ok_or_else(|| format!("profile not found: {profile_id}"))?
            .clone()
    };
    state
        .proxy
        .start(profile, app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn disconnect(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.proxy.stop().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_status(state: tauri::State<'_, AppState>) -> Result<proxy::ProxyStatus, String> {
    Ok(state.proxy.status().await)
}

// ── System proxy ────────────────────────────────────────────────────────

#[tauri::command]
async fn set_system_proxy(enable: bool, listen_addr: String) -> Result<(), String> {
    system_proxy::set_system_proxy(enable, &listen_addr)
        .await
        .map_err(|e| e.to_string())
}

// ── Key generation ──────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct GeneratedKeys {
    secret: String,
    short_id: String,
}

#[tauri::command]
fn generate_keys() -> GeneratedKeys {
    use rand::RngCore;

    let mut secret = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut secret);

    let mut short_id = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut short_id);

    GeneratedKeys {
        secret: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, secret),
        short_id: socks6::reality::hex_encode(&short_id),
    }
}
