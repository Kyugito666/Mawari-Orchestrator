# orchestrator/setup_mawari.py

import json
import subprocess
import os
import time
import sys

# --- Nama File Konfigurasi & Data ---
CONFIG_FILE = 'config_setup.json'
TOKENS_FILE = 'tokens_mawari.json'
SECRETS_FILE = 'secrets_mawari.json'
TOKEN_CACHE_FILE = 'token_cache_mawari.json'
INVITED_USERS_FILE = 'invited_users_mawari.txt'

def load_setup_config():
    """Memuat konfigurasi utama dari config_setup.json"""
    try:
        with open(CONFIG_FILE, 'r') as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        print(f"❌ FATAL: File '{CONFIG_FILE}' tidak ditemukan atau formatnya salah.")
        print("Pastikan file tersebut ada dan berisi 'main_account_username', 'main_token', dan 'blueprint_repo_name'.")
        sys.exit(1) # Keluar dari script jika config tidak ada

# (Fungsi-fungsi lain seperti run_command, load_json_file, dll. tetap sama)
# ...
# ... (copy-paste fungsi run_command, load_json_file, save_json_file, 
#      load_lines_from_file, save_lines_to_file dari script lama ke sini)
# ...

def invite_collaborators(config):
    """Opsi 1: Mengundang kolaborator berdasarkan token."""
    print("\n--- Opsi 1: Auto Invite Collaborator & Get Username ---\n")
    # ... (sisa kodenya sama persis, tapi variabelnya diambil dari 'config')
    
    # Ganti baris ini:
    # env = os.environ.copy(); env['GH_TOKEN'] = MAIN_TOKEN_CONFIG
    # menjadi:
    env = os.environ.copy(); env['GH_TOKEN'] = config['main_token']
    
    # Ganti baris ini:
    # if username.lower() == MAIN_ACCOUNT_USERNAME.lower(): continue
    # menjadi:
    if username.lower() == config['main_account_username'].lower(): continue
    
    # Ganti baris ini:
    # command = f"gh api repos/{MAIN_ACCOUNT_USERNAME}/{BLUEPRINT_REPO_NAME}/collaborators/{username} -f permission=push --silent"
    # menjadi:
    command = f"gh api repos/{config['main_account_username']}/{config['blueprint_repo_name']}/collaborators/{username} -f permission=push --silent"
    
    # ... (sisa logika di fungsi ini tetap sama)

def auto_set_secrets(config):
    """Opsi 2: Sinkronisasi secrets ke semua akun."""
    # ... (logikanya sama, tapi variabelnya diambil dari 'config')

    # Ganti baris ini:
    # repo_full_name = f"{username}/{BLUEPRINT_REPO_NAME}"
    # menjadi:
    repo_full_name = f"{username}/{config['blueprint_repo_name']}"

    # Ganti baris ini:
    # success, _ = run_command(f"gh repo fork {MAIN_ACCOUNT_USERNAME}/{BLUEPRINT_REPO_NAME} --clone=false --remote=false", env=env)
    # menjadi:
    success, _ = run_command(f"gh repo fork {config['main_account_username']}/{config['blueprint_repo_name']} --clone=false --remote=false", env=env)
    # ... (sisa logika di fungsi ini tetap sama)

def auto_accept_invitations(config):
    """Opsi 3: Menerima undangan kolaborasi."""
    # ... (logikanya sama, tapi variabelnya diambil dari 'config')

    # Ganti baris ini:
    # target_repo = f"{MAIN_ACCOUNT_USERNAME}/{BLUEPRINT_REPO_NAME}".lower()
    # menjadi:
    target_repo = f"{config['main_account_username']}/{config['blueprint_repo_name']}".lower()
    # ... (sisa logika di fungsi ini tetap sama)

def main():
    """Fungsi utama untuk menjalankan setup tool."""
    # Muat konfigurasi di awal
    config = load_setup_config()
    print(f"✅ Konfigurasi berhasil dimuat untuk repo: {config['blueprint_repo_name']}")

    while True:
        print("\n=============================================")
        print("      MAWARI ORCHESTRATOR SETUP TOOL")
        print("=============================================")
        print("1. Validasi & Undang Kolaborator Baru")
        print("2. Auto Set Secrets (dengan Pengecekan)")
        print("3. Auto Accept Invitations")
        print("0. Keluar")
        choice = input("Pilih menu (1/2/3/0): ")
        if choice == '1': invite_collaborators(config)
        elif choice == '2': auto_set_secrets(config)
        elif choice == '3': auto_accept_invitations(config)
        elif choice == '0':
            print("Terima kasih!"); break
        else:
            print("Pilihan tidak valid.")
        input("\nTekan Enter untuk kembali ke menu utama...")

if __name__ == "__main__":
    main()
