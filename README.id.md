# Crosswind

[English](README.md) · **Bahasa Indonesia**

Proyek riset: kirim dan terima file dengan AirDrop bawaan iPhone di perangkat non-Apple, misalnya Linux, Windows, dan Android. Isinya aplikasi menu bar dengan efek glow di tepi layar ala AirDrop, alat command line, serta pustaka AWDL dan AirDrop di bawahnya.

Crosswind tidak berafiliasi dengan dan tidak didukung oleh Apple Inc. AirDrop adalah merek dagang Apple Inc.

## Status

| Platform | Aplikasi | Menerima dari iPhone |
|---|---|---|
| macOS (Apple Silicon) | ✅ sudah di-build dan dites | ✅ lewat AirDrop bawaan Mac, dengan glow dan kartu file · ⚠️ lewat penerima Crosswind sendiri: siap, belum dicoba dengan iPhone |
| macOS (Intel) | belum di-build | |
| Linux | kode ada, belum di-build | butuh `awdl0` dari `crosswindd` (`crosswind-linux radio up`) dan kartu WiFi yang didukung, belum dicoba |
| Windows | kode ada, belum di-build | belum ada jalur AWDL, hanya Crosswind ke Crosswind lewat jaringan biasa |
| Android | belum di-build | belum ada jalur AWDL, butuh root atau dongle ESP32 |

Pustaka AWDL dan AirDrop sudah terkompilasi untuk Windows dan Android, dan inti AWDL terkompilasi untuk ESP32-C3 (`riscv32imc`, no_std).

## Aplikasi

Aplikasi menu bar. Menutup jendelanya tidak menghentikan aplikasi.

- **Glow di tepi layar:** overlay layar penuh yang tidak menghalangi klik memutar glow dan gelombang dari tepi layar saat ada file masuk. Shader Metal dari AirPro di-port ke WebGL.
- **Kartu file:** file yang masuk meluncur dari tepi lengkap dengan thumbnail aslinya. Di macOS, QuickLook membuatkan thumbnail untuk HEIC, PDF, dan video. Bisa dimatikan di Settings.
- **AirDrop bawaan Mac:** di macOS, file yang disimpan AirDrop bawaan ke Downloads juga memutar glow. File dianggap kiriman AirDrop kalau atribut karantinanya mencatat `sharingd` dan diterima dalam 2 menit terakhir, jadi salinan file AirDrop lama tidak memicu animasi.
- **Penerima Crosswind:** muncul di share sheet iPhone sebagai "<nama komputer> (Crosswind)", dengan tombol Accept dan Decline, atau terima otomatis kalau diaktifkan.
- **Ikon menu bar:** Open Crosswind, Preview the Glow, Quit Crosswind. Tanpa ikon Dock di macOS.
- **Buka saat login:** langsung ke menu bar, dan menyalakan lagi penerima kalau terakhir kali aktif.

### Build aplikasi

Butuh Rust 1.88 atau lebih baru dan [Bun](https://bun.sh) untuk Tauri CLI:

```sh
cd apps/crosswind-app
bunx @tauri-apps/cli@2 build --bundles app,dmg
```

Aplikasinya ada di `target/release/bundle/macos/Crosswind.app`, dan disk image di `target/release/bundle/dmg/`. Salin aplikasinya ke `/Applications` sebelum menyalakan opsi buka saat login, supaya item login menunjuk ke sana. Build ini ditandatangani ad hoc, jadi Mac lain akan meminta konfirmasi sebelum membukanya.

Untuk pengembangan, `cargo run -p crosswind-app` menjalankannya langsung dari workspace.

## Kenapa sulit

iPhone hanya mencari penerima AirDrop lewat Bluetooth LE dan AWDL, jalur WiFi peer-to-peer milik Apple. Berada di jaringan WiFi yang sama tidak membantu. Penerima harus bisa menjalankan AWDL, dan itu butuh kartu WiFi yang mendukung mode monitor dan frame injection. macOS sudah punya AWDL, Linux bisa menjalankannya di kartu yang didukung, sedangkan Windows dan Android butuh hardware tambahan, misalnya adapter WiFi USB lewat WSL2 atau dongle ESP32.

## Command line

```sh
cargo build --release
crosswind doctor
crosswind receive --dir ~/Downloads
crosswind browse
crosswind send --to "iPhone Jane" foto.jpg
crosswind send --addr fe80::1 --iface lo0 foto.jpg
```

`--iface` default-nya `awdl0` di macOS dan Linux. Di Windows, isi dengan nama adapter jaringan. Supaya iPhone bisa mengirim ke penerima Crosswind, atur AirDrop iPhone ke "Everyone for 10 minutes".

Di Linux, nyalakan AWDL dulu. Daemon dan file pendukungnya dipasang dengan `packaging/install.sh crosswind`:

```sh
cargo build --release --workspace
sudo packaging/install.sh crosswind
crosswind-linux doctor
crosswind-linux radio up
crosswind-linux receive --dir ~/Downloads
```

## Struktur

| Path | Fungsi | Platform |
|---|---|---|
| `apps/crosswind-app` | Aplikasi Tauri: glow overlay, kartu file, tray, buka saat login | macOS, Windows, Linux, Android |
| `crates/crosswind-core` | Deteksi platform, interface jaringan, jalur AWDL yang bisa dicoba tiap platform | semua |
| `crates/crosswind-awdl` | AWDL: frame, TLV, election, sync, urutan channel | semua, termasuk no_std |
| `crates/crosswind-airdrop` | AirDrop: discovery, request, upload, TLS, mDNS, iklan BLE | semua |
| `crates/crosswind-cli` | Perintah `crosswind` yang portabel | semua |
| `crates/crosswind-radio` | Kartu WiFi di Linux: nl80211, mode monitor, injection, quirk driver | Linux |
| `crates/crosswindd` | Service sistem yang memegang radio dan menyediakannya lewat D-Bus | Linux |
| `crates/crosswind-linux` | Perintah `crosswind-linux`: radio, doctor, test-card, capture, BLE lewat BlueZ | Linux |

Folder `hardware/` berisi laporan kartu WiFi, `tests/captures/` rekaman iPhone asli untuk tes replay, dan `packaging/` file systemd, D-Bus, dan NetworkManager.

`cargo build` hanya membangun crate yang jalan di semua platform. Tambahkan `--workspace` di Linux untuk sisanya.

## Belum didukung

- Mengirim dari aplikasi. Pengiriman sudah bisa lewat command line.
- Mode Contacts Only AirDrop pada penerima Crosswind.
- Tanda tangan Developer ID dan notarisasi.

## Lisensi

GNU General Public License v3.0 atau lebih baru. Lihat [LICENSE](LICENSE). Crate yang bertanda MIT di `Cargo.toml`-nya membawa file `LICENSE` sendiri.
