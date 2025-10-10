// orchestrator/src/github.rs - Fixed warnings

use crate::config::State;
use std::process::{Command, Stdio};
use std::fmt;
use std::thread;
use std::time::Duration;
use std::io::{BufRead, BufReader};

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
    
    let mut child = Command::new("gh")
        .args(args)
        .env("GH_TOKEN", token)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| GHError::CommandError(format!("Gagal spawn gh: {}", e)))?;

    let stdout = child.stdout.take().ok_or_else(|| GHError::CommandError("Stdout tidak tersedia".to_string()))?;
    let stderr = child.stderr.take().ok_or_else(|| GHError::CommandError("Stderr tidak tersedia".to_string()))?;

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let stdout_handle = thread::spawn(move || {
        stdout_reader.lines().filter_map(|line| line.ok()).collect::<Vec<_>>()
    });

    let stderr_handle = thread::spawn(move || {
        stderr_reader.lines().filter_map(|line| line.ok()).collect::<Vec<_>>()
    });

    let start = std::time::Instant::now();
    let mut timed_out = false;

    loop {
        if start.elapsed() > Duration::from_secs(timeout_secs) {
            let _ = child.kill();
            timed_out = true;
            break;
        }

        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(e) => return Err(GHError::CommandError(format!("Wait error: {}", e))),
        }
    }

    if timed_out {
        return Err(GHError::Timeout(format!("Command timeout setelah {}s", timeout_secs)));
    }

    let stdout_lines = stdout_handle.join().unwrap_or_default();
    let stderr_lines = stderr_handle.join().unwrap_or_default();

    let status = child.wait().map_err(|e| GHError::CommandError(format!("Wait failed: {}", e)))?;

    let stdout_text = stdout_lines.join("\n");
    let stderr_text = stderr_lines.join("\n");

    if !stderr_text.is_empty() {
        eprintln!("DEBUG STDERR: {}", stderr_text.lines().take(5).collect::<Vec<_>>().join(" | "));
    }
    if !stdout_text.is_empty() {
        eprintln!("DEBUG STDOUT: {}", stdout_text.lines().take(5).collect::<Vec<_>>().join(" | "));
    }

    if !status.success() {
        if stderr_text.contains("Bad credentials") 
            || stderr_text.contains("authentication required")
            || stderr_text.contains("HTTP 401")
            || stderr_text.contains("HTTP 403") {
            return Err(GHError::AuthError(stderr_text));
        }

        if stderr_text.contains("no codespaces found") || (stdout_text.trim().is_empty() && stderr_text.trim().is_empty()) {
            return Ok("".to_string());
        }

        return Err(GHError::CommandError(if !stderr_text.is_empty() { stderr_text } else { stdout_text }));
    }

    Ok(stdout_text.trim().to_string())
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
                    eprintln!("      Peringatan saat berhenti: {}", e.to_string().lines().next().unwrap_or("error"));
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
        
        let result = run_gh_command_with_timeout(
            token,
            &[
                "codespace", "create",
                "-R", repo,
                "-m", "standardLinux32gb",
                "--display-name", display_name,
                "--idle-timeout", "240m",
                "--default-permissions"
            ],
            150
        );
        
        match result {
            Ok(output) if !output.trim().is_empty() => {
                let name = output.lines().next().unwrap_or(&output).trim();
                if name.len() > 5 {
                    println!("      ✅ Berhasil dibuat: {}", name);
                    return Ok(name.to_string());
                } else {
                    eprintln!("      ⚠️ Output tidak valid: '{}'", name);
                }
            }
            Ok(_) => {
                eprintln!("      ⚠️ Output kosong pada attempt {}", attempt);
            }
            Err(e) => {
                let err_msg = e.to_string();
                eprintln!("      ❌ Error pada attempt {}: {}", attempt, 
                    err_msg.lines().next().unwrap_or("unknown"));
                
                if err_msg.contains("Bad credentials") || err_msg.contains("HTTP 401") || err_msg.contains("HTTP 403") {
                    return Err(e);
                }
            }
        }
        
        if attempt < max_retries {
            let wait_time = 15 * attempt as u64;
            eprintln!("      ⏳ Retry dalam {} detik...", wait_time);
            thread::sleep(Duration::from_secs(wait_time));
        }
    }
    
    Err(GHError::CommandError(format!(
        "Gagal membuat '{}' setelah {} percobaan",
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
