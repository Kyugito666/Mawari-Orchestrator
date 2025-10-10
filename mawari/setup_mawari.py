# orchestrator/setup_mawari.py

import json
import subprocess
import os
import time

# ==========================================================
# KONFIGURASI
# ==========================================================
MAIN_TOKEN_CONFIG = "ghp_..." # Ganti dengan token utama Anda
MAIN_ACCOUNT_USERNAME = "YourGitHubUsername" 
BLUEPRINT_REPO_NAME = "Mawari-Orchestrator" # Sesuaikan dengan nama repo baru
# ==========================================================

# MODIFIED: Nama file spesifik untuk Mawari
TOKENS_FILE = 'tokens_mawari.json'
SECRETS_FILE = 'secrets_mawari.json'
TOKEN_CACHE_FILE = 'token_cache_mawari.json'
INVITED_USERS_FILE = 'invited_users_mawari.txt'

def run_command(command, env=None, input_data=None):
    try:
        process = subprocess.run(command, shell=True, check=True, capture_output=True, text=True, encoding='utf-8', env=env, input=input_data)
        return (True, process.stdout.strip())
    except subprocess.CalledProcessError as e:
        return (False, f"{e.stdout.strip()} {e.stderr.strip()}")

# ... (sisa fungsi helper seperti load_json_file, dll. tetap sama) ...

def invite_collaborators():
    # ... (logika sama, tapi membaca dari TOKENS_FILE, TOKEN_CACHE_FILE, INVITED_USERS_FILE) ...

def auto_set_secrets():
    print("\n--- Opsi 2: Auto Set Secrets untuk Mawari ---\n")
    secrets_to_set = load_json_file(SECRETS_FILE)
    if not secrets_to_set:
        print(f"❌ FATAL: {SECRETS_FILE} tidak ditemukan atau kosong."); return
    print(f"✅ Berhasil memuat secrets dari {SECRETS_FILE}.")

    # ... (sisa logika sama, tapi membaca dari TOKENS_FILE dan hanya set secret Mawari) ...

def auto_accept_invitations():
    # ... (logika sama, tapi membaca dari TOKENS_FILE) ...

def main():
    while True:
        print("\n=============================================")
        print("      MAWARI ORCHESTRATOR SETUP TOOL")
        print("=============================================")
        # ... (menu tetap sama) ...

if __name__ == "__main__":
    main()
