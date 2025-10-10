// orchestrator/src/billing.rs - Hardened Version

use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct BillingInfo {
    pub total_core_hours_used: f32,
    pub hours_remaining: f32,
    pub is_quota_ok: bool,
}

#[derive(Deserialize, Debug)]
struct UsageItem {
    product: String,
    sku: String,
    quantity: f32,
}

#[derive(Deserialize, Debug)]
struct BillingReport {
    #[serde(rename = "usageItems")]
    usage_items: Vec<UsageItem>,
}

fn run_gh_api_with_timeout(token: &str, endpoint: &str, timeout_secs: u64) -> Result<String, String> {
    let token_clone = token.to_string();
    let endpoint_clone = endpoint.to_string();
    
    let result = Arc::new(Mutex::new(None));
    let result_clone = Arc::clone(&result);
    let completed = Arc::new(AtomicBool::new(false));
    let completed_clone = Arc::clone(&completed);
    
    let handle = thread::spawn(move || {
        let output = Command::new("gh")
            .args(&["api", &endpoint_clone, "-H", "Accept: application/vnd.github+json"])
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
        return Err(format!("API billing timeout setelah {}s", timeout_secs));
    }
    
    let _ = handle.join();
    
    let output = result.lock().unwrap().take()
        .ok_or_else(|| "Thread result kosong".to_string())?
        .map_err(|e| format!("Gagal mengeksekusi gh: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_gh_api(token: &str, endpoint: &str) -> Result<String, String> {
    run_gh_api_with_timeout(token, endpoint, 30)
}

pub fn get_billing_info(token: &str, username: &str) -> Result<BillingInfo, String> {
    let endpoint = format!("/users/{}/settings/billing/usage", username);
    
    let response = match run_gh_api(token, &endpoint) {
        Ok(r) => r,
        Err(e) => {
            let error_preview = e.lines().next().unwrap_or("API error");
            eprintln!("   PERINGATAN: Gagal menghubungi API billing ({})", error_preview);
            
            // Jika timeout atau network error, kembalikan state "tidak yakin"
            if e.contains("timeout") || e.contains("network") || e.contains("connection") {
                return Ok(BillingInfo {
                    total_core_hours_used: 0.0,
                    hours_remaining: 0.0,
                    is_quota_ok: false,
                });
            }
            
            // Untuk error lain (misal auth), anggap quota habis
            return Ok(BillingInfo {
                total_core_hours_used: 999.0,
                hours_remaining: 0.0,
                is_quota_ok: false,
            });
        }
    };
    
    let report = match serde_json::from_str::<BillingReport>(&response) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("   PERINGATAN: Format data billing tidak dikenal ({})", e);
            return Ok(BillingInfo {
                total_core_hours_used: 999.0,
                hours_remaining: 0.0,
                is_quota_ok: false,
            });
        }
    };
    
    let mut total_core_hours_used = 0.0;

    for item in report.usage_items {
        if item.product == "codespaces" {
            let multiplier = if item.sku.contains("compute 2-core") {
                2.0
            } else if item.sku.contains("compute 4-core") {
                4.0
            } else if item.sku.contains("compute 8-core") {
                8.0
            } else if item.sku.contains("compute 16-core") {
                16.0
            } else if item.sku.contains("compute 32-core") {
                32.0
            } else {
                continue; // Unknown SKU, skip
            };
            
            total_core_hours_used += item.quantity * multiplier;
        }
    }
    
    let included_core_hours = 180.0; // Free tier GitHub
    let remaining_core_hours = (included_core_hours - total_core_hours_used).max(0.0);
    
    // Asumsi kita menjalankan 2x standardLinux32gb (4-core) = 8 core total
    let hours_remaining = (remaining_core_hours / 8.0).max(0.0);
    
    let is_quota_ok = hours_remaining > 1.0; // Butuh minimal 1 jam sisa
    
    println!("Billing @{}: Digunakan ~{:.1} dari {:.1} core-hours | Kira-kira {:.1} jam tersisa", 
        username, 
        total_core_hours_used,
        included_core_hours,
        hours_remaining
    );
    
    if !is_quota_ok {
        println!("   PERINGATAN: Kuota rendah (< 1 jam) atau habis.");
    } else {
        println!("   Kuota OK");
    }
    
    Ok(BillingInfo {
        total_core_hours_used,
        hours_remaining,
        is_quota_ok,
    })
}
