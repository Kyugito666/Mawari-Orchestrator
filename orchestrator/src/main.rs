// orchestrator/src/main.rs

mod config;
mod github;
mod billing;

use std::thread;
use std::time::{Duration, Instant};
use std::env;

const STATE_FILE: &str = "state.json";
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(4 * 3600); // 4 jam

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("❌ ERROR: Gunakan: cargo run --release -- username/nama-repo");
        return;
    }
    let repo_name = &args[1];

    println!("==================================================");
    println!("      MAWARI MULTI-CODESPACE ORCHESTRATOR");
    println!("==================================================");
    
    // MODIFIED: Membaca file token khusus Mawari
    println!("\nMemuat tokens_mawari.json...");
    let config = match config::load_config("tokens_mawari.json") {
        Ok(cfg) => cfg,
        Err(e) => { eprintln!("FATAL: {}", e); return; }
    };
    
    println!("Berhasil memuat {} token", config.tokens.len());
    println!("Target Repo: {}", repo_name);

    let mut state = config::load_state(STATE_FILE).unwrap_or_default();
    let mut current_token_index = state.current_account_index;

    loop {
        let token = &config.tokens[current_token_index];
        println!("\n==================================================");
        println!("Menggunakan Token #{}", current_token_index + 1);
        
        let username = match github::get_username(token) {
            Ok(u) => { println!("✅ Token valid untuk: @{}", u); u }
            Err(_) => {
                eprintln!("❌ Token TIDAK VALID atau error API. Ganti token...");
                current_token_index = (current_token_index + 1) % config.tokens.len();
                state.current_account_index = current_token_index;
                config::save_state(STATE_FILE, &state).ok();
                continue;
            }
        };

        println!("\nMengecek kuota billing...");
        let billing = billing::get_billing_info(token, &username).unwrap();
        if !billing.is_quota_ok {
            eprintln!("   Kuota tidak cukup. Beralih ke akun berikutnya...\n");
            current_token_index = (current_token_index + 1) % config.tokens.len();
            state.current_account_index = current_token_index;
            config::save_state(STATE_FILE, &state).ok();
            continue;
        }

        // MODIFIED: Memanggil fungsi baru dan menyimpan 2 nama codespace
        let (cs1_name, cs2_name) = match github::ensure_mawari_codespaces(token, repo_name) {
            Ok(names) => names,
            Err(e) => {
                eprintln!("❌ Deployment gagal: {}", e);
                eprintln!("   Mencoba lagi dalam 5 menit...\n");
                thread::sleep(Duration::from_secs(5 * 60));
                continue;
            }
        };

        println!("\n================ DEPLOYMENT BERHASIL ================");
        println!("Akun       : @{}", username);
        println!("Mawari CS #1: {}", cs1_name);
        println!("Mawari CS #2: {}", cs2_name);
        
        state.current_account_index = current_token_index;
        state.mawari_codespace_name = cs1_name.clone();
        state.nexus_codespace_name = cs2_name.clone(); // Menggunakan field lama, tapi isinya nama CS kedua
        config::save_state(STATE_FILE, &state).ok();
        
        // MODIFIED: Durasi jalan dikurangi sedikit untuk memperhitungkan 2 codespace @ 32-core
        let run_duration_hours = (billing.hours_remaining / 2.0 - 1.0).max(1.0);
        let run_duration = Duration::from_secs((run_duration_hours * 3600.0) as u64);
        
        println!("\nNode akan berjalan selama {:.1} jam", run_duration_hours);
        println!("Keep-alive akan dijalankan setiap 4 jam.\n");
        
        let start_time = Instant::now();
        
        while start_time.elapsed() < run_duration {
            let remaining_time = run_duration.saturating_sub(start_time.elapsed());
            let sleep_time = std::cmp::min(KEEP_ALIVE_INTERVAL, remaining_time);

            if sleep_time.as_secs() > 60 {
                 println!("Siklus keep-alive berikutnya dalam {:.1} jam...", sleep_time.as_secs_f32() / 3600.0);
                 thread::sleep(sleep_time);
            } else { break; }

            if start_time.elapsed() >= run_duration { break; }
            
            // MODIFIED: Menjalankan keep-alive untuk KEDUA codespace
            println!("\n--- MENJALANKAN SIKLUS KEEP-ALIVE ---");
            let script_path = "/workspaces/Mawari-Orchestrator/mawari/auto-start.sh";
            github::wait_and_run_startup_script(token, &cs1_name, script_path, "PRIMARY").ok();
            github::wait_and_run_startup_script(token, &cs2_name, script_path, "SECONDARY").ok();
            println!("--- SIKLUS KEEP-ALIVE SELESAI ---\n");
        }
        
        println!("\n==================================================");
        println!("Siklus Selesai! Beralih ke token berikutnya...");
        
        current_token_index = (current_token_index + 1) % config.tokens.len();
        state.current_account_index = current_token_index;
        config::save_state(STATE_FILE, &state).ok();
    }
}
