# orchestrator/setup_mawari.py

import json
import subprocess
import os
import time
import sys

# --- Nama File Konfigurasi & Data ---
CONFIG_FILE = 'config/config_setup.json'
TOKENS_FILE = 'config/tokens_mawari.json'
SECRETS_FILE = 'config/secrets_mawari.json'
TOKEN_CACHE_FILE = 'config/token_cache_mawari.json'
INVITED_USERS_FILE = 'config/invited_users_mawari.txt'

# ==========================================================
# FUNGSI HELPER (Lengkap)
# ==========================================================
def run_command(command, env=None, input_data=None):
    """Menjalankan perintah shell dan mengembalikan (status, output)."""
    try:
        process = subprocess.run(
            command, shell=True, check=True, capture_output=True,
            text=True, encoding='utf-8', env=env, input_data=input_data
        )
        return (True, process.stdout.strip())
    except subprocess.CalledProcessError as e:
        return (False, f"{e.stdout.strip()} {e.stderr.strip()}")

def load_json_file(filename):
    """Memuat data dari file JSON."""
    try:
        with open(filename, 'r') as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return {}

def save_json_file(filename, data):
    """Menyimpan data ke file JSON."""
    with open(filename, 'w') as f:
        json.dump(data, f, indent=4)

def load_lines_from_file(filename):
    """Memuat baris dari file teks ke dalam sebuah set."""
    try:
        with open(filename, 'r') as f:
            return {line.strip() for line in f if line.strip()}
    except FileNotFoundError:
        return set()

def save_lines_to_file(filename, lines):
    """Menambahkan baris baru ke file teks."""
    with open(filename, 'a') as f:
        for line in lines:
            f.write(f"{line}\n")

def load_setup_config():
    """Memuat konfigurasi utama dari config_setup.json"""
    try:
        with open(CONFIG_FILE, 'r') as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        print(f"❌ FATAL: File '{CONFIG_FILE}' tidak ditemukan atau formatnya salah.")
        print("Pastikan file tersebut ada dan berisi 'main_account_username', 'main_token', dan 'blueprint_repo_name'.")
        sys.exit(1)

# ==========================================================
# FITUR BARU
# ==========================================================
def convert_tokens_from_txt():
    """Opsi 1: Konversi token dari file .txt ke .json."""
    print("\n--- Opsi 1: Konversi Token dari .txt ke .json ---\n")
    try:
        txt_filename = input("Masukkan nama file .txt berisi token (contoh: tokens.txt): ")
        with open(txt_filename, 'r') as f:
            tokens = [line.strip() for line in f if line.strip()]
        
        if not tokens:
            print("⚠️  File .txt kosong atau tidak berisi token yang valid.")
            return

        json_data = {"tokens": tokens}
        save_json_file(TOKENS_FILE, json_data)
        
        print(f"✅ Berhasil! {len(tokens)} token telah dikonversi dari '{txt_filename}' dan disimpan ke '{TOKENS_FILE}'.")

    except FileNotFoundError:
        print(f"❌ GAGAL: File '{txt_filename}' tidak ditemukan.")
    except Exception as e:
        print(f"❌ GAGAL: Terjadi error saat memproses file. Pesan: {e}")


