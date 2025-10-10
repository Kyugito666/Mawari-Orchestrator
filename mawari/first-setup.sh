#!/bin/bash
# mawari/first-setup.sh - (Versi 12-Wallet Terdistribusi)

set -e
WORKDIR="/workspaces/Mawari-Orchestrator/mawari"
LOG_FILE="$WORKDIR/setup.log"

exec > >(tee -a "$LOG_FILE") 2>&1

echo "╔════════════════════════════════════════════════╗"
echo "║     MAWARI: 12-WALLET DISTRIBUTED SETUP        ║"
echo "╚════════════════════════════════════════════════╝"
echo "📅 $(date '+%Y-%m-%d %H:%M:%S')"
echo "ℹ️  MODE DETECTED: $SETUP_MODE"

if [[ "$CODESPACE_NAME" != *"mawari-nodes"* ]]; then
    echo "ℹ️  Bukan Codespace Mawari, skrip setup dilewati."
    exit 0
fi

mkdir -p ~/mawari
success_count=0

# =================================================================
# MODE 1: CODESPACE #1 (1 Wallet Utama + 5 dari Seed Phrase 1)
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
        chmod 600 "${wallet_dir}/flohive-cache.json"
        echo "   ✅ Config file untuk node utama berhasil dibuat."
        success_count=$((success_count + 1))
    else
        echo "⚠️  WARNING: Secret untuk node utama tidak ditemukan, dilewati."
    fi

    # Tahap 2: 5 Node Dinamis dari Seed Phrase 1
    if [ -n "$SEED_PHRASE_1" ] && [ -n "$MAWARI_OWNERS_1" ]; then
        IFS=',' read -r -a owners <<< "$MAWARI_OWNERS_1"
        echo "✅ Terdeteksi ${#owners[@]} owner dari MAWARI_OWNERS_1."

        for i in $(seq 0 $((${#owners[@]} - 1))); do
            wallet_index=$(($i + 2)) # Mulai dari wallet_2
            derivation_path_index=$i # Derivation path: m/.../0, m/.../1, dst.
            owner_address=${owners[$i]}
            wallet_dir=~/mawari/wallet_${wallet_index}
            
            echo "🔧 Memproses Wallet #${wallet_index} (Dinamis, Derivasi: ${derivation_path_index})..."
            mkdir -p "$wallet_dir"
            
            wallet_json=$(node -e "const e=require('ethers');const w=e.Wallet.fromMnemonic('${SEED_PHRASE_1}',\"m/44'/60'/0'/0/${derivation_path_index}\");console.log(JSON.stringify({a:w.address,p:w.privateKey}));")
            burner_address=$(echo "$wallet_json" | jq -r .a)
            burner_private_key=$(echo "$wallet_json" | jq -r .p)

            cat > "${wallet_dir}/flohive-cache.json" <<EOF
{ "burnerWallet": { "privateKey": "${burner_private_key}", "address": "${burner_address}" } }
EOF
            chmod 600 "${wallet_dir}/flohive-cache.json"
            echo "   ✅ Config file created."
            success_count=$((success_count + 1))
        done
    else
        echo "⚠️  WARNING: SEED_PHRASE_1 atau MAWARI_OWNERS_1 tidak ditemukan."
    fi
fi

# =================================================================
# MODE 2: CODESPACE #2 (6 Wallet dari Seed Phrase 2)
# =================================================================
if [ "$SETUP_MODE" == "SECONDARY" ]; then
    echo "--- Menjalankan Mode SECONDARY (6 Dinamis) ---"
    
    if [ -n "$SEED_PHRASE_2" ] && [ -n "$MAWARI_OWNERS_2" ]; then
        IFS=',' read -r -a owners <<< "$MAWARI_OWNERS_2"
        echo "✅ Terdeteksi ${#owners[@]} owner dari MAWARI_OWNERS_2."

        for i in $(seq 0 $((${#owners[@]} - 1))); do
            # MODIFIED: Wallet index dan derivation path dimulai dari offset yang lebih tinggi
            # untuk menghindari konflik dengan codespace pertama.
            wallet_index=$(($i + 7))  # Mulai dari wallet_7, wallet_8, dst.
            derivation_path_index=$(($i + 5)) # Mulai derivasi dari m/.../5, m/.../6, dst.
            owner_address=${owners[$i]}
            wallet_dir=~/mawari/wallet_${wallet_index}

            echo "🔧 Memproses Wallet #${wallet_index} (Dinamis, Derivasi: ${derivation_path_index})..."
            mkdir -p "$wallet_dir"

            wallet_json=$(node -e "const e=require('ethers');const w=e.Wallet.fromMnemonic('${SEED_PHRASE_2}',\"m/44'/60'/0'/0/${derivation_path_index}\");console.log(JSON.stringify({a:w.address,p:w.privateKey}));")
            burner_address=$(echo "$wallet_json" | jq -r .a)
            burner_private_key=$(echo "$wallet_json" | jq -r .p)
            
            cat > "${wallet_dir}/flohive-cache.json" <<EOF
{ "burnerWallet": { "privateKey": "${burner_private_key}", "address": "${burner_address}" } }
EOF
            chmod 600 "${wallet_dir}/flohive-cache.json"
            echo "   ✅ Config file created."
            success_count=$((success_count + 1))
        done
    else
        echo "⚠️  WARNING: SEED_PHRASE_2 atau MAWARI_OWNERS_2 tidak ditemukan."
    fi
fi

echo "════════════════════════════════════════════════"
echo "✅ Setup Mawari Selesai! Total ${success_count} wallet dikonfigurasi di codespace ini."
touch /tmp/mawari_first_setup_done
