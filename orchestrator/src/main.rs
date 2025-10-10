// orchestrator/src/main.rs - Final Stable Hybrid Version

mod config;
mod github;
mod billing;

use std::thread;
use std::time::{Duration, Instant};
use std::env;

const STATE_FILE: &str = "state.json";
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(3 * 3600 + 30 * 60); // 3.5 jam

// Fungsi untuk menampilkan status dari state.json
fn show_status() {
    println!("╔════════════════════════════════════════════════╗");
    println!("║        ORCHESTRATOR STATUS                    ║");
    println!("╚════════════════════════════════════════════════╝");
    
    match config::load_state(STATE_FILE) {
        Ok(state) => {
            println!("State file ditemukan");
            println!("Current Token Index: {}", state.current_account_index);
            if !state.mawari_node_1_name.is_empty() {
                println!("Node 1: {}", state.mawari_node_1_name);
            }
            if !state.mawari_node_2_name.is_empty() {
                println!("Node 2: {}", state.mawari_node_2_name);
            }
        }
        Err(_) => {
            println!("Tidak ada file state ditemukan");
        }
    }
    
    println!("\nTokens Tersedia:");
    match config::load_config("tokens.json") {
        Ok(cfg) => {
            println!("   Total: {} token", cfg.tokens.len());
        }
        Err(e) => {
            eprintln!("   Error: {}", e);
        }
    }
}

// Fungsi untuk memverifikasi kesehatan codespace yang sedang berjalan
fn verify_current() {
    println!("╔════════════════════════════════════════════════╗");
    println!("║        VERIFIKASI NODE                      ║");
    println!("╚════════════════════════════════════════════════╝");
    
    let state = match config::load_state(STATE_FILE) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Tidak ada file state ditemukan");
            return;
        }
    };
    
    let config = match config::load_config("tokens.json") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error memuat token: {}", e);
            return;
        }
    };
    
    if state.current_account_index >= config.tokens.len() {
        eprintln!("Indeks token tidak valid");
        return;
    }
    
    let token = &config.tokens[state.current_account_index];
    
    println!("Indeks Token: {}", state.current_account_index);
    
    if !state.mawari_node_1_name.is_empty() {
        println!("\n🔍 Memverifikasi Node 1: {}", state.mawari_node_1_name);
        match github::verify_codespace(token, &state.mawari_node_1_name) {
            Ok(true) => println!("   ✅ BERJALAN & TERSEDIA"),
            Ok(false) => println!("   ⚠️ TIDAK TERSEDIA atau BERHENTI"),
            Err(e) => eprintln!("   ❌ Error: {}", e),
        }
    }
    
    if !state.mawari_node_2_name.is_empty() {
        println!("\n🔍 Memverifikasi Node 2: {}", state.mawari_node_2_name);
        match github::verify_codespace(token, &state.mawari_node_2_name) {
            Ok(true) => println!("   ✅ BERJALAN & TERSEDIA"),
            Ok(false) => println!("   ⚠️ TIDAK TERSEDIA atau BERHENTI"),
            Err(e) => eprintln!("   ❌ Error: {}", e),
        }
    }
}

// Fungsi untuk menjalankan siklus keep-alive
fn restart_nodes(token: &str, name1: &str, name2: &str, repo_name: &str) {
    println!("\n╔════════════════════════════════════════════════╗");
    println!("║        SIKLUS KEEP-ALIVE                      ║");
    println!("╚════════════════════════════════════════════════╝");
    
    let repo_basename = repo_name.split('/').last().unwrap_or("Mawari-Orchestrator");
    let script_path = format!("/workspaces/{}/mawari/auto-start.sh", repo_basename);
    
    if !name1.is_empty() {
        println!("  🔄 Merestart Node 1 (PRIMARY): {}", name1);
        match github::wait_and_run_startup_script(token, name1, &script_path, "PRIMARY") {
            Ok(_) => println!("    ✅ Restart berhasil"),
            Err(e) => eprintln!("    ⚠️ Peringatan: {}", e),
        }
        thread::sleep(Duration::from_secs(5));
    }
    
    if !name2.is_empty() {
        println!("  🔄 Merestart Node 2 (SECONDARY): {}", name2);
        match github::wait_and_run_startup_script(token, name2, &script_path, "SECONDARY") {
            Ok(_) => println!("    ✅ Restart berhasil"),
            Err(e) => eprintln!("    ⚠️ Peringatan: {}", e),
        }
    }
    
    println!("\n✅ Siklus keep-alive selesai!\n");
}

