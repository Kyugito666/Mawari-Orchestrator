// orchestrator/src/github.rs

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

fn stop_codespace(token: &str, name: &str) -> Result<(), GHError> {
    println!("      Menghentikan '{}'...", name);
    match run_gh_command(token, &["codespace", "stop", "-c", name]) {
        Ok(_) => { println!("      Berhasil dihentikan."); Ok(()) }
        Err(e) => { eprintln!("      Peringatan saat menghentikan: {}", e); Ok(()) }
    }
}

fn delete_codespace(token: &str, name: &str) -> Result<(), GHError> {
    println!("      Menghapus '{}'...", name);
    match run_gh_command(token, &["codespace", "delete", "-c", name, "--force"]) {
        Ok(_) => { println!("      Berhasil dihapus."); thread::sleep(Duration::from_secs(3)); Ok(()) }
        Err(e) => { eprintln!("      Gagal menghapus, melanjutkan...: {}", e); Ok(()) }
    }
}

fn health_check(token: &str, name: &str) -> bool {
    let check_cmd = "test -f /tmp/mawari_auto_start_done && echo 'healthy'";
    match run_gh_command(token, &["codespace", "ssh", "-c", name, "--", check_cmd]) {
        Ok(output) if output.contains("healthy") => true,
        _ => false,
    }
}

// MODIFIED: 'setup_mode' ditambahkan untuk membedakan setup wallet
pub fn wait_and_run_startup_script(token: &str, name: &str, script_path: &str, setup_mode: &str) -> Result<(), GHError> {
    println!("   Memverifikasi dan menjalankan node di '{}'...", name);
    
    for attempt in 1..=15 {
        println!("      Attempt {}/15: Mengecek kesiapan SSH...", attempt);
        match run_gh_command(token, &["codespace", "ssh", "-c", name, "--", "echo 'ready'"]) {
            Ok(output) if output.contains("ready") => {
                println!("      ✅ SSH sudah siap!");
                // MODIFIED: Menyuntikkan environment variable SETUP_MODE saat eksekusi
                let exec_command = format!("bash -l -c 'export SETUP_MODE={} && bash {}'", setup_mode, script_path);
                
                println!("      🚀 Menjalankan skrip auto-start (Mode: {}): {}", setup_mode, script_path);
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


// MODIFIED: Fungsi ini sekarang mengelola DUA codespace Mawari
pub fn ensure_mawari_codespaces(token: &str, repo: &str) -> Result<(String, String), GHError> {
    println!("  Mengecek Codespace Mawari yang ada...");
    
    let mut cs1_name = String::new(); // Untuk 1+5 wallets
    let mut cs2_name = String::new(); // Untuk 6 wallets

    let list_output = run_gh_command(token, &["codespace", "list", "--json", "name,repository,state,displayName"])?;
    
    if !list_output.is_empty() {
        if let Ok(codespaces) = serde_json::from_str::<Vec<serde_json::Value>>(&list_output) {
            for cs in codespaces {
                if cs["repository"].as_str().unwrap_or("") != repo { continue; }

                let name = cs["name"].as_str().unwrap_or("").to_string();
                let state = cs["state"].as_str().unwrap_or("").to_string();
                let display_name = cs["displayName"].as_str().unwrap_or("");

                let mut process_node = |current_name: &mut String, target_display: &str| -> Result<(), GHError> {
                    if display_name == target_display {
                        println!("  -> Ditemukan '{}': {} (State: {})", target_display, name, state);
                        
                        if state == "Available" && health_check(token, &name) {
                            println!("    ✅ Health check LULUS. Digunakan kembali.");
                            *current_name = name.clone();
                        } else {
                            println!("    ❌ Health check GAGAL atau state tidak 'Available'. Dibuat ulang...");
                            if state == "Available" || state == "Running" { stop_codespace(token, &name)?; }
                            delete_codespace(token, &name)?;
                        }
                    }
                    Ok(())
                };

                process_node(&mut cs1_name, "mawari-nodes-1")?;
                process_node(&mut cs2_name, "mawari-nodes-2")?;
            }
        }
    }

    let script_path = "/workspaces/Mawari-Orchestrator/mawari/auto-start.sh";

    // --- Logic untuk Codespace #1 (1+5 Wallets) ---
    if cs1_name.is_empty() {
        println!("\n  Membuat codespace 'mawari-nodes-1'...");
        let new_name = run_gh_command(token, &["codespace", "create", "-r", repo, "-m", "standardLinux32gb", "--display-name", "mawari-nodes-1", "--idle-timeout", "240m"])?;
        if new_name.is_empty() { return Err(GHError::CommandError("Gagal membuat codespace mawari-nodes-1".to_string())); }
        
        cs1_name = new_name;
        println!("     ✅ Berhasil dibuat: {}", cs1_name);
        wait_and_run_startup_script(token, &cs1_name, script_path, "PRIMARY")?;
    }
    
    println!("\n  Menunggu 10 detik sebelum lanjut ke codespace berikutnya...\n");
    thread::sleep(Duration::from_secs(10));
    
    // --- Logic untuk Codespace #2 (6 Wallets) ---
    if cs2_name.is_empty() {
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
