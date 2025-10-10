// orchestrator/src/billing.rs - Final Stable Hybrid Version

use std::process::Command;
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

fn run_gh_api(token: &str, endpoint: &str) -> Result<String, String> {
    let output = Command::new("gh")
        .args(&["api", endpoint, "-H", "Accept: application/vnd.github+json"])
        .env("GH_TOKEN", token)
        .output()
        .map_err(|e| format!("Gagal mengeksekusi gh: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_billing_info(token: &str, username: &str) -> Result<BillingInfo, String> {
    let endpoint = format!("/users/{}/settings/billing/usage", username);
    
    let response = match run_gh_api(token, &endpoint) {
        Ok(r) => r,
        Err(e) => {
            println!("   PERINGATAN: Gagal menghubungi API billing ({}). Anggap kuota habis.", e.lines().next().unwrap_or("API error"));
            return Ok(BillingInfo {
                total_core_hours_used: 999.0,
                hours_remaining: 0.0,
                is_quota_ok: false,
            });
        }
    };
    
    if let Ok(report) = serde_json::from_str::<BillingReport>(&response) {
        let mut total_core_hours_used = 0.0;

        for item in report.usage_items {
            if item.product == "codespaces" {
                if item.sku.contains("compute 2-core") {
                    total_core_hours_used += item.quantity * 2.0;
                } else if item.sku.contains("compute 4-core") {
                    total_core_hours_used += item.quantity * 4.0;
                } else if item.sku.contains("compute 8-core") {
                    total_core_hours_used += item.quantity * 8.0;
                } else if item.sku.contains("compute 16-core") {
                    total_core_hours_used += item.quantity * 16.0;
                } else if item.sku.contains("compute 32-core") {
                    total_core_hours_used += item.quantity * 32.0;
                }
            }
        }
        
        let included_core_hours = 180.0; // Free tier GitHub
        let remaining_core_hours = included_core_hours - total_core_hours_used;
        
        // Asumsi kita menjalankan 2x standardLinux32gb (4-core) = 8 core total
        let hours_remaining = (remaining_core_hours / 8.0).max(0.0);
        
        let is_quota_ok = hours_remaining > 1.0; // Butuh minimal 1 jam sisa
        
        println!("Billing @{}: Digunakan ~{:.1} dari 180.0 core-hours | Kira-kira {:.1} jam tersisa", 
            username, 
            total_core_hours_used,
            hours_remaining
        );
        
        if !is_quota_ok {
            println!("   PERINGATAN: Kuota rendah (< 1 jam) atau habis.");
        } else {
            println!("   Kuota OK");
        }
        
        return Ok(BillingInfo {
            total_core_hours_used,
            hours_remaining,
            is_quota_ok,
        });
    }

    println!("   PERINGATAN: Format data billing tidak dikenal. Anggap kuota habis.");
    Ok(BillingInfo {
        total_core_hours_used: 999.0,
        hours_remaining: 0.0,
        is_quota_ok: false,
    })
}