// Fungsi utama program
fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        let command = args[1].trim_matches('"');
        if command == "status" {
            show_status();
            return;
        }
        if command == "verify" {
            verify_current();
            return;
        }
    }
    
    if args.len() < 2 {
        eprintln!("❌ ERROR: Argumen repositori tidak ada!");
        eprintln!("Gunakan: .\\start.bat \"username/repo-name\"");
        eprintln!("   atau: cargo run --release -- status");
        eprintln!("   atau: cargo run --release -- verify");
        return;
    }
    
    let repo_name = args[1].trim_matches('"');

    println!("╔════════════════════════════════════════════════╗");
    println!("║   MAWARI 12-NODE MULTI-WALLET ORCHESTRATOR    ║");
    println!("║            (Versi Stabil Terpadu)             ║");
    println!("╚════════════════════════════════════════════════╝");
    println!("📦 Repositori: {}", repo_name);
    println!("");

    let config = match config::load_config("tokens.json") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ FATAL: {}", e);
            eprintln!("   Buat config/tokens.json dengan token GitHub Anda");
            return;
        }
    };

    println!("✅ Berhasil memuat {} token", config.tokens.len());

    let mut state = config::load_state(STATE_FILE).unwrap_or_default();
    let mut i = state.current_account_index;
    
    if i >= config.tokens.len() {
        println!("⚠️ Mereset indeks tidak valid {} ke 0", i);
        i = 0;
    }

    let mut consecutive_failures = 0;
    const MAX_FAILURES: usize = 3;

    println!("\n🚀 Memulai loop orkestrasi...\n");

    loop {
        let token = &config.tokens[i];
        
        println!("\n╔════════════════════════════════════════════════╗");
        println!("║           TOKEN #{:<2} dari {:<2}                   ║", i + 1, config.tokens.len());
        println!("╚════════════════════════════════════════════════╝");

        let username = match github::get_username(token) {
            Ok(u) => {
                println!("✅ Token valid untuk: @{}", u);
                consecutive_failures = 0;
                u
            },
            Err(e) => {
                eprintln!("❌ Error token: {}", e);
                consecutive_failures += 1;
                
                if consecutive_failures >= MAX_FAILURES {
                    eprintln!("\n⚠️ Terlalu banyak kegagalan ({}). Cooldown 10 menit...", consecutive_failures);
                    thread::sleep(Duration::from_secs(600));
                    consecutive_failures = 0;
                }
                
                i = (i + 1) % config.tokens.len();
                state.current_account_index = i;
                config::save_state(STATE_FILE, &state).ok();
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        println!("\n📊 Mengecek kuota billing...");
        let billing = match billing::get_billing_info(token, &username) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("⚠️ Pengecekan billing gagal. Anggap habis...");
                i = (i + 1) % config.tokens.len();
                state.current_account_index = i;
                config::save_state(STATE_FILE, &state).ok();
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        if !billing.is_quota_ok {
            eprintln!("\n⚠️ Kuota tidak cukup untuk @{}", username);
            eprintln!("   Beralih ke akun berikutnya...\n");
            i = (i + 1) % config.tokens.len();
            state.current_account_index = i;
            config::save_state(STATE_FILE, &state).ok();
            thread::sleep(Duration::from_secs(5));
            continue;
        }

        println!("\n🚀 Memastikan Codespace sehat untuk @{}...", username);
        let (node1_name, node2_name) = match github::ensure_healthy_codespaces(token, repo_name, &state) {
            Ok(names) => {
                consecutive_failures = 0;
                names
            },
            Err(e) => {
                eprintln!("\n❌ Deployment gagal: {}", e);
                consecutive_failures += 1;
                
                if consecutive_failures >= MAX_FAILURES {
                    eprintln!("\n⚠️ Terlalu banyak kegagalan deployment. Cooldown 15 menit...", consecutive_failures);
                    thread::sleep(Duration::from_secs(900));
                    consecutive_failures = 0;
                } else {
                    eprintln!("   Mencoba lagi dalam 5 menit...");
                    thread::sleep(Duration::from_secs(300));
                }
                continue;
            }
        };

        println!("\n╔════════════════════════════════════════════════╗");
        println!("║         DEPLOYMENT BERHASIL! 🎉              ║");
        println!("╚════════════════════════════════════════════════╝");
        println!("Akun: @{}", username);
        println!("Node 1:  {}", node1_name);
        println!("Node 2:  {}", node2_name);
        
        state.current_account_index = i;
        state.mawari_node_1_name = node1_name.clone();
        state.mawari_node_2_name = node2_name.clone();
        config::save_state(STATE_FILE, &state).ok();

        let run_duration_hours = (billing.hours_remaining - 0.5).max(0.5).min(20.0);
        let run_duration = Duration::from_secs((run_duration_hours * 3600.0) as u64);
        
        println!("\n⏱️ Berjalan selama {:.1} jam", run_duration_hours);
        println!("   Interval Keep-alive: {:.1} jam", KEEP_ALIVE_INTERVAL.as_secs() as f32 / 3600.0);
        
        let start_time = Instant::now();
        let mut cycle_count = 0;
        
        while start_time.elapsed() < run_duration {
            let remaining = run_duration.saturating_sub(start_time.elapsed());
            let sleep_for = std::cmp::min(remaining, KEEP_ALIVE_INTERVAL);
            
            if sleep_for.as_secs() < 60 {
                println!("\n⏰ Waktu habis! Beralih akun...");
                break;
            }

            let hours_left = remaining.as_secs() as f32 / 3600.0;
            println!("\n💤 Tidur selama {:.1} jam (sisa: {:.1} jam)...", 
                sleep_for.as_secs() as f32 / 3600.0, hours_left);
            
            thread::sleep(sleep_for);

            if start_time.elapsed() >= run_duration {
                break;
            }
            
            cycle_count += 1;
            restart_nodes(token, &node1_name, &node2_name, repo_name);
        }
        
        println!("\n╔════════════════════════════════════════════════╗");
        println!("║         SIKLUS SELESAI                        ║");
        println!("╚════════════════════════════════════════════════╝");
        println!("Akun: @{}", username);
        println!("Durasi: {:.1} jam", run_duration_hours);
        println!("Siklus Keep-alive: {}", cycle_count);
        println!("⏭️ Beralih ke token berikutnya...\n");
        
        i = (i + 1) % config.tokens.len();
        state.current_account_index = i;
        config::save_state(STATE_FILE, &state).ok();
        
        println!("⏸️ Cooldown 30 detik...");
        thread::sleep(Duration::from_secs(30));
    }
}

