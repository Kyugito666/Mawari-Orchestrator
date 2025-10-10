# orchestrator/setup.py

import json
import subprocess
import os
import time
import sys
import base64

# --- Nama File Konfigurasi & Data ---
CONFIG_FILE = 'config/setup.json'
TOKENS_FILE = 'config/tokens.json'
SECRETS_FILE = 'config/secrets.json'
TOKEN_CACHE_FILE = 'config/token_cache.json'
INVITED_USERS_FILE = 'config/invited_users.txt'
ACCEPTED_USERS_FILE = 'config/accepted_users.txt'
SECRETS_SET_FILE = 'config/secrets_set.txt'
STAR_REPOS_FILE = 'star_repos.txt'

# ==========================================================
# FUNGSI HELPER
# ==========================================================
def run_command(command, env=None, input=None, max_retries=3, timeout=30):
    retry_delay = 30
    retry_count = 0
    
    while retry_count < max_retries:
        try:
            process = subprocess.run(
                command, shell=True, check=True, capture_output=True,
                text=True, encoding='utf-8', env=env, input=input, timeout=timeout
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
    try:
        with open(filename, 'r') as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return {}

def save_json_file(filename, data):
    with open(filename, 'w') as f:
        json.dump(data, f, indent=4)

def load_lines_from_file(filename):
    try:
        with open(filename, 'r') as f:
            return {line.strip() for line in f if line.strip()}
    except FileNotFoundError:
        return set()

def save_lines_to_file(filename, lines):
    with open(filename, 'a') as f:
        for line in lines:
            f.write(f"{line}\n")

def load_setup_config():
    try:
        with open(CONFIG_FILE, 'r') as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        print(f"❌ FATAL: File '{CONFIG_FILE}' tidak ditemukan atau formatnya salah.")
        sys.exit(1)

# ==========================================================
# CRYPTO HELPER UNTUK SODIUM ENCRYPTION
# ==========================================================
def encrypt_secret(public_key_b64, secret_value):
    """Encrypt secret menggunakan libsodium (via pynacl)"""
    try:
        from nacl import encoding, public
        
        public_key = public.PublicKey(public_key_b64.encode("utf-8"), encoding.Base64Encoder())
        sealed_box = public.SealedBox(public_key)
        encrypted = sealed_box.encrypt(secret_value.encode("utf-8"))
        return base64.b64encode(encrypted).decode("utf-8")
    except ImportError:
        print("❌ ERROR: Library PyNaCl tidak ditemukan!")
        print("Install dengan: pip install pynacl")
        sys.exit(1)

# ==========================================================
# FITUR-FITUR UTAMA
# ==========================================================

def convert_files_to_json():
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
    print("\n--- Opsi 2: Auto Invite Collaborator & Get Username ---\n")
    tokens_data = load_json_file(TOKENS_FILE)
    if not tokens_data or 'tokens' not in tokens_data:
        print(f"❌ FATAL: {TOKENS_FILE} tidak ditemukan. Jalankan Opsi 1 terlebih dahulu.")
        return

    tokens = tokens_data['tokens']
    token_cache = load_json_file(TOKEN_CACHE_FILE)
    invited_users = load_lines_from_file(INVITED_USERS_FILE)
    usernames_to_invite = []

    for index, token in enumerate(tokens):
        print(f"--- Memvalidasi Token {index + 1}/{len(tokens)} ---")
        username = token_cache.get(token)
        if not username:
            env = os.environ.copy()
            env['GH_TOKEN'] = token
            success, result = run_command("gh api user --jq .login", env=env)
            if success:
                username = result
                print(f"   ✅ Token valid untuk @{username}")
                token_cache[token] = username
            else:
                print(f"   ⚠️  Token tidak valid. Pesan: {result}")
                continue
        
        if username and username.lower() not in (u.lower() for u in invited_users) and username.lower() != config['main_account_username'].lower():
            usernames_to_invite.append(username)

    save_json_file(TOKEN_CACHE_FILE, token_cache)
    
    if not usernames_to_invite:
        print("\n✅ Tidak ada user baru untuk diundang (semua sudah ada di daftar).")
        return

    print(f"\n--- Mengundang {len(usernames_to_invite)} Akun Baru ke Repo ---")
    env = os.environ.copy()
    env['GH_TOKEN'] = config['main_token']
    
    newly_invited = set()

    for idx, username in enumerate(usernames_to_invite, 1):
        print(f"\n[{idx}/{len(usernames_to_invite)}] Memproses @{username}...")
        
        check_endpoint = f"repos/{config['main_account_username']}/{config['blueprint_repo_name']}/collaborators/{username}"
        check_command = f"gh api {check_endpoint}"
        is_collaborator, _ = run_command(check_command, env=env)
        
        if is_collaborator:
            print(f"     ℹ️  @{username} sudah menjadi kolaborator (skip invite).")
            newly_invited.add(username)
            save_lines_to_file(INVITED_USERS_FILE, [username])
            time.sleep(0.5)
            continue
        
        print(f"     📤 Mengirim undangan...")
        invite_endpoint = f"repos/{config['main_account_username']}/{config['blueprint_repo_name']}/collaborators/{username}"
        invite_command = f"gh api --silent -X PUT -f permission='push' {invite_endpoint}"
        success, result = run_command(invite_command, env=env)
        
        if success:
            print(f"     ✅ Undangan berhasil dikirim!")
            newly_invited.add(username)
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
    print("\n--- Opsi 3: Auto Accept Collaboration Invitations ---\n")
    tokens_data = load_json_file(TOKENS_FILE)
    if not tokens_data or 'tokens' not in tokens_data: 
        print(f"❌ FATAL: {TOKENS_FILE} tidak ditemukan.")
        return
    
    tokens = tokens_data.get('tokens', [])
    token_cache = load_json_file(TOKEN_CACHE_FILE)
    accepted_users = load_lines_from_file(ACCEPTED_USERS_FILE)
    target_repo = f"{config['main_account_username']}/{config['blueprint_repo_name']}".lower()
    
    accepted_count = 0
    already_member = 0
    no_invitation = 0
    skipped_count = 0
    
    print(f"🎯 Target Repo: {target_repo}")
    print(f"📋 {len(accepted_users)} user sudah pernah diproses sebelumnya\n")
    
    for index, token in enumerate(tokens):
        username = token_cache.get(token)
        if not username:
            env = os.environ.copy()
            env['GH_TOKEN'] = token
            success, result = run_command("gh api user --jq .login", env=env)
            if not success: 
                print(f"[{index + 1}/{len(tokens)}] ❌ Token tidak valid")
                continue
            username = result
            token_cache[token] = username
        
        if username.lower() in (u.lower() for u in accepted_users):
            print(f"[{index + 1}/{len(tokens)}] ⏭️  @{username} - Sudah diproses sebelumnya (skip)")
            skipped_count += 1
            time.sleep(0.3)
            continue
        
        print(f"[{index + 1}/{len(tokens)}] Memproses @{username}...", end=" ")
        env = os.environ.copy()
        env['GH_TOKEN'] = token
        
        check_endpoint = f"repos/{config['main_account_username']}/{config['blueprint_repo_name']}/collaborators/{username}"
        is_collaborator, _ = run_command(f"gh api {check_endpoint}", env=env)
        
        if is_collaborator:
            print("✅ Sudah menjadi kolaborator")
            already_member += 1
            save_lines_to_file(ACCEPTED_USERS_FILE, [username])
            accepted_users.add(username)
            time.sleep(0.5)
            continue
        
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
                        save_lines_to_file(ACCEPTED_USERS_FILE, [username])
                        accepted_users.add(username)
                    else:
                        print(f"❌ Gagal accept ({accept_result[:30]}...)")
                    break
            
            if not found_invitation:
                print("ℹ️  Tidak ada undangan")
                no_invitation += 1
                
        except (json.JSONDecodeError, AttributeError) as e:
            print(f"❌ Error parsing: {e}")
        
        time.sleep(1)
    
    save_json_file(TOKEN_CACHE_FILE, token_cache)
    
    print(f"\n{'='*50}")
    print(f"📊 Summary:")
    print(f"   ✅ Undangan diterima: {accepted_count}")
    print(f"   👥 Sudah kolaborator: {already_member}")
    print(f"   ⏭️  Dilewati (sudah diproses): {skipped_count}")
    print(f"   ℹ️  Tidak ada undangan: {no_invitation}")
    print(f"{'='*50}")

def auto_set_secrets(config):
    print("\n--- Opsi 4: Auto Set Secrets (Direct API) ---\n")
    
    secrets_to_set = load_json_file(SECRETS_FILE)
    if not secrets_to_set:
        print(f"❌ FATAL: {SECRETS_FILE} tidak ditemukan.")
        return
    
    tokens_data = load_json_file(TOKENS_FILE)
    tokens = tokens_data.get('tokens', [])
    token_cache = load_json_file(TOKEN_CACHE_FILE)
    secrets_set_users = load_lines_from_file(SECRETS_SET_FILE)
    
    actual_secrets = {k: v for k, v in secrets_to_set.items() 
                     if not k.startswith("COMMENT_") and not k.startswith("NOTE") and v}
    
    if not actual_secrets:
        print("❌ Tidak ada secrets valid untuk di-set")
        return
    
    print(f"📝 Akan mengatur {len(actual_secrets)} secrets")
    print(f"🎯 Target repo: {config['main_account_username']}/{config['blueprint_repo_name']}")
    print(f"📋 {len(secrets_set_users)} user sudah diproses\n")
    
    main_owner = config['main_account_username']
    main_repo = config['blueprint_repo_name']
    skipped_count = 0
    
    for index, token in enumerate(tokens):
        username = token_cache.get(token)
        if not username:
            continue
        
        if username.lower() in (u.lower() for u in secrets_set_users):
            print(f"[{index + 1}/{len(tokens)}] ⏭️  @{username} - Skip")
            skipped_count += 1
            time.sleep(0.3)
            continue
        
        print(f"[{index + 1}/{len(tokens)}] @{username}...")
        
        env = os.environ.copy()
        env['GH_TOKEN'] = token
        
        # Get repo ID
        print(f"   🔍 Get repo ID...", end=" ")
        cmd = f'gh api repos/{main_owner}/{main_repo} --jq .id'
        success, repo_id = run_command(cmd, env=env, timeout=15)
        if not success:
            print(f"❌ Gagal: {repo_id[:40]}")
            continue
        print(f"✅ {repo_id}")
        
        # Get public key
        print(f"   🔑 Get public key...", end=" ")
        cmd = f'gh api /user/codespaces/secrets/public-key'
        success, pubkey_json = run_command(cmd, env=env, timeout=15)
        if not success:
            print(f"❌ Gagal: {pubkey_json[:40]}")
            continue
        
        try:
            pubkey_data = json.loads(pubkey_json)
            public_key = pubkey_data['key']
            key_id = pubkey_data['key_id']
            print(f"✅")
        except:
            print(f"❌ Parse gagal")
            continue
        
        print(f"   🔐 Set secrets...")
        success_count = 0
        
        for name, value in actual_secrets.items():
            print(f"      - {name}...", end=" ", flush=True)
            
            try:
                encrypted_value = encrypt_secret(public_key, str(value))
                
                payload = json.dumps({
                    "encrypted_value": encrypted_value,
                    "key_id": key_id,
                    "selected_repository_ids": [int(repo_id)]
                })
                
                cmd = f"gh api --method PUT /user/codespaces/secrets/{name} --input -"
                success, result = run_command(cmd, env=env, input=payload, timeout=15, max_retries=1)
                
                if success or result == "":
                    print("✅")
                    success_count += 1
                else:
                    print(f"❌ {result[:30]}")
                
                time.sleep(0.3)
                
            except Exception as e:
                print(f"❌ {str(e)[:30]}")
        
        print(f"   📊 Berhasil: {success_count}/{len(actual_secrets)}")
        
        if success_count == len(actual_secrets):
            save_lines_to_file(SECRETS_SET_FILE, [username])
            secrets_set_users.add(username)
            print(f"   ✅ Selesai\n")
        else:
            print(f"   ⚠️  Sebagian gagal\n")
        
        time.sleep(1)
    
    print(f"{'='*50}")
    print(f"✅ Selesai!")
    print(f"   📝 Diproses: {len(tokens) - skipped_count}")
    print(f"   ⏭️  Dilewati: {skipped_count}")
    print(f"{'='*50}")

def auto_follow_and_star(config):
    print("\n--- Opsi 5: Auto Follow & Multi-Repo Star ---\n")
    tokens_data = load_json_file(TOKENS_FILE)
    if not tokens_data or 'tokens' not in tokens_data:
        print(f"❌ FATAL: {TOKENS_FILE} tidak ditemukan.")
        return
    tokens = tokens_data['tokens']
    main_user = config['main_account_username']
    print(f"--- 1. Memulai Auto-Follow ke @{main_user} ---")
    for index, token in enumerate(tokens):
        print(f"   - Menggunakan Token {index + 1}/{len(tokens)}...")
        env = os.environ.copy()
        env['GH_TOKEN'] = token
        command = f"gh api --method PUT /user/following/{main_user} --silent"
        run_command(command, env=env)
        time.sleep(1)

    print(f"\n--- 2. Memulai Auto-Star dari {STAR_REPOS_FILE} ---")
    try:
        with open(STAR_REPOS_FILE, 'r') as f:
            repos_to_star = [line.strip() for line in f if line.strip()]
        if not repos_to_star:
            print(f"⚠️  File '{STAR_REPOS_FILE}' kosong.")
            return
    except FileNotFoundError:
        print(f"❌ GAGAL: File '{STAR_REPOS_FILE}' tidak ditemukan.")
        return

    for repo in repos_to_star:
        print(f"\n   - Menargetkan Repositori: {repo}")
        for index, token in enumerate(tokens):
            env = os.environ.copy()
            env['GH_TOKEN'] = token
            command = f"gh repo star {repo}"
            run_command(command, env=env)
            time.sleep(1)

def main():
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
            print("Terima kasih!")
            break
        else:
            print("Pilihan tidak valid.")
        input("\nTekan Enter untuk kembali ke menu utama...")

if __name__ == "__main__":
    main()
