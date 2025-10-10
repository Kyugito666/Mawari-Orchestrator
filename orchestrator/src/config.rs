// orchestrator/src/config.rs

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Deserialize)]
pub struct Config {
    pub tokens: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct State {
    pub current_account_index: usize,
    pub mawari_codespace_name: String,   // Akan menyimpan nama CS #1
    pub nexus_codespace_name: String,    // Akan menyimpan nama CS #2
}

pub fn load_config(path: &str) -> io::Result<Config> {
    if !Path::new(path).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File {} tidak ditemukan. Pastikan file ada dan berisi token GitHub Anda.", path)
        ));
    }
    
    let data = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Format JSON salah: {}", e)))?;
    
    if config.tokens.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Array 'tokens' kosong di dalam file JSON."));
    }
    
    Ok(config)
}

pub fn load_state(path: &str) -> io::Result<State> {
    if !Path::new(path).exists() {
        return Ok(State::default());
    }
    
    let data = fs::read_to_string(path)?;
    let state: State = serde_json::from_str(&data).unwrap_or_default();
    Ok(state)
}

pub fn save_state(path: &str, state: &State) -> io::Result<()> {
    let data = serde_json::to_string_pretty(state)?;
    fs::write(path, data)?;
    Ok(())
}
