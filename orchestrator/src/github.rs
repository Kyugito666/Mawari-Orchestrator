// orchestrator/src/github.rs

use crate::config::State;
use std::process::Command;
use std::fmt;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum GHError {
    CommandError(String),
    AuthError(String),
}

impl fmt::Display for GHError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GHError::CommandError(e) => write!(f, "Command Gagal: {}", e),
            GHError::AuthError(e) => write!(f, "Auth Error: {}", e),
        }
    }
}

// Helper function to run gh commands robustly
fn run_gh_command(token: &str, args: &[&str]) -> Result<String, GHError> {
    eprintln!("DEBUG: gh {}", args.join(" "));
    let output = Command::new("gh")
        .args(args)
        .env("GH_TOKEN", token)
        .output()
        .map_err(|e| GHError::CommandError(format!("Gagal eksekusi gh: {}", e)))?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    
    if !output.status.success() {
        if stderr.contains("Bad credentials") || stderr.contains("authentication required") || stderr.contains("HTTP 401") {
            return Err(GHError::AuthError(stderr));
        }
        if stderr.contains("no codespaces found") || stdout.trim().is_empty() {
            return Ok("".to_string());
        }
        return Err(GHError::CommandError(stderr));
    }
    
    Ok(stdout.trim().to_string())
}

pub fn get_username(token: &str) -> Result<String, GHError> {
    run_gh_command(token, &["api", "user", "--jq", ".login"])
}

// Function to stop a codespace
fn stop_codespace(token: &str, name: &str) -> Result<(), GHError> {
    println!("      Menghentikan '{}'...", name);
    match run_gh_command(token, &["codespace", "stop", "-c", name]) {
        Ok(_) => { println!("      Berhasil dihentikan."); Ok(()) }
        Err(e) => { eprintln!("      Peringatan saat menghentikan: {}", e); Ok(()) }
    }
}

// Function to delete a codespace with retries
fn delete_codespace(token: &str, name: &str) -> Result<(), GHError> {
    println!("      Menghapus '{}'...", name);
    for attempt in 1..=3 {
        match run_gh_command(token, &["codespace", "delete", "-c", name, "--force"]) {
            Ok(_) => { println!("      Berhasil dihapus."); thread::sleep(Duration::from_secs(3)); return Ok(()); }
            Err(_) => {
                if attempt < 3 { eprintln!("      Retry {}/3", attempt); thread::sleep(Duration::from_secs(5)); } 
                else { eprintln!("      Gagal menghapus, melanjutkan..."); return Ok(()); }
            }
        }
    }
    Ok(())
}

// Function to perform a health check by checking for a file inside the codespace
fn health_check(token: &str, name: &str) -> bool {
    println!("    -> Health Check untuk '{}'...", name);
    let check_cmd = "test -f /tmp/mawari_auto_start_done && echo 'healthy'";
    match run_gh_command(token, &["codespace", "ssh", "-c", name, "--", check_cmd]) {
        Ok(output) if output.contains("healthy") => {
            println!("    ✅ Health check LULUS.");
            true
        },
        _ => {
            println!("    ❌ Health check GAGAL.");
            false
        }
    }
}

// Function to wait for SSH and run the startup script
pub fn wait_and_run_startup_script(token: &str, name: &str, script_path: &str, setup_mode: &str) -> Result<(), GHError> {
    println!("   Memverifikasi dan menjalankan node di '{}'...", name);
    
    for attempt in 1..=15 {
        println!("      Attempt {}/15: Mengecek kesiapan SSH...", attempt);
        match run_gh_command(token, &["codespace", "ssh", "-c", name, "--", "echo 'ready'"]) {
            Ok(output) if output.contains("ready") => {
                println!("      ✅ SSH sudah siap!");
                let exec_command = format!("bash -l -c 'export SETUP_MODE={} && bash {}'", setup_mode, script_path);
                
                println!("      🚀 Menjalankan skrip auto-start (Mode: {})...", setup_mode);
                match run_gh_command(token, &["codespace", "ssh", "-c", name, "--", &exec_command]) {
                    Ok(_) => { println!("      ✅ Perintah eksekusi skrip berhasil dikirim."); return Ok(()); },
                    Err(e) => { eprintln!("      ⚠️  Peringatan saat eksekusi skrip: {}", e.to_string()); return Ok(()); }
                }
            },
            _ => { println!("      ... Belum siap."); }
        }
        if attempt < 15 {
            println!("      Menunggu 30 detik...");
            thread::sleep(Duration::from_secs(30));
        }
    }
    Err(GHError::CommandError(format!("Timeout: SSH tidak siap untuk '{}'", name)))
}

