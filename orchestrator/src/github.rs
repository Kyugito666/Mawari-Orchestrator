// orchestrator/src/github.rs - Fixed Codespace Creation

use crate::config::State;
use std::process::{Command, Stdio};
use std::fmt;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub enum GHError {
    CommandError(String),
    AuthError(String),
    Timeout(String),
}

impl fmt::Display for GHError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GHError::CommandError(e) => write!(f, "Command gagal: {}", e),
            GHError::AuthError(e) => write!(f, "Auth error: {}", e),
            GHError::Timeout(e) => write!(f, "Timeout: {}", e),
        }
    }
}

fn run_gh_command_with_timeout(token: &str, args: &[&str], timeout_secs: u64) -> Result<String, GHError> {
    eprintln!("DEBUG: gh {} (timeout: {}s)", args.join(" "), timeout_secs);
    
    let token_clone = token.to_string();
    let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    
    let result = Arc::new(Mutex::new(None));
    let result_clone = Arc::clone(&result);
    let completed = Arc::new(AtomicBool::new(false));
    let completed_clone = Arc::clone(&completed);
    
    let handle = thread::spawn(move || {
        let output = Command::new("gh")
            .args(&args_vec)
            .env("GH_TOKEN", &token_clone)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        
        if completed_clone.load(Ordering::Relaxed) {
            return;
        }
        
        let mut res = result_clone.lock().unwrap();
        *res = Some(output);
    });
    
    let timeout_duration = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout_duration {
        if handle.is_finished() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    
    if !handle.is_finished() {
        completed.store(true, Ordering::Relaxed);
        return Err(GHError::Timeout(format!("Command timeout setelah {}s", timeout_secs)));
    }
    
    let _ = handle.join();
    
    let output_result = result.lock().unwrap().take()
        .ok_or_else(|| GHError::CommandError("Thread result kosong".to_string()))?
        .map_err(|e| GHError::CommandError(format!("Gagal mengeksekusi gh: {}", e)))?;

    let stderr = String::from_utf8_lossy(&output_result.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output_result.stdout).to_string();
    
    // Log raw output untuk debugging
    if !stderr.is_empty() {
        eprintln!("DEBUG STDERR: {}", stderr.lines().take(3).collect::<Vec<_>>().join(" | "));
    }
    if !stdout.is_empty() {
        eprintln!("DEBUG STDOUT: {}", stdout.lines().take(3).collect::<Vec<_>>().join(" | "));
    }
    
    if !output_result.status.success() {
        if stderr.contains("Bad credentials") 
            || stderr.contains("authentication required")
            || stderr.contains("HTTP 401") {
            return Err(GHError::AuthError(stderr));
        }
        
        if stderr.contains("no codespaces found") || (stdout.trim().is_empty() && stderr.trim().is_empty()) {
            return Ok("".to_string());
        }
        
        // Return stderr sebagai error message
        return Err(GHError::CommandError(if !stderr.is_empty() { stderr } else { stdout }));
    }
    
    // Beberapa gh command output ke stderr (seperti create), cek keduanya
    let output = if !stdout.trim().is_empty() {
        stdout.trim().to_string()
    } else if !stderr.trim().is_empty() && !stderr.contains("error") && !stderr.contains("failed") {
        stderr.trim().to_string()
    } else {
        stdout.trim().to_string()
    };
    
    Ok(output)
}

fn run_gh_command(token: &str, args: &[&str]) -> Result<String, GHError> {
    run_gh_command_with_timeout(token, args, 90)
}

pub fn get_username(token: &str) -> Result<String, GHError> {
    run_gh_command(token, &["api", "user", "--jq", ".login"])
}

fn stop_codespace(token: &str, name: &str) -> Result<(), GHError> {
    println!("      Menghentikan '{}'...", name);
    
    for attempt in 1..=2 {
        match run_gh_command_with_timeout(token, &["codespace", "stop", "-c", name], 45) {
            Ok(_) => {
                println!("      Berhenti");
                thread::sleep(Duration::from_secs(3));
                return Ok(());
            }
            Err(e) => {
                if attempt < 2 {
                    eprintln!("      Retry stop {}/2...", attempt);
                    thread::sleep(Duration::from_secs(2));
                } else {
                    eprintln!("      Peringatan saat berhenti: {}", e);
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

fn delete_codespace(token: &str, name: &str) -> Result<(), GHError> {
    println!("      Menghapus '{}'...", name);
    
    for attempt in 1..=3 {
        match run_gh_command_with_timeout(token, &["codespace", "delete", "-c", name, "--force"], 45) {
            Ok(_) => {
                println!("      Terhapus");
                thread::sleep(Duration::from_secs(2));
                return Ok(());
            }
            Err(e) => {
                if attempt < 3 {
                    eprintln!("      Retry delete {}/3... ({})", attempt, 
                        e.to_string().lines().next().unwrap_or("unknown"));
                    thread::sleep(Duration::from_secs(3));
                } else {
                    eprintln!("      Gagal hapus ({}), melanjutkan...", 
                        e.to_string().lines().next().unwrap_or("unknown"));
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

pub fn verify_codespace(token: &str, name: &str) -> Result<bool, GHError> {
    match run_gh_command_with_timeout(
        token, 
        &["codespace", "view", "-c", name, "--json", "state", "-q", ".state"],
        30
    ) {
        Ok(state) if state == "Available" => Ok(true),
        Ok(_) => Ok(false),
        Err(GHError::Timeout(_)) => Ok(false),
        Err(e) => Err(e),
    }
}

fn health_check(token: &str, name: &str) -> bool {
    let check_cmd = "test -f /tmp/mawari_auto_start_done && echo 'healthy' || echo 'unhealthy'";
    
    match run_gh_command_with_timeout(
        token, 
        &["codespace", "ssh", "-c", name, "--", check_cmd],
        25
    ) {
        Ok(output) if output.contains("healthy") => {
            eprintln!("      Health check: ✅ PASS");
            true
        }
        Ok(_) => {
            eprintln!("      Health check: ❌ FAIL (marker tidak ada)");
            false
        }
        Err(e) => {
            eprintln!("      Health check: ❌ FAIL ({})", 
                e.to_string().lines().next().unwrap_or("error"));
            false
        }
    }
}

pub fn wait_and_run_startup_script(token: &str, name: &str, script_path: &str, setup_mode: &str) -> Result<(), GHError> {
    println!("   Memverifikasi dan menjalankan node di '{}'...", name);
    
    for attempt in 1..=8 {
        println!("      SSH Check {}/8...", attempt);
        
        match run_gh_command_with_timeout(
            token, 
            &["codespace", "ssh", "-c", name, "--", "echo 'ready'"],
            20
        ) {
            Ok(output) if output.contains("ready") => {
                println!("      SSH siap!");
                break;
            }
            Err(GHError::Timeout(_)) if attempt < 8 => {
                println!("      Timeout, retry...");
                thread::sleep(Duration::from_secs(15));
            }
            _ if attempt < 8 => {
                println!("      Belum siap, tunggu 20s...");
                thread::sleep(Duration::from_secs(20));
            }
            _ => {
                return Err(GHError::Timeout(format!("SSH tidak siap untuk '{}'", name)));
            }
        }
    }
    
    let exec_command = format!(
        "bash -l -c 'export SETUP_MODE={} && nohup bash {} > /tmp/mawari_startup.log 2>&1 & echo $!'",
        setup_mode, 
        script_path
    );
    
    println!("      Eksekusi skrip (Mode: {})...", setup_mode);
    
    match run_gh_command_with_timeout(
        token, 
        &["codespace", "ssh", "-c", name, "--", &exec_command],
        30
    ) {
        Ok(pid) if !pid.is_empty() => {
            println!("      Skrip dijalankan (PID: {})", pid.lines().next().unwrap_or("unknown"));
            Ok(())
        }
        Ok(_) => {
            eprintln!("      Peringatan: Skrip dieksekusi tapi tanpa PID");
            Ok(())
        }
        Err(e) => {
            eprintln!("      Peringatan eksekusi: {}", 
                e.to_string().lines().next().unwrap_or("unknown"));
            Ok(())
        }
    }
}

fn create_codespace_with_retry(
    token: &str, 
    repo: &str, 
    display_name: &str, 
    max_retries: usize
) -> Result<String, GHError> {
    for attempt in 1..=max_retries {
        println!("      Attempt {}/{} untuk membuat '{}'...", attempt, max_retries, display_name);
        
        // Try WITHOUT --default-permissions first (might not be supported)
        let result = run_gh_command_with_timeout(
            token,
            &[
                "codespace", "create",
                "-r", repo,
                "-m", "standardLinux32gb",
                "--display-name", display_name,
                "--idle-timeout", "240m"
            ],
            120 // 2 menit untuk create
        );
        
        match result {
            Ok(output) if !output.trim().is_empty() => {
                let name = output.lines().next().unwrap_or(&output).trim();
                println!("      Berhasil dibuat: {}", name);
                return Ok(name.to_string());
            }
            Ok(_) => {
                eprintln!("      Output kosong pada attempt {}", attempt);
                if attempt < max_retries {
                    eprintln!("      Retry dalam 10 detik...");
                    thread::sleep(Duration::from_secs(10));
                }
            }
            Err(e) => {
                eprintln!("      Error pada attempt {}: {}", attempt, 
                    e.to_string().lines().next().unwrap_or("unknown"));
                if attempt < max_retries {
                    eprintln!("      Retry dalam 10 detik...");
                    thread::sleep(Duration::from_secs(10));
                }
            }
        }
    }
    
    Err(GHError::CommandError(format!(
        "Gagal membuat '{}' setelah {} percobaan (output selalu kosong atau error)",
        display_name, max_retries
    )))
}

pub fn ensure_healthy_codespaces(token: &str, repo: &str, state: &State) -> Result<(String, String), GHError> {
    println!("  Memeriksa Codespace yang ada...");
    
    let mut node1_name = state.mawari_node_1_name.clone();
    let mut node2_name = state.mawari_node_2_name.clone();

    let list_output = run_gh_command(
        token, 
        &["codespace", "list", "--json", "name,repository,state,displayName"]
    )?;
    
    let mut found_cs1 = false;
    let mut found_cs2 = false;

    if !list_output.is_empty() {
        if let Ok(codespaces) = serde_json::from_str::<Vec<serde_json::Value>>(&list_output) {
            for cs in codespaces {
                let name = cs["name"].as_str().unwrap_or("").to_string();
                let cs_repo = cs["repository"]["nameWithOwner"].as_str().unwrap_or("");
                let cs_state = cs["state"].as_str().unwrap_or("");
                let display_name = cs["displayName"].as_str().unwrap_or("");
                
                if cs_repo != repo { continue; }

                if display_name == "mawari-multi-node-1" && !found_cs1 {
                    println!("  Menemukan 'mawari-multi-node-1': {} (State: {})", name, cs_state);
                    
                    if cs_state == "Available" && health_check(token, &name) {
                        println!("    Health check LULUS. Digunakan kembali.");
                        node1_name = name.clone();
                        found_cs1 = true;
                    } else {
                        println!("    Health check GAGAL. Dibuat ulang...");
                        if cs_state == "Available" || cs_state == "Running" {
                            stop_codespace(token, &name)?;
                        }
                        delete_codespace(token, &name)?;
                    }
                }

                if display_name == "mawari-multi-node-2" && !found_cs2 {
                    println!("  Menemukan 'mawari-multi-node-2': {} (State: {})", name, cs_state);
                    
                    if cs_state == "Available" && health_check(token, &name) {
                        println!("    Health check LULUS. Digunakan kembali.");
                        node2_name = name.clone();
                        found_cs2 = true;
                    } else {
                        println!("    Health check GAGAL. Dibuat ulang...");
                        if cs_state == "Available" || cs_state == "Running" {
                            stop_codespace(token, &name)?;
                        }
                        delete_codespace(token, &name)?;
                    }
                }
            }
        } else {
            eprintln!("  Peringatan: Format list codespace tidak valid");
        }
    }
    
    let repo_basename = repo.split('/').last().unwrap_or("Mawari-Orchestrator");
    let script_path = format!("/workspaces/{}/mawari/auto-start.sh", repo_basename);

    if !found_cs1 {
        println!("  Membuat 'mawari-multi-node-1'...");
        node1_name = create_codespace_with_retry(token, repo, "mawari-multi-node-1", 3)?;
        
        thread::sleep(Duration::from_secs(5));
        wait_and_run_startup_script(token, &node1_name, &script_path, "PRIMARY")?;
    }
    
    thread::sleep(Duration::from_secs(15));
    
    if !found_cs2 {
        println!("  Membuat 'mawari-multi-node-2'...");
        node2_name = create_codespace_with_retry(token, repo, "mawari-multi-node-2", 3)?;
        
        thread::sleep(Duration::from_secs(5));
        wait_and_run_startup_script(token, &node2_name, &script_path, "SECONDARY")?;
    }

    println!("\n  Kedua Codespace siap!");
    Ok((node1_name, node2_name))
}
