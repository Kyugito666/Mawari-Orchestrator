#!/bin/bash
# mawari/first-setup.sh - (Versi 12-Wallet, 1 Seed Phrase)

set -e
WORKDIR="/workspaces/Mawari-Orchestrator/mawari"
LOG_FILE="$WORKDIR/setup.log"

exec > >(tee -a "$LOG_FILE") 2>&1

echo "╔════════════════════════════════════════════════╗"
echo "║      MAWARI: 12-WALLET SIMPLIFIED SETUP        ║"
echo "╚════════════════════════════════════════════════╝"
echo "📅 $(date '+%Y-%m-%d %H:%M:%S')"
echo "ℹ️  MODE DETECTED: $SETUP_MODE"

if [[ "$CODESPACE_NAME" != *"mawari-nodes"* ]]; then
    echo "ℹ️  Bukan Codespace Mawari, skrip setup dilewati."
    exit 0
fi

mkdir -p ~/mawari
success_count=0

# --- Membaca semua owner dinamis ke dalam array ---
IFS=',' read -r -a all_owners <<< "$MAWARI_OWNERS"
total_dynamic_wallets=${#all_owners[@]}

if [ -z "$SEED_PHRASE" ] || [ ${total_dynamic_wallets} -lt 11 ]; then
    echo "❌ FATAL: SEED_PHRASE tidak ada atau MAWARI_OWNERS kurang dari 11 alamat."
    exit 1
fi

# =================================================================
# MODE 1: CODESPACE #1 (1 Wallet Utama + 5 Dinamis)
# =================================================================
if [ "$SETUP_MODE" == "PRIMARY" ]; then
    echo "--- Menjalankan Mode PRIMARY (1 Utama + 5 Dinamis) ---"

    # Tahap 1: Node Statis / Utama
    if [ -n "$MAWARI_OWNER_ADDRESS" ] && [ -n "$MAWARI_BURNER_PRIVATE_KEY" ]; then
        wallet_dir=~/mawari/wallet_1
        echo "🔧 Memproses Wallet #1 (Utama)..."
        mkdir -p "$wallet_dir"
        cat > "${wallet_dir}/flohive-cache.json" <<EOF
{ "burnerWallet": { "privateKey": "${MAWARI_BURNER_PRIVATE_KEY}", "address": "${MAWARI_BURNER_ADDRESS}" } }
EOF
        chmod 600 "${wallet_dir}/flohive-cache.json"; success_count=$((success_count + 1))
        echo "   ✅ Config file untuk node utama berhasil dibuat."
    else
        echo "⚠️  WARNING: Secret untuk node utama tidak ditemukan, dilewati."
    fi

    # Tahap 2: 5 Node Dinamis Pertama (Wallet #2 - #6)
    echo "🔧 Memproses 5 wallet dinamis pertama..."
    for i in $(seq 0 4); do
        wallet_index=$(($i + 2))          # Wallet: 2, 3, 4, 5, 6
        derivation_path_index=$i          # Derivasi: 0, 1, 2, 3, 4
        wallet_dir=~/mawari/wallet_${wallet_index}
        
        echo "   -> Memproses Wallet #${wallet_index} (Derivasi: ${derivation_path_index})..."
        mkdir -p "$wallet_dir"
        
        wallet_json=$(node -e "const e=require('ethers');const w=e.Wallet.fromMnemonic('${SEED_PHRASE}',\"m/44'/60'/0'/0/${derivation_path_index}\");console.log(JSON.stringify({a:w.address,p:w.privateKey}));")
        burner_address=$(echo "$wallet_json" | jq -r .a)
        burner_private_key=$(echo "$wallet_json" | jq -r .p)

        cat > "${wallet_dir}/flohive-cache.json" <<EOF
{ "burnerWallet": { "privateKey": "${burner_private_key}", "address": "${burner_address}" } }
EOF
        chmod 600 "${wallet_dir}/flohive-cache.json"; success_count=$((success_count + 1))
        echo "      ✅ Config file created."
    done
fi

# =================================================================
# MODE 2: CODESPACE #2 (6 Wallet Dinamis Berikutnya)
# =================================================================
if [ "$SETUP_MODE" == "SECONDARY" ]; then
    echo "--- Menjalankan Mode SECONDARY (6 Dinamis) ---"
    
    # Tahap 1: 6 Node Dinamis Berikutnya (Wallet #7 - #12)
    echo "🔧 Memproses 6 wallet dinamis berikutnya..."
    for i in $(seq 0 5); do
        wallet_index=$(($i + 7))          # Wallet: 7, 8, 9, 10, 11, 12
        derivation_path_index=$(($i + 5)) # Derivasi: 5, 6, 7, 8, 9, 10
        wallet_dir=~/mawari/wallet_${wallet_index}

        echo "   -> Memproses Wallet #${wallet_index} (Derivasi: ${derivation_path_index})..."
        mkdir -p "$wallet_dir"

        wallet_json=$(node -e "const e=require('ethers');const w=e.Wallet.fromMnemonic('${SEED_PHRASE}',\"m/44'/60'/0'/0/${derivation_path_index}\");console.log(JSON.stringify({a:w.address,p:w.privateKey}));")
        burner_address=$(echo "$wallet_json" | jq -r .a)
        burner_private_key=$(echo "$wallet_json" | jq -r .p)
        
        cat > "${wallet_dir}/flohive-cache.json" <<EOF
{ "burnerWallet": { "privateKey": "${burner_private_key}", "address": "${burner_address}" } }
EOF
        chmod 600 "${wallet_dir}/flohive-cache.json"; success_count=$((success_count + 1))
        echo "      ✅ Config file created."
    done
fi

echo "════════════════════════════════════════════════"
echo "✅ Setup Mawari Selesai! Total ${success_count} wallet dikonfigurasi di codespace ini."
touch /tmp/mawari_first_setup_done