// UPGRADED: This function now implements the "Inspect, Health Check & Reuse" strategy
pub fn ensure_mawari_codespaces(token: &str, repo: &str, state: &State) -> Result<(String, String), GHError> {
    println!("\n  Menerapkan Strategi: Inspect, Health Check & Reuse...");
    
    let mut cs1_name = state.mawari_codespace_name.clone();
    let mut cs2_name = state.nexus_codespace_name.clone();

    // UPGRADED: Using robust list command with jq query from your reference repo
    let list_output = run_gh_command(token, &["codespace", "list", "--json", "name,repository,state,displayName", "-q", ".[]"])?;
    
    let mut found_cs1 = false;
    let mut found_cs2 = false;

    if !list_output.is_empty() {
        for line in list_output.lines() {
            if let Ok(cs) = serde_json::from_str::<serde_json::Value>(line) {
                let name = cs["name"].as_str().unwrap_or("").to_string();
                let cs_repo = cs["repository"]["nameWithOwner"].as_str().unwrap_or("");
                let cs_state = cs["state"].as_str().unwrap_or("");
                let display_name = cs["displayName"].as_str().unwrap_or("");

                if cs_repo != repo { continue; } // Skip codespaces from other repos

                if display_name == "mawari-nodes-1" {
                    println!("  -> Ditemukan 'mawari-nodes-1': {} (State: {})", name, cs_state);
                    if cs_state == "Available" && health_check(token, &name) {
                        cs1_name = name;
                        found_cs1 = true;
                    } else {
                        println!("     Tidak sehat atau state salah. Akan dihapus...");
                        if cs_state == "Available" || cs_state == "Running" { stop_codespace(token, &name)?; }
                        delete_codespace(token, &name)?;
                    }
                }

                if display_name == "mawari-nodes-2" {
                    println!("  -> Ditemukan 'mawari-nodes-2': {} (State: {})", name, cs_state);
                    if cs_state == "Available" && health_check(token, &name) {
                        cs2_name = name;
                        found_cs2 = true;
                    } else {
                        println!("     Tidak sehat atau state salah. Akan dihapus...");
                        if cs_state == "Available" || cs_state == "Running" { stop_codespace(token, &name)?; }
                        delete_codespace(token, &name)?;
                    }
                }
            }
        }
    }

    let script_path = "/workspaces/Mawari-Orchestrator/mawari/auto-start.sh";

    if !found_cs1 {
        println!("\n  Membuat codespace 'mawari-nodes-1'...");
        let new_name = run_gh_command(token, &["codespace", "create", "-r", repo, "-m", "standardLinux32gb", "--display-name", "mawari-nodes-1", "--idle-timeout", "240m"])?;
        if new_name.is_empty() { return Err(GHError::CommandError("Gagal membuat codespace mawari-nodes-1".to_string())); }
        
        cs1_name = new_name;
        println!("     ✅ Berhasil dibuat: {}", cs1_name);
        wait_and_run_startup_script(token, &cs1_name, script_path, "PRIMARY")?;
    }
    
    println!("\n  Menunggu 10 detik sebelum lanjut...\n");
    thread::sleep(Duration::from_secs(10));
    
    if !found_cs2 {
        println!("\n  Membuat codespace 'mawari-nodes-2'...");
        let new_name = run_gh_command(token, &["codespace", "create", "-r", repo, "-m", "standardLinux32gb", "--display-name", "mawari-nodes-2", "--idle-timeout", "240m"])?;
        if new_name.is_empty() { return Err(GHError::CommandError("Gagal membuat codespace mawari-nodes-2".to_string())); }

        cs2_name = new_name;
        println!("     ✅ Berhasil dibuat: {}", cs2_name);
        wait_and_run_startup_script(token, &cs2_name, script_path, "SECONDARY")?;
    }

    println!("\n  ✅ Kedua codespace Mawari siap!");
    Ok((cs1_name, cs2_name))
}

