# Crosswind

**English** · [Bahasa Indonesia](README.id.md)

Research project: send and receive files with an iPhone's built-in AirDrop on devices that aren't Apple, such as Linux, Windows and Android. It comes as a menu bar app with an AirDrop-style edge glow, a command-line tool, and the AWDL and AirDrop libraries underneath.

Crosswind is not affiliated with or endorsed by Apple Inc. AirDrop is a trademark of Apple Inc.

## Status

| Platform | App | Receiving from an iPhone |
|---|---|---|
| macOS (Apple Silicon) | ✅ built and tested | ✅ through the Mac's own AirDrop, with glow and file card · ⚠️ through Crosswind's own receiver: ready, not yet tried with an iPhone |
| macOS (Intel) | not built yet | |
| Linux | code is there, not built yet | needs `awdl0` from `crosswindd` (`crosswind-linux radio up`) and a supported Wi-Fi card; not yet tried |
| Windows | code is there, not built yet | no AWDL route yet; only Crosswind to Crosswind over a normal network |
| Android | not built yet | no AWDL route yet; needs root or an ESP32 dongle |

The AWDL and AirDrop libraries compile for Windows and Android, and the AWDL core compiles for ESP32-C3 (`riscv32imc`, no_std).

## The app

A menu bar app. Closing its window keeps it running.

- **Edge glow:** a full-screen, click-through overlay plays a glow and a sweep from the screen edge when a file comes in, ported from AirPro's Metal shader to WebGL.
- **File card:** the incoming file slides in from the edge with its real thumbnail. QuickLook makes thumbnails for HEIC, PDF and video on macOS. It can be turned off in Settings.
- **The Mac's own AirDrop:** on macOS, files the system's AirDrop saves to Downloads also play the glow. A file counts as AirDrop when its quarantine attribute names `sharingd` and it arrived in the last two minutes, so copies of older AirDrops stay quiet.
- **Crosswind's receiver:** shows up in the iPhone's share sheet as "<computer name> (Crosswind)", with Accept and Decline, or accepts everything if you choose.
- **Menu bar icon:** Open Crosswind, Preview the Glow, Quit Crosswind. No Dock icon on macOS.
- **Launch at login:** opens straight to the menu bar, and turns the receiver back on if it was on.

### Build the app

Requires Rust 1.88 or newer and [Bun](https://bun.sh) for the Tauri CLI:

```sh
cd apps/crosswind-app
bunx @tauri-apps/cli@2 build --bundles app,dmg
```

The app lands in `target/release/bundle/macos/Crosswind.app` and the disk image in `target/release/bundle/dmg/`. Copy the app to `/Applications` before turning on launch at login, so the login item points there. The build is signed ad hoc: other Macs will ask before opening it.

For development, `cargo run -p crosswind-app` runs it straight from the workspace.

## Why this is hard

An iPhone only looks for AirDrop receivers over Bluetooth LE and AWDL, Apple's peer-to-peer Wi-Fi link. Being on the same Wi-Fi network does not help. A receiver must run AWDL, which needs a Wi-Fi card that supports monitor mode and frame injection. macOS has AWDL built in; Linux can run it on supported cards; Windows and Android need outside hardware, such as a USB Wi-Fi adapter through WSL2 or an ESP32 dongle.

## Command line

```sh
cargo build --release
crosswind doctor
crosswind receive --dir ~/Downloads
crosswind browse
crosswind send --to "Jane's iPhone" photo.jpg
crosswind send --addr fe80::1 --iface lo0 photo.jpg
```

`--iface` defaults to `awdl0` on macOS and Linux. On Windows, pass the name of a network adapter. For an iPhone to send to Crosswind's receiver, set its AirDrop to "Everyone for 10 minutes".

On Linux, bring AWDL up first. The daemon and its files install with `packaging/install.sh crosswind`:

```sh
cargo build --release --workspace
sudo packaging/install.sh crosswind
crosswind-linux doctor
crosswind-linux radio up
crosswind-linux receive --dir ~/Downloads
```

## Layout

| Path | Purpose | Platforms |
|---|---|---|
| `apps/crosswind-app` | The Tauri app: overlay glow, file card, tray, launch at login | macOS, Windows, Linux, Android |
| `crates/crosswind-core` | Platform detection, network interfaces, the AWDL routes each platform can try | all |
| `crates/crosswind-awdl` | AWDL: frames, TLVs, election, sync, channel sequence | all, including no_std |
| `crates/crosswind-airdrop` | AirDrop: discovery, request, upload, TLS, mDNS, BLE advert | all |
| `crates/crosswind-cli` | The portable `crosswind` command | all |
| `crates/crosswind-radio` | The Wi-Fi card on Linux: nl80211, monitor mode, injection, driver quirks | Linux |
| `crates/crosswindd` | System service that leases the radio and exposes it over D-Bus | Linux |
| `crates/crosswind-linux` | The `crosswind-linux` command: radio, doctor, test-card, capture, BLE over BlueZ | Linux |

`hardware/` holds the card reports, `tests/captures/` real iPhone captures for replay tests, and `packaging/` the systemd, D-Bus and NetworkManager files.

`cargo build` builds only the crates that run everywhere. Add `--workspace` on Linux for the rest.

## Not supported yet

- Sending from the app. The command line can send.
- AirDrop's Contacts Only mode on Crosswind's receiver.
- Developer ID signing and notarization.

## License

GNU General Public License v3.0 or later. See [LICENSE](LICENSE). Crates marked MIT in their `Cargo.toml` carry their own `LICENSE`.
