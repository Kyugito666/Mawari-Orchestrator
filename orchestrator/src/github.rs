// orchestrator/src/github.rs - Final Stable Hybrid Version

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
            GHError::CommandError(e) => write!(f, "Command gagal: {}", e),
            GHError::AuthError(e) => write!(f, "Auth error: {}", e),
        }
    }
}

// Helper untuk menjalankan perintah gh
fn run_gh_command(token: &str, args: &[&str]) -> Result<String, GHError> {
    eprintln!("DEBUG: gh {}", args.join(" "));
    
    let output = Command::new("gh")
        .args(args)
        .env("GH_TOKEN", token)
        .output()
        .map_err(|e| GHError::CommandError(format!("Gagal mengeksekusi gh: {}", e)))?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    
    if !output.status.success() {
        if stderr.contains("Bad credentials") 
            || stderr.contains("authentication required")
            || stderr.contains("HTTP 401") {
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
        Ok(_) => { 
            println!("      Berhenti"); 
            thread::sleep(Duration::from_secs(3)); 
            Ok(()) 
        }
        Err(e) => { 
            eprintln!("      Peringatan saat berhenti: {}", e); 
            thread::sleep(Duration::from_secs(2)); 
            Ok(())
        }
    }
}

fn delete_codespace(token: &str, name: &str) -> Result<(), GHError> {
    println!("      Menghapus '{}'...", name);
    for attempt in 1..=3 {
        match run_gh_command(token, &["codespace", "delete", "-c", name, "--force"]) {
            Ok(_) => { 
                println!("      Terhapus"); 
                thread::sleep(Duration::from_secs(2)); 
                return Ok(()); 
            }
            Err(_) => {
                if attempt < 3 { 
                    eprintln!("      Coba lagi {}/3", attempt); 
                    thread::sleep(Duration::from_secs(3)); 
                } else { 
                    eprintln!("      Gagal, melanjutkan..."); 
                    return Ok(()); 
                }
            }
        }
    }
    Ok(())
}

pub fn verify_codespace(token: &str, name: &str) -> Result<bool, GHError> {
    let state_check = run_gh_command(token, &["codespace", "view", "-c", name, "--json", "state", "-q", ".state"]);
    match state_check {
        Ok(state) if state == "Available" => Ok(true),
        _ => Ok(false),
    }
}

// Pengecekan kesehatan dengan mencari file marker
fn health_check(token: &str, name: &str) -> bool {
    let check_cmd = "test -f /tmp/mawari_auto_start_done && echo 'healthy'";
    match run_gh_command(token, &["codespace", "ssh", "-c", name, "--", check_cmd]) {
        Ok(output) if output.contains("healthy") => true,
        _ => false,
    }
}

// Menunggu SSH siap dan menjalankan skrip startup dengan SETUP_MODE
pub fn wait_and_run_startup_script(token: &str, name: &str, script_path: &str, setup_mode: &str) -> Result<(), GHError> {
    println!("   Memverifikasi dan menjalankan node di '{}'...", name);
    
    for attempt in 1..=10 {
        println!("      Coba {}/10: Mengecek kesiapan SSH...", attempt);
        
        match run_gh_command(token, &["codespace", "ssh", "-c", name, "--", "echo 'ready'"]) {
            Ok(output) if output.contains("ready") => {
                println!("      SSH siap!");
                
                let exec_command = format!("bash -l -c 'export SETUP_MODE={} && bash {}'", setup_mode, script_path);
                
                println!("      Mengeksekusi skrip auto-start (Mode: {})...", setup_mode);
                match run_gh_command(token, &["codespace", "ssh", "-c", name, "--", &exec_command]) {
                    Ok(_) => {
                        println!("      Eksekusi skrip berhasil");
                        return Ok(());
                    },
                    Err(e) => {
                        eprintln!("      Peringatan skrip: {}", e.to_string().lines().next().unwrap_or(""));
                        return Ok(());
                    }
                }
            },
            _ => {
                println!("      Belum siap...");
            }
        }
        
        if attempt < 10 {
            println!("      Menunggu 30 detik...");
            thread::sleep(Duration::from_secs(30));
        }
    }
    
    Err(GHError::CommandError(format!("Timeout: SSH tidak siap untuk '{}'", name)))
}

// Fungsi utama untuk memastikan kedua codespace Mawari berjalan sehat
pub fn ensure_healthy_codespaces(token: &str, repo: &str, state: &State) -> Result<(String, String), GHError> {
    println!("  Memeriksa Codespace yang ada...");
    
    let mut node1_name = state.mawari_node_1_name.clone();
    let mut node2_name = state.mawari_node_2_name.clone();

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
                
                if cs_repo != repo { continue; }

                let process_node = |current_name: &mut String, found_flag: &mut bool, target_display: &str| -> Result<(), GHError> {
                    if display_name == target_display {
                        println!("  Menemukan '{}': {} (State: {})", target_display, name, cs_state);
                        
                        if cs_state == "Available" && health_check(token, &name) {
                            println!("    Health check LULUS. Digunakan kembali.");
                            *current_name = name.clone();
                            *found_flag = true;
                        } else {
                            println!("    Health check GAGAL. Dibuat ulang...");
                            if cs_state == "Available" || cs_state == "Running" {
                                stop_codespace(token, &name)?;
                            }
                            delete_codespace(token, &name)?;
                        }
                    }
                    Ok(())
                };

                process_node(&mut node1_name, &mut found_cs1, "mawari-multi-node-1")?;
                process_node(&mut node2_name, &mut found_cs2, "mawari-multi-node-2")?;
            }
        }
    }
    
    let repo_basename = repo.split('/').last().unwrap_or("Mawari-Orchestrator");
    let script_path = format!("/workspaces/{}/mawari/auto-start.sh", repo_basename);

    if !found_cs1 {
        println!("  Membuat 'mawari-multi-node-1'...");
        let new_name = run_gh_command(token, &[
            "codespace", "create", 
            "-r", repo, 
            "-m", "standardLinux32gb",
            "--display-name", "mawari-multi-node-1", 
            "--idle-timeout", "240m"
        ])?;
        
        if new_name.is_empty() { 
            return Err(GHError::CommandError("Gagal membuat node-1".to_string())); 
        }
        
        node1_name = new_name;
        println!("     Dibuat: {}", node1_name);
        
        wait_and_run_startup_script(token, &node1_name, &script_path, "PRIMARY")?;
    }
    
    thread::sleep(Duration::from_secs(10));
    
    if !found_cs2 {
        println!("  Membuat 'mawari-multi-node-2'...");
        let new_name = run_gh_command(token, &[
            "codespace", "create", 
            "-r", repo, 
            "-m", "standardLinux32gb",
            "--display-name", "mawari-multi-node-2", 
            "--idle-timeout", "240m"
        ])?;
        
        if new_name.is_empty() { 
            return Err(GHError::CommandError("Gagal membuat node-2".to_string())); 
        }
        
        node2_name = new_name;
        println!("     Dibuat: {}", node2_name);
        
        wait_and_run_startup_script(token, &node2_name, &script_path, "SECONDARY")?;
    }

    println!("\n  Kedua Codespace siap!");
    Ok((node1_name, node2_name))
}