# ==========================================================
# FUNGSI UTAMA LAINNYA
# ==========================================================
def invite_collaborators(config):
    """Opsi 2: Mengundang kolaborator berdasarkan token."""
    print("\n--- Opsi 2: Auto Invite Collaborator & Get Username ---\n")
    tokens_data = load_json_file(TOKENS_FILE)
    if not tokens_data or 'tokens' not in tokens_data:
        print(f"❌ FATAL: {TOKENS_FILE} tidak ditemukan atau formatnya salah. Jalankan Opsi 1 terlebih dahulu."); return

    tokens = tokens_data['tokens']
    token_cache = load_json_file(TOKEN_CACHE_FILE)
    invited_users = load_lines_from_file(INVITED_USERS_FILE)
    print(f"ℹ️  Ditemukan {len(invited_users)} user yang sudah pernah diundang.")
    
    usernames_to_invite = []
    for index, token in enumerate(tokens):
        print(f"\n--- Memproses Token {index + 1}/{len(tokens)} ---")
        username = token_cache.get(token)
        if not username:
            print("   - Memvalidasi token via API...")
            env = os.environ.copy(); env['GH_TOKEN'] = token
            success, result = run_command("gh api user --jq .login", env=env)
            if success:
                username = result; print(f"     ✅ Token valid untuk @{username}"); token_cache[token] = username
            else:
                print(f"     ⚠️  Token tidak valid. Pesan: {result}"); continue
        
        if username and username not in invited_users:
            usernames_to_invite.append(username)
            print(f"   - @{username} adalah user baru yang akan diundang.")
        elif username:
            print(f"   - @{username} sudah ada di daftar undangan (dilewati).")

    save_json_file(TOKEN_CACHE_FILE, token_cache)
    print("\n✅ Cache token-username telah diperbarui.")

    if not usernames_to_invite:
        print("\n✅ Tidak ada user baru untuk diundang."); return

    print(f"\n--- Mengundang {len(usernames_to_invite)} Akun Baru ke Repo ---")
    env = os.environ.copy(); env['GH_TOKEN'] = config['main_token']
    newly_invited = set()

    for username in usernames_to_invite:
        if username.lower() == config['main_account_username'].lower(): continue
        print(f"   - Mengirim undangan ke @{username}...")
        command = f"gh api repos/{config['main_account_username']}/{config['blueprint_repo_name']}/collaborators/{username} -f permission=push --silent"
        success, result = run_command(command, env=env)
        if success:
            print("     ✅ Undangan berhasil dikirim!"); newly_invited.add(username)
        elif "already a collaborator" in result.lower():
            print("     ℹ️  Sudah menjadi kolaborator."); newly_invited.add(username)
        else:
            print(f"     ⚠️  Gagal. Pesan: {result}")
        time.sleep(1)
        
    if newly_invited:
        save_lines_to_file(INVITED_USERS_FILE, newly_invited)
        print(f"\n✅ {len(newly_invited)} user baru berhasil ditambahkan ke tracking file {INVITED_USERS_FILE}.")


def auto_set_secrets(config):
    """Opsi 3: Sinkronisasi secrets ke semua akun."""
    print("\n--- Opsi 3: Auto Set Secrets untuk Mawari ---\n")
    secrets_to_set = load_json_file(SECRETS_FILE)
    if not secrets_to_set:
        print(f"❌ FATAL: {SECRETS_FILE} tidak ditemukan atau kosong."); return
    print(f"✅ Berhasil memuat secrets dari {SECRETS_FILE}.")

    tokens_data = load_json_file(TOKENS_FILE)
    if not tokens_data or 'tokens' not in tokens_data: return
    tokens = tokens_data['tokens']
    
    token_cache = load_json_file(TOKEN_CACHE_FILE)
    if not token_cache:
        print("⚠️ Cache token tidak ditemukan. Jalankan Opsi 2 terlebih dahulu."); return

    for index, token in enumerate(tokens):
        print(f"\n--- Memproses Akun {index + 1}/{len(tokens)} ---")
        username = token_cache.get(token)
        if not username: 
            print("   - Username tidak ada di cache. Jalankan Opsi 2 untuk update. Dilewati."); continue
            
        repo_full_name = f"{username}/{config['blueprint_repo_name']}"
        print(f"   - Target Repositori: {repo_full_name}")

        env = os.environ.copy(); env['GH_TOKEN'] = token
        
        print(f"   - Memeriksa fork...")
        success, _ = run_command(f"gh repo view {repo_full_name}", env=env)
        if not success:
            print(f"     - Fork tidak ditemukan. Membuat fork dari {config['main_account_username']}/{config['blueprint_repo_name']}..."); 
            run_command(f"gh repo fork {config['main_account_username']}/{config['blueprint_repo_name']} --clone=false --remote=false", env=env)
            time.sleep(5)
        else:
            print("     - Fork sudah ada.")

        for name, value in secrets_to_set.items():
            if name.startswith("COMMENT_") or name.startswith("NOTE"): continue
            print(f"   - Mengatur secret '{name}'...")
            command = f'gh secret set {name} --app codespaces --repo "{repo_full_name}"'
            success, result = run_command(command, env=env, input_data=str(value)) # Pastikan value adalah string
            if success: print(f"     ✅ Secret '{name}' berhasil diatur.")
            else: print(f"     ⚠️  Gagal mengatur secret '{name}'. Pesan: {result}")
        time.sleep(1)

