# Crosswind

[English](README.md) · **Bahasa Indonesia**

Proyek riset: kirim dan terima file dengan AirDrop bawaan iPhone di perangkat non-Apple, misalnya Linux, Windows, dan Android.

Crosswind tidak berafiliasi dengan dan tidak didukung oleh Apple Inc. AirDrop adalah merek dagang Apple Inc.

## Status

| Platform | Protokol AirDrop | Jalur ke iPhone |
|---|---|---|
| macOS | ✅ jalan antar dua instance Crosswind | `awdl0` sudah ada dari sistem, belum dicoba dengan iPhone |
| Linux | belum terverifikasi, dicek CI | `awdl0` dari `crosswindd` (`crosswind-linux radio up`), dengan iklan BLE lewat BlueZ, belum dicoba |
| Windows | inti AWDL dan AirDrop terkompilasi, CLI belum terverifikasi | belum ada jalur driver WiFi, hanya Crosswind ke Crosswind lewat adapter jaringan biasa |
| Android | inti AWDL dan AirDrop terkompilasi | belum ada jalur driver WiFi |
| ESP32 | inti AWDL terkompilasi (`riscv32imc`, no_std) | dongle belum dibuat |

## Kenapa sulit

iPhone hanya mencari penerima AirDrop lewat Bluetooth LE dan AWDL, jalur WiFi peer-to-peer milik Apple. Berada di jaringan WiFi yang sama tidak membantu. Penerima harus bisa menjalankan AWDL, dan itu butuh kartu WiFi yang mendukung mode monitor dan frame injection.

## Build

Butuh Rust 1.88 atau lebih baru:

```sh
cargo build --release
```

## Cara pakai

```sh
crosswind doctor
crosswind receive --dir ~/Downloads
crosswind browse
crosswind send --to "iPhone Jane" foto.jpg
crosswind send --addr fe80::1 --iface lo0 foto.jpg
```

`--iface` default-nya `awdl0` di macOS dan Linux. Di Windows, isi dengan nama adapter jaringan. Atur AirDrop iPhone ke "Everyone for 10 minutes".

Di Linux, nyalakan AWDL dulu. Daemon dan file pendukungnya dipasang dengan `packaging/install.sh crosswind`:

```sh
cargo build --release --workspace
sudo packaging/install.sh crosswind
crosswind-linux doctor
crosswind-linux radio up
crosswind-linux receive --dir ~/Downloads
```

## Struktur

| Crate | Fungsi | Platform |
|---|---|---|
| `crosswind-core` | Deteksi platform dan jalur AWDL yang bisa dicoba tiap platform | semua |
| `crosswind-awdl` | AWDL: frame, TLV, election, sync, urutan channel | semua, termasuk no_std |
| `crosswind-airdrop` | AirDrop: discovery, request, upload, TLS, mDNS, iklan BLE | semua |
| `crosswind-cli` | Perintah `crosswind` yang portabel | semua |
| `crosswind-radio` | Kartu WiFi di Linux: nl80211, mode monitor, injection, quirk driver | Linux |
| `crosswindd` | Service sistem yang memegang radio dan menyediakannya lewat D-Bus | Linux |
| `crosswind-linux` | Perintah `crosswind-linux`: radio, doctor, test-card, capture, BLE lewat BlueZ | Linux |

Folder `hardware/` berisi laporan kartu WiFi, `tests/captures/` rekaman iPhone asli untuk tes replay, dan `packaging/` file systemd, D-Bus, dan NetworkManager.

`cargo build` hanya membangun crate yang jalan di semua platform. Tambahkan `--workspace` di Linux untuk sisanya.

## Lisensi

GNU General Public License v3.0 atau lebih baru. Lihat [LICENSE](LICENSE). Crate yang bertanda MIT di `Cargo.toml`-nya membawa file `LICENSE` sendiri.
