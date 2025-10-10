# orchestrator/setup_mawari.py

import json
import subprocess
import os
import time
import sys

# --- Nama File Konfigurasi & Data (Path sudah benar) ---
CONFIG_FILE = 'config/config_setup.json'
TOKENS_FILE = 'config/tokens_mawari.json'
SECRETS_FILE = 'config/secrets_mawari.json'
TOKEN_CACHE_FILE = 'config/token_cache_mawari.json'
INVITED_USERS_FILE = 'config/invited_users_mawari.txt'

def run_command(command, env=None, input_data=None):
    try:
        process = subprocess.run(command, shell=True, check=True, capture_output=True, text=True, encoding='utf-8', env=env, input_data=input_data)
        return (True, process.stdout.strip())
    except subprocess.CalledProcessError as e:
        return (False, f"{e.stdout.strip()} {e.stderr.strip()}")
def load_json_file(filename):
    try:
        with open(filename, 'r') as f: return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError): return {}
def save_json_file(filename, data):
    with open(filename, 'w') as f: json.dump(data, f, indent=4)
def load_lines_from_file(filename):
    try:
        with open(filename, 'r') as f: return {line.strip() for line in f if line.strip()}
    except FileNotFoundError: return set()
def save_lines_to_file(filename, lines):
    with open(filename, 'a') as f:
        for line in lines: f.write(f"{line}\n")

def load_setup_config():
    try:
        with open(CONFIG_FILE, 'r') as f: return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        print(f"❌ FATAL: File '{CONFIG_FILE}' tidak ditemukan atau formatnya salah.")
        sys.exit(1)

def invite_collaborators(config):
    print("\n--- Opsi 1: Auto Invite Collaborator & Get Username ---\n")
    tokens_data = load_json_file(TOKENS_FILE)
    if not tokens_data or 'tokens' not in tokens_data:
        print(f"❌ FATAL: {TOKENS_FILE} tidak ditemukan atau formatnya salah."); return
    tokens = tokens_data['tokens']
    token_cache = load_json_file(TOKEN_CACHE_FILE)
    invited_users = load_lines_from_file(INVITED_USERS_FILE)
    usernames_to_invite = []
    for index, token in enumerate(tokens):
        print(f"\n--- Memproses Token {index + 1}/{len(tokens)} ---")
        username = token_cache.get(token)
        if not username:
            env = os.environ.copy(); env['GH_TOKEN'] = token
            success, result = run_command("gh api user --jq .login", env=env)
            if success:
                username = result; print(f"     ✅ Token valid untuk @{username}"); token_cache[token] = username
            else:
                print(f"     ⚠️  Token tidak valid. Pesan: {result}"); continue
        if username and username not in invited_users:
            usernames_to_invite.append(username)
    save_json_file(TOKEN_CACHE_FILE, token_cache)
    if not usernames_to_invite:
        print("\n✅ Tidak ada user baru untuk diundang."); return
    env = os.environ.copy(); env['GH_TOKEN'] = config['main_token']
    newly_invited = set()
    for username in usernames_to_invite:
        if username.lower() == config['main_account_username'].lower(): continue
        command = f"gh api repos/{config['main_account_username']}/{config['blueprint_repo_name']}/collaborators/{username} -f permission=push --silent"
        success, result = run_command(command, env=env)
        if success or "already a collaborator" in result.lower():
            newly_invited.add(username)
        else:
            print(f"     ⚠️  Gagal. Pesan: {result}")
        time.sleep(1)
    if newly_invited:
        save_lines_to_file(INVITED_USERS_FILE, newly_invited)

def auto_set_secrets(config):
    print("\n--- Opsi 2: Auto Set Secrets untuk Mawari ---\n")
    secrets_to_set = load_json_file(SECRETS_FILE)
    if not secrets_to_set:
        print(f"❌ FATAL: {SECRETS_FILE} tidak ditemukan."); return
    tokens_data = load_json_file(TOKENS_FILE)
    tokens = tokens_data.get('tokens', [])
    token_cache = load_json_file(TOKEN_CACHE_FILE)
    for index, token in enumerate(tokens):
        username = token_cache.get(token)
        if not username: continue
        repo_full_name = f"{username}/{config['blueprint_repo_name']}"
        env = os.environ.copy(); env['GH_TOKEN'] = token
        success, _ = run_command(f"gh repo view {repo_full_name}", env=env)
        if not success:
            run_command(f"gh repo fork {config['main_account_username']}/{config['blueprint_repo_name']} --clone=false --remote=false", env=env)
            time.sleep(5)
        for name, value in secrets_to_set.items():
            if name.startswith("COMMENT_") or name.startswith("NOTE"): continue
            command = f'gh secret set {name} --app codespaces --repo "{repo_full_name}"'
            run_command(command, env=env, input_data=str(value))
        time.sleep(1)

def auto_accept_invitations(config):
    print("\n--- Opsi 3: Auto Accept Collaboration Invitations ---\n")
    tokens_data = load_json_file(TOKENS_FILE)
    tokens = tokens_data.get('tokens', [])
    target_repo = f"{config['main_account_username']}/{config['blueprint_repo_name']}".lower()
    for index, token in enumerate(tokens):
        env = os.environ.copy(); env['GH_TOKEN'] = token
        success, username = run_command("gh api user --jq .login", env=env)
        if not success: continue
        success, invitations_json = run_command("gh api /user/repository_invitations", env=env)
        if not success: continue
        try:
            invitations = json.loads(invitations_json)
            for inv in invitations:
                if inv.get("repository", {}).get("full_name", "").lower() == target_repo:
                    accept_cmd = f"gh api --method PATCH /user/repository_invitations/{inv.get('id')} --silent"
                    run_command(accept_cmd, env=env)
        except (json.JSONDecodeError, AttributeError): continue
        time.sleep(1)

def main():
    config = load_setup_config()
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
        elif choice == '0': break
        input("\nTekan Enter untuk kembali ke menu utama...")

if __name__ == "__main__":
    main()
