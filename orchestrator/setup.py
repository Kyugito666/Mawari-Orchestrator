# orchestrator/setup.py

import json
import subprocess
import os
import time
import sys

# --- Nama File Konfigurasi & Data ---
CONFIG_FILE = 'config/setup.json'
TOKENS_FILE = 'config/tokens.json'
SECRETS_FILE = 'config/secrets.json'
TOKEN_CACHE_FILE = 'config/token_cache.json'
INVITED_USERS_FILE = 'config/invited_users.txt'
STAR_REPOS_FILE = 'star_repos.txt'

# ==========================================================
# FUNGSI HELPER (Lengkap dan Sudah Diperbaiki)
# ==========================================================
def run_command(command, env=None, input=None):
    """Menjalankan perintah shell dan mengembalikan (status, output)."""
    try:
        process = subprocess.run(
            command, shell=True, check=True, capture_output=True,
            text=True, encoding='utf-8', env=env, input=input
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
    """Memuat konfigurasi utama dari setup.json"""
    try:
        with open(CONFIG_FILE, 'r') as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        print(f"❌ FATAL: File '{CONFIG_FILE}' tidak ditemukan atau formatnya salah.")
        sys.exit(1)

# ==========================================================
# FITUR-FITUR UTAMA
# ==========================================================

def convert_files_to_json():
    """Opsi 1: Konversi file .txt (Token atau Owner) ke format .json."""
    print("\n--- Opsi 1: Konversi Data dari .txt ke .json ---")
    print("1. Konversi Token (dari tokens.txt -> tokens.json)")
    print("2. Konversi Owner Address (dari owners.txt -> secrets.json)")
    choice = input("Pilih jenis konversi (1/2): ")
    if choice == '1':
        try:
            txt_filename = input("Masukkan nama file .txt berisi token (default: tokens.txt): ") or "tokens.txt"
            with open(txt_filename, 'r') as f:
                tokens = [line.strip() for line in f if line.strip().startswith("ghp_")]
            if not tokens:
                print("⚠️  File .txt kosong atau tidak berisi token yang valid.")
                return
            json_data = {"tokens": tokens}
            save_json_file(TOKENS_FILE, json_data)
            print(f"✅ Berhasil! {len(tokens)} token telah dikonversi dari '{txt_filename}' dan disimpan ke '{TOKENS_FILE}'.")
        except FileNotFoundError:
            print(f"❌ GAGAL: File '{txt_filename}' tidak ditemukan di folder 'orchestrator/'.")
        except Exception as e:
            print(f"❌ GAGAL: Terjadi error. Pesan: {e}")
    elif choice == '2':
        try:
            txt_filename = input("Masukkan nama file .txt berisi owner address (default: owners.txt): ") or "owners.txt"
            with open(txt_filename, 'r') as f:
                addresses = [line.strip() for line in f if line.strip().startswith("0x")]
            if not addresses:
                print("⚠️  File .txt kosong atau tidak berisi address yang valid.")
                return
            owners_string = ",".join(addresses)
            secrets_data = load_json_file(SECRETS_FILE)
            if not secrets_data:
                print(f"ℹ️  File '{SECRETS_FILE}' tidak ditemukan, membuat struktur baru.")
                secrets_data = { "MAWARI_OWNERS": "" }
            secrets_data["MAWARI_OWNERS"] = owners_string
            save_json_file(SECRETS_FILE, secrets_data)
            print(f"✅ Berhasil! {len(addresses)} owner address telah digabungkan dan disimpan ke '{SECRETS_FILE}'.")
            print(f"ℹ️  PENTING: Pastikan Anda melengkapi field lain di '{SECRETS_FILE}' secara manual.")
        except FileNotFoundError:
            print(f"❌ GAGAL: File '{txt_filename}' tidak ditemukan di folder 'orchestrator/'.")
        except Exception as e:
            print(f"❌ GAGAL: Terjadi error. Pesan: {e}")
    else:
        print("Pilihan tidak valid.")

def invite_collaborators(config):
    """Opsi 2: Mengundang kolaborator berdasarkan token."""
    print("\n--- Opsi 2: Auto Invite Collaborator & Get Username ---\n")
    tokens_data = load_json_file(TOKENS_FILE)
    if not tokens_data or 'tokens' not in tokens_data:
        print(f"❌ FATAL: {TOKENS_FILE} tidak ditemukan. Jalankan Opsi 1 terlebih dahulu."); return

    tokens = tokens_data['tokens']
    token_cache = load_json_file(TOKEN_CACHE_FILE)
    invited_users = load_lines_from_file(INVITED_USERS_FILE)
    usernames_to_invite = []

    for index, token in enumerate(tokens):
        print(f"--- Memvalidasi Token {index + 1}/{len(tokens)} ---")
        username = token_cache.get(token)
        if not username:
            env = os.environ.copy(); env['GH_TOKEN'] = token
            success, result = run_command("gh api user --jq .login", env=env)
            if success:
                username = result; print(f"   ✅ Token valid untuk @{username}"); token_cache[token] = username
            else:
                print(f"   ⚠️  Token tidak valid. Pesan: {result}"); continue
        
        if username and username.lower() not in (u.lower() for u in invited_users) and username.lower() != config['main_account_username'].lower():
            usernames_to_invite.append(username)

    save_json_file(TOKEN_CACHE_FILE, token_cache)
    
    if not usernames_to_invite:
        print("\n✅ Tidak ada user baru untuk diundang (semua sudah ada di daftar)."); return

    print(f"\n--- Mengundang {len(usernames_to_invite)} Akun Baru ke Repo ---")
    env = os.environ.copy(); env['GH_TOKEN'] = config['main_token']
    
    newly_invited = set()

    for username in usernames_to_invite:
        print(f"   - Mengirim undangan ke @{username}...")
        # ==========================================================
        # PERBAIKAN FINAL BERDASARKAN CONTOH LO
        # ==========================================================
        endpoint = f"repos/{config['main_account_username']}/{config['blueprint_repo_name']}/collaborators/{username}"
        command = f"gh api --silent -X PUT -f permission='push' {endpoint}"
        success, result = run_command(command, env=env)
        
        if success:
            print(f"     ✅ Undangan untuk @{username} berhasil dikirim!")
            newly_invited.add(username)
        else:
            if "already a collaborator" in result.lower():
                print(f"     ℹ️  @{username} sudah menjadi kolaborator.")
                newly_invited.add(username)
            else:
                print(f"     ❌ GAGAL mengirim undangan ke @{username}. Pesan Error: {result}")
        time.sleep(1)
        
    if newly_invited:
        save_lines_to_file(INVITED_USERS_FILE, newly_invited)
        print(f"\n✅ {len(newly_invited)} user baru berhasil diproses dan ditambahkan ke daftar undangan.")

def auto_accept_invitations(config):
    """Opsi 3: Menerima undangan kolaborasi."""
    print("\n--- Opsi 3: Auto Accept Collaboration Invitations ---\n")
    tokens_data = load_json_file(TOKENS_FILE)
    if not tokens_data or 'tokens' not in tokens_data: return
    tokens = tokens_data.get('tokens', [])
    target_repo = f"{config['main_account_username']}/{config['blueprint_repo_name']}".lower()
    for index, token in enumerate(tokens):
        env = os.environ.copy(); env['GH_TOKEN'] = token
        success, username = run_command("gh api user --jq .login", env=env)
        if not success: continue
        print(f"--- Memproses Akun @{username} ({index + 1}/{len(tokens)}) ---")
        success, invitations_json = run_command("gh api /user/repository_invitations", env=env)
        if not success: continue
        try:
            invitations = json.loads(invitations_json)
            for inv in invitations:
                if inv.get("repository", {}).get("full_name", "").lower() == target_repo:
                    accept_cmd = f"gh api --method PATCH /user/repository_invitations/{inv.get('id')} --silent"
                    run_command(accept_cmd, env=env)
                    print(f"   ✅ Undangan untuk {target_repo} diterima.")
        except (json.JSONDecodeError, AttributeError): continue
        time.sleep(1)

def auto_set_secrets(config):
    """Opsi 4: Sinkronisasi secrets ke semua akun."""
    print("\n--- Opsi 4: Auto Set Secrets ---\n")
    secrets_to_set = load_json_file(SECRETS_FILE)
    if not secrets_to_set:
        print(f"❌ FATAL: {SECRETS_FILE} tidak ditemukan."); return
    tokens_data = load_json_file(TOKENS_FILE)
    tokens = tokens_data.get('tokens', [])
    token_cache = load_json_file(TOKEN_CACHE_FILE)
    for index, token in enumerate(tokens):
        username = token_cache.get(token)
        if not username: continue
        print(f"\n--- Memproses Akun @{username} ({index + 1}/{len(tokens)}) ---")
        repo_full_name = f"{username}/{config['blueprint_repo_name']}"
        env = os.environ.copy(); env['GH_TOKEN'] = token
        success, _ = run_command(f"gh repo view {repo_full_name}", env=env)
        if not success:
            print(f"   - Fork tidak ditemukan. Membuat fork...")
            run_command(f"gh repo fork {config['main_account_username']}/{config['blueprint_repo_name']} --clone=false --remote=false", env=env)
            time.sleep(5)
        for name, value in secrets_to_set.items():
            if name.startswith("COMMENT_") or name.startswith("NOTE"): continue
            print(f"   - Mengatur secret '{name}'...")
            command = f'gh secret set {name} --app codespaces --repo "{repo_full_name}"'
            run_command(command, env=env, input=str(value))
        time.sleep(1)

def auto_follow_and_star(config):
    """Opsi 5: Follow akun utama dan star repositori dari daftar."""
    print("\n--- Opsi 5: Auto Follow & Multi-Repo Star ---\n")
    tokens_data = load_json_file(TOKENS_FILE)
    if not tokens_data or 'tokens' not in tokens_data:
        print(f"❌ FATAL: {TOKENS_FILE} tidak ditemukan."); return
    tokens = tokens_data['tokens']
    main_user = config['main_account_username']
    print(f"--- 1. Memulai Auto-Follow ke @{main_user} ---")
    for index, token in enumerate(tokens):
        print(f"   - Menggunakan Token {index + 1}/{len(tokens)}...")
        env = os.environ.copy(); env['GH_TOKEN'] = token
        command = f"gh api --method PUT /user/following/{main_user} --silent"
        run_command(command, env=env)
        time.sleep(1)

    print(f"\n--- 2. Memulai Auto-Star dari {STAR_REPOS_FILE} ---")
    try:
        with open(STAR_REPOS_FILE, 'r') as f:
            repos_to_star = [line.strip() for line in f if line.strip()]
        if not repos_to_star:
            print(f"⚠️  File '{STAR_REPOS_FILE}' kosong."); return
    except FileNotFoundError:
        print(f"❌ GAGAL: File '{STAR_REPOS_FILE}' tidak ditemukan."); return

    for repo in repos_to_star:
        print(f"\n   - Menargetkan Repositori: {repo}")
        for index, token in enumerate(tokens):
            env = os.environ.copy(); env['GH_TOKEN'] = token
            command = f"gh repo star {repo}"
            run_command(command, env=env)
            time.sleep(1)

def main():
    """Fungsi utama untuk menjalankan setup tool."""
    config = load_setup_config()
    print(f"✅ Konfigurasi berhasil dimuat untuk repo: {config['blueprint_repo_name']}")
    while True:
        print("\n=============================================")
        print("         ORCHESTRATOR SETUP TOOL")
        print("=============================================")
        print("1. Konversi dari .txt ke .json (Token/Owner)")
        print("2. Validasi & Undang Kolaborator Baru")
        print("3. Auto Accept Invitations (Jalankan setelah menu 2)")
        print("4. Auto Set Secrets (Jalankan setelah menu 3)")
        print("5. Auto Follow Akun Utama & Star Repositori")
        print("---------------------------------------------")
        print("0. Keluar")
        choice = input("Pilih menu (1/2/3/4/5/0): ")
        if choice == '1': convert_files_to_json()
        elif choice == '2': invite_collaborators(config)
        elif choice == '3': auto_accept_invitations(config)
        elif choice == '4': auto_set_secrets(config)
        elif choice == '5': auto_follow_and_star(config)
        elif choice == '0':
            print("Terima kasih!"); break
        else:
            print("Pilihan tidak valid.")
        input("\nTekan Enter untuk kembali ke menu utama...")

if __name__ == "__main__":
    main()
