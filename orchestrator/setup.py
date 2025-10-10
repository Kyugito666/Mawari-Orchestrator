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
# FUNGSI HELPER (Lengkap dengan Retry Koneksi)
# ==========================================================
def run_command(command, env=None, input=None, max_retries=3):
    """Menjalankan perintah dengan mekanisme retry untuk masalah koneksi."""
    retry_delay = 30  # Jeda dalam detik sebelum mencoba lagi
    retry_count = 0
    
    while retry_count < max_retries:
        try:
            process = subprocess.run(
                command, shell=True, check=True, capture_output=True,
                text=True, encoding='utf-8', env=env, input=input, timeout=30
            )
            return (True, process.stdout.strip())
        except subprocess.TimeoutExpired:
            retry_count += 1
            if retry_count < max_retries:
                print(f"     ⏱️  TIMEOUT. Percobaan {retry_count}/{max_retries}. Mencoba lagi dalam {retry_delay} detik...")
                time.sleep(retry_delay)
                continue
            else:
                return (False, "Command timeout after multiple retries")
        except subprocess.CalledProcessError as e:
            error_message = f"{e.stdout.strip()} {e.stderr.strip()}".lower()
            
            # Cek apakah error karena koneksi
            connection_errors = [
                "connecting to api.github.com",
                "could not resolve host",
                "tls handshake timeout",
                "connection reset",
                "connection timed out",
                "network is unreachable",
                "temporary failure in name resolution"
            ]
            
            is_connection_error = any(err in error_message for err in connection_errors)
            
            if is_connection_error:
                retry_count += 1
                if retry_count < max_retries:
                    print(f"     ❌ KONEKSI GAGAL. Percobaan {retry_count}/{max_retries}. Mencoba lagi dalam {retry_delay} detik...")
                    time.sleep(retry_delay)
                    print("     🔄 Mencoba ulang perintah...")
                    continue
                else:
                    return (False, f"Connection failed after {max_retries} retries: {error_message}")
            else:
                return (False, error_message.strip())
    
    return (False, "Max retries exceeded")

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

    # Validasi dan kumpulkan username
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

    for idx, username in enumerate(usernames_to_invite, 1):
        print(f"\n[{idx}/{len(usernames_to_invite)}] Memproses @{username}...")
        
        # Cek status kolaborator terlebih dahulu
        check_endpoint = f"repos/{config['main_account_username']}/{config['blueprint_repo_name']}/collaborators/{username}"
        check_command = f"gh api {check_endpoint}"
        is_collaborator, _ = run_command(check_command, env=env)
        
        if is_collaborator:
            print(f"     ℹ️  @{username} sudah menjadi kolaborator (skip invite).")
            newly_invited.add(username)
            # Langsung simpan ke file
            save_lines_to_file(INVITED_USERS_FILE, [username])
            time.sleep(0.5)
            continue
        
        # Kirim undangan
        print(f"     📤 Mengirim undangan...")
        invite_endpoint = f"repos/{config['main_account_username']}/{config['blueprint_repo_name']}/collaborators/{username}"
        invite_command = f"gh api --silent -X PUT -f permission='push' {invite_endpoint}"
        success, result = run_command(invite_command, env=env)
        
        if success:
            print(f"     ✅ Undangan berhasil dikirim!")
            newly_invited.add(username)
            # Langsung simpan ke file setelah berhasil
            save_lines_to_file(INVITED_USERS_FILE, [username])
        else:
            if "already a collaborator" in result.lower():
                print(f"     ℹ️  @{username} sudah menjadi kolaborator.")
                newly_invited.add(username)
                save_lines_to_file(INVITED_USERS_FILE, [username])
            else:
                print(f"     ❌ GAGAL mengirim undangan. Pesan Error: {result}")
                print(f"     ⚠️  User @{username} akan dilewati dan tidak ditambahkan ke daftar.")
        
        time.sleep(1)
        
    print(f"\n{'='*50}")
    print(f"✅ Proses selesai! {len(newly_invited)} user berhasil diproses.")
    print(f"{'='*50}")

