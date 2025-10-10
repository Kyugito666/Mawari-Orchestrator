// orchestrator/src/billing.rs

use std::process::Command;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct BillingInfo {
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
        .map_err(|e| format!("Gagal eksekusi gh: {}", e))?;

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
            eprintln!("   PERINGATAN: Gagal menghubungi API billing ({}). Anggap kuota habis.", e.lines().next().unwrap_or("API error"));
            return Ok(BillingInfo { hours_remaining: 0.0, is_quota_ok: false });
        }
    };
    
    if let Ok(report) = serde_json::from_str::<BillingReport>(&response) {
        let mut total_core_hours_used = 0.0;
        for item in report.usage_items {
            if item.product == "codespaces" {
                if item.sku.contains("compute 2-core") { total_core_hours_used += item.quantity * 2.0; }
                else if item.sku.contains("compute 4-core") { total_core_hours_used += item.quantity * 4.0; }
                else if item.sku.contains("compute 8-core") { total_core_hours_used += item.quantity * 8.0; }
                else if item.sku.contains("compute 16-core") { total_core_hours_used += item.quantity * 16.0; }
                else if item.sku.contains("compute 32-core") { total_core_hours_used += item.quantity * 32.0; }
            }
        }
        
        let included_core_hours = 180.0; // Free tier sekarang 180 core-hours
        let remaining_core_hours = included_core_hours - total_core_hours_used;
        
        // Asumsi kita pakai 2x 4-core = 8 core total, atau 2x 32-core = 64 core
        // Kita pakai basis 8-core untuk perhitungan aman
        let hours_remaining = (remaining_core_hours / 8.0).max(0.0);
        let is_quota_ok = hours_remaining > 1.0;
        
        return Ok(BillingInfo { hours_remaining, is_quota_ok });
    }

    eprintln!("   PERINGATAN: Format data billing tidak dikenal. Anggap kuota habis.");
    Ok(BillingInfo { hours_remaining: 0.0, is_quota_ok: false })
}