def auto_accept_invitations(config):
    """Opsi 4: Menerima undangan kolaborasi."""
    print("\n--- Opsi 4: Auto Accept Collaboration Invitations ---\n")
    tokens_data = load_json_file(TOKENS_FILE)
    if not tokens_data or 'tokens' not in tokens_data: return
    tokens = tokens_data['tokens']
    target_repo = f"{config['main_account_username']}/{config['blueprint_repo_name']}".lower()

    for index, token in enumerate(tokens):
        print(f"\n--- Memproses Akun {index + 1}/{len(tokens)} ---")
        env = os.environ.copy(); env['GH_TOKEN'] = token
        success, username = run_command("gh api user --jq .login", env=env)
        if not success:
            print("   - ⚠️ Token tidak valid, dilewati."); continue
        print(f"   - Login sebagai @{username}")
        print("   - Mengecek undangan...")
        success, invitations_json = run_command("gh api /user/repository_invitations", env=env)
        if not success:
            print("     - ⚠️ Gagal mendapatkan daftar undangan."); continue
        try:
            invitations = json.loads(invitations_json)
            if not invitations:
                print("     - ✅ Tidak ada undangan yang tertunda."); continue
            for inv in invitations:
                inv_id = inv.get("id"); repo_name = inv.get("repository", {}).get("full_name", "").lower()
                if repo_name == target_repo:
                    print(f"     - Ditemukan undangan untuk {repo_name}. Menerima..."); 
                    accept_cmd = f"gh api --method PATCH /user/repository_invitations/{inv_id} --silent"
                    success, result = run_command(accept_cmd, env=env)
                    if success: print("       ✅ Undangan berhasil diterima!")
                    else: print(f"       ⚠️ Gagal menerima undangan. Pesan: {result}")
        except (json.JSONDecodeError, AttributeError):
            print("     - ⚠️ Gagal mem-parsing daftar undangan atau tidak ada undangan.")
        time.sleep(1)

def main():
    """Fungsi utama untuk menjalankan setup tool."""
    config = load_setup_config()
    print(f"✅ Konfigurasi berhasil dimuat untuk repo: {config['blueprint_repo_name']}")

    while True:
        print("\n=============================================")
        print("      MAWARI ORCHESTRATOR SETUP TOOL")
        print("=============================================")
        print("1. [BARU] Konversi Token dari .txt ke .json")
        print("---------------------------------------------")
        print("2. Validasi & Undang Kolaborator Baru")
        print("3. Auto Set Secrets (dengan Pengecekan)")
        print("4. Auto Accept Invitations")
        print("0. Keluar")
        choice = input("Pilih menu (1/2/3/4/0): ")
        if choice == '1': convert_tokens_from_txt()
        elif choice == '2': invite_collaborators(config)
        elif choice == '3': auto_set_secrets(config)
        elif choice == '4': auto_accept_invitations(config)
        elif choice == '0':
            print("Terima kasih!"); break
        else:
            print("Pilihan tidak valid.")
        input("\nTekan Enter untuk kembali ke menu utama...")

if __name__ == "__main__":
    main()
