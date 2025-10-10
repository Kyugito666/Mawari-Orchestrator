# Mawari-Orchestrator (12-Node Distributed Edition)

Ini adalah *blueprint* terpadu yang dirancang khusus untuk menjalankan **12 node Mawari** secara otomatis dan terdistribusi di dua GitHub Codespace terpisah. Proyek ini dilengkapi dengan orkestrator cerdas yang mengelola rotasi akun, monitoring kuota, *health check*, dan *keep-alive* otomatis.

---

## 🎯 Fitur Utama

- ✅ **Multi-Account Rotation**: Otomatis beralih antar akun GitHub saat kuota habis.
- ✅ **12-Node Distributed Setup**: Menjalankan 6 node di Codespace pertama dan 6 node di Codespace kedua.
- ✅ **Hybrid Wallet Support**: Mendukung node utama (statis) dan node dinamis (dari *seed phrase*).
- ✅ **Dual Codespace Management**: Orkestrator secara cerdas membuat, memonitor, dan menjaga kedua Codespace tetap aktif.
- ✅ **Auto Keep-Alive**: Menjaga semua node tetap aktif dengan siklus 4 jam.
- ✅ **Billing Monitor**: Melacak penggunaan kuota Codespace untuk rotasi yang efisien.
- ✅ **Automated Setup Tools**: Dilengkapi *script* Python untuk mempermudah manajemen kolaborator dan sinkronisasi *secrets*.

---

## 🚀 Cara Penggunaan

### Tahap 1: Persiapan Awal (Lokal)

1.  **Fork & Clone**: Fork repositori ini ke akun GitHub utama Anda, lalu clone ke komputer lokal.
2.  **Masuk ke Folder Orkestrator**: Buka terminal atau Command Prompt dan navigasi ke folder `orchestrator/`.
3.  **Buat File `config_setup.json`**:
    - Salin `config_setup.json.template` menjadi `config_setup.json`.
    - Edit file tersebut dan isi dengan data Anda:
      ```json
      {
        "main_account_username": "YourGitHubUsername",
        "main_token": "ghp_TokenAkunUtama...",
        "blueprint_repo_name": "Mawari-Orchestrator"
      }
      ```
4.  **Buat File `tokens_mawari.json`**:
    - Salin `tokens_mawari.json.template` (jika ada) atau buat file baru.
    - Isi dengan semua token GitHub (PAT Classic) yang akan Anda gunakan untuk rotasi.
      ```json
      {
        "tokens": [
          "ghp_TokenAkun1...",
          "ghp_TokenAkun2...",
          "ghp_TokenAkun3..."
        ]
      }
      ```
5.  **Buat File `secrets_mawari.json`**:
    - Salin `secrets_mawari.json.template` menjadi `secrets_mawari.json`.
    - **Isi dengan sangat teliti** sesuai panduan di dalam file tersebut untuk 12 wallet Anda.

### Tahap 2: Jalankan Setup Tool (Python)

Setup tool ini akan mengotomatiskan persiapan multi-akun.

```bash
# Di dalam folder orchestrator/
python setup_mawari.py
```

Menu yang tersedia:
- **[1] Validasi & Undang Kolaborator**: Otomatis mengundang semua akun dari `tokens_mawari.json` ke repositori.
- **[2] Auto Set Secrets**: Sinkronisasi semua *secret* dari `secrets_mawari.json` ke setiap akun.
- **[3] Auto Accept Invitations**: Otomatis menerima undangan kolaborasi untuk semua akun.

**Jalankan menu 1 → 3 → 2 secara berurutan!**

### Tahap 3: Jalankan Orkestrator

#### Windows:

1.  Edit `start_mawari.bat`, ubah `ORCHESTRATOR_PATH` ke lokasi folder `orchestrator/` Anda.
2.  Jalankan file tersebut:
    ```cmd
    start_mawari.bat
    ```

#### Linux/Mac/Termux:

```bash
# Di dalam folder orchestrator/
cargo run --release -- YourGitHubUsername/Mawari-Orchestrator
```

---

## 📊 Monitoring

- **Windows**: Edit `monitor_mawari.bat` dan jalankan untuk melihat status *real-time*.
- **Manual**: Gunakan perintah `gh cs list` untuk melihat Codespace yang aktif dan `gh cs ssh -c <nama-cs>` untuk masuk ke dalamnya.

---
