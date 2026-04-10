use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub server: String,
    pub server_name: String,
    pub secret: String,
    pub short_id: String,
    #[serde(default)]
    pub auth_user: String,
    #[serde(default)]
    pub auth_pass: String,
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_max_tls")]
    pub max_tls_parallel: usize,
    #[serde(default)]
    pub auth_time_offset_secs: i64,
}

fn default_listen() -> String {
    "127.0.0.1:1080".to_string()
}

fn default_max_tls() -> usize {
    12
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProfileStore {
    profiles: Vec<Profile>,
}

fn profiles_path() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("socks6-app");
    std::fs::create_dir_all(&dir).ok();
    dir.join("profiles.json")
}

impl ProfileStore {
    pub fn load() -> Result<Self> {
        let path = profiles_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let store: Self =
            serde_json::from_str(&data).with_context(|| "parsing profiles.json")?;
        Ok(store)
    }

    pub fn save(&self) -> Result<()> {
        let path = profiles_path();
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data)
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    pub fn list(&self) -> Vec<Profile> {
        self.profiles.clone()
    }

    pub fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn upsert(&mut self, profile: Profile) {
        if let Some(existing) = self.profiles.iter_mut().find(|p| p.id == profile.id) {
            *existing = profile;
        } else {
            self.profiles.push(profile);
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.profiles.retain(|p| p.id != id);
    }
}