def auto_accept_invitations(config):
    """Opsi 3: Menerima undangan kolaborasi."""
    print("\n--- Opsi 3: Auto Accept Collaboration Invitations ---\n")
    tokens_data = load_json_file(TOKENS_FILE)
    if not tokens_data or 'tokens' not in tokens_data: 
        print(f"❌ FATAL: {TOKENS_FILE} tidak ditemukan."); return
    
    tokens = tokens_data.get('tokens', [])
    target_repo = f"{config['main_account_username']}/{config['blueprint_repo_name']}".lower()
    
    accepted_count = 0
    already_member = 0
    no_invitation = 0
    
    print(f"🎯 Target Repo: {target_repo}\n")
    
    for index, token in enumerate(tokens):
        env = os.environ.copy(); env['GH_TOKEN'] = token
        success, username = run_command("gh api user --jq .login", env=env)
        if not success: 
            print(f"[{index + 1}/{len(tokens)}] ❌ Token tidak valid")
            continue
        
        print(f"[{index + 1}/{len(tokens)}] Memproses @{username}...", end=" ")
        
        # Cek apakah sudah menjadi kolaborator
        check_endpoint = f"repos/{config['main_account_username']}/{config['blueprint_repo_name']}/collaborators/{username}"
        is_collaborator, _ = run_command(f"gh api {check_endpoint}", env=env)
        
        if is_collaborator:
            print("✅ Sudah menjadi kolaborator")
            already_member += 1
            time.sleep(0.5)
            continue
        
        # Ambil daftar undangan
        success, invitations_json = run_command("gh api /user/repository_invitations", env=env)
        if not success: 
            print(f"❌ Gagal mengambil undangan")
            continue
        
        try:
            invitations = json.loads(invitations_json)
            found_invitation = False
            
            for inv in invitations:
                if inv.get("repository", {}).get("full_name", "").lower() == target_repo:
                    found_invitation = True
                    invitation_id = inv.get('id')
                    accept_cmd = f"gh api --method PATCH /user/repository_invitations/{invitation_id} --silent"
                    accept_success, accept_result = run_command(accept_cmd, env=env)
                    
                    if accept_success:
                        print("✅ Undangan diterima!")
                        accepted_count += 1
                    else:
                        print(f"❌ Gagal accept ({accept_result[:30]}...)")
                    break
            
            if not found_invitation:
                print("ℹ️  Tidak ada undangan")
                no_invitation += 1
                
        except (json.JSONDecodeError, AttributeError) as e:
            print(f"❌ Error parsing: {e}")
        
        time.sleep(1)
    
    print(f"\n{'='*50}")
    print(f"📊 Summary:")
    print(f"   ✅ Undangan diterima: {accepted_count}")
    print(f"   👥 Sudah kolaborator: {already_member}")
    print(f"   ℹ️  Tidak ada undangan: {no_invitation}")
    print(f"{'='*50}")

def auto_set_secrets(config):
    """Opsi 4: Sinkronisasi secrets ke semua akun."""
    print("\n--- Opsi 4: Auto Set Secrets (User Codespaces Secrets) ---\n")
    secrets_to_set = load_json_file(SECRETS_FILE)
    if not secrets_to_set:
        print(f"❌ FATAL: {SECRETS_FILE} tidak ditemukan."); return
    tokens_data = load_json_file(TOKENS_FILE)
    tokens = tokens_data.get('tokens', [])
    token_cache = load_json_file(TOKEN_CACHE_FILE)
    
    # Hitung jumlah secrets yang akan diatur (exclude COMMENT_ dan NOTE)
    actual_secrets = {k: v for k, v in secrets_to_set.items() if not k.startswith("COMMENT_") and not k.startswith("NOTE")}
    
    print(f"📝 Akan mengatur {len(actual_secrets)} secrets ke user settings setiap akun")
    print(f"🎯 Target repo: {config['main_account_username']}/{config['blueprint_repo_name']}\n")
    
    # Get repo ID untuk selected repositories
    main_repo = f"{config['main_account_username']}/{config['blueprint_repo_name']}"
    
    for index, token in enumerate(tokens):
        username = token_cache.get(token)
        if not username: continue
        print(f"[{index + 1}/{len(tokens)}] Memproses @{username}...")
        
        env = os.environ.copy(); env['GH_TOKEN'] = token
        
        # Dapatkan repository ID
        print(f"   🔍 Mendapatkan repo ID...", end=" ")
        repo_id_cmd = f"gh api repos/{main_repo} --jq .id"
        success, repo_id = run_command(repo_id_cmd, env=env)
        
        if not success:
            print(f"❌ Gagal mendapatkan repo ID: {repo_id[:50]}...")
            continue
        print(f"✅ ID: {repo_id}")
        
        # Set secrets ke USER CODESPACES (bukan repo secrets)
        print(f"   🔐 Mengatur user codespaces secrets...")
        success_count = 0
        
        for name, value in actual_secrets.items():
            print(f"      - {name}...", end=" ")
            
            # Set secret di user level dengan selected repo
            # Step 1: Set the secret value
            set_secret_cmd = f'gh api -X PUT /user/codespaces/secrets/{name} -f visibility=selected'
            success, result = run_command(set_secret_cmd, env=env, input=str(value))
            
            if not success:
                print(f"❌ ({result[:40]}...)")
                continue
            
            # Step 2: Add repository access to the secret
            add_repo_cmd = f'gh api -X PUT /user/codespaces/secrets/{name}/repositories/{repo_id}'
            success, result = run_command(add_repo_cmd, env=env)
            
            if success:
                print("✅")
                success_count += 1
            else:
                print(f"⚠️ Secret dibuat tapi gagal add repo access")
        
        print(f"   📊 Berhasil: {success_count}/{len(actual_secrets)} secrets\n")
        time.sleep(1)
    
    print(f"{'='*50}")
    print(f"✅ Proses selesai untuk {len(tokens)} akun")
    print(f"{'='*50}")

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
