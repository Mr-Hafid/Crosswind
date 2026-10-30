# Crosswind

**English** · [Bahasa Indonesia](README.id.md)

Research project: send and receive files with an iPhone's built-in AirDrop on devices that aren't Apple, such as Linux, Windows and Android.

Crosswind is not affiliated with or endorsed by Apple Inc. AirDrop is a trademark of Apple Inc.

## Status

| Platform | AirDrop protocol | Link to an iPhone |
|---|---|---|
| macOS | ✅ works between two Crosswind instances | `awdl0` is built in; not yet tried with an iPhone |
| Linux | not verified yet, CI builds it | `awdl0` from `crosswindd` (`crosswind-linux radio up`), with a BLE advert over BlueZ; not yet tried |
| Windows | AWDL and AirDrop cores compile; CLI not verified yet | no Wi-Fi driver route yet; only Crosswind to Crosswind over a normal network adapter |
| Android | AWDL and AirDrop cores compile | no Wi-Fi driver route yet |
| ESP32 | AWDL core compiles (`riscv32imc`, no_std) | dongle not built yet |

## Why this is hard

An iPhone only looks for AirDrop receivers over Bluetooth LE and AWDL, Apple's peer-to-peer Wi-Fi link. Being on the same Wi-Fi network does not help. A receiver must run AWDL, which needs a Wi-Fi card that supports monitor mode and frame injection.

## Build

Requires Rust 1.88 or newer:

```sh
cargo build --release
```

## Usage

```sh
crosswind doctor
crosswind receive --dir ~/Downloads
crosswind browse
crosswind send --to "Jane's iPhone" photo.jpg
crosswind send --addr fe80::1 --iface lo0 photo.jpg
```

`--iface` defaults to `awdl0` on macOS and Linux. On Windows, pass the name of a network adapter. Set the iPhone's AirDrop to "Everyone for 10 minutes".

On Linux, bring AWDL up first. The daemon and its files install with `packaging/install.sh crosswind`:

```sh
cargo build --release --workspace
sudo packaging/install.sh crosswind
crosswind-linux doctor
crosswind-linux radio up
crosswind-linux receive --dir ~/Downloads
```

## Layout

| Crate | Purpose | Platforms |
|---|---|---|
| `crosswind-core` | Platform detection and the AWDL routes each platform can try | all |
| `crosswind-awdl` | AWDL: frames, TLVs, election, sync, channel sequence | all, including no_std |
| `crosswind-airdrop` | AirDrop: discovery, request, upload, TLS, mDNS, BLE advert | all |
| `crosswind-cli` | The portable `crosswind` command | all |
| `crosswind-radio` | The Wi-Fi card on Linux: nl80211, monitor mode, injection, driver quirks | Linux |
| `crosswindd` | System service that leases the radio and exposes it over D-Bus | Linux |
| `crosswind-linux` | The `crosswind-linux` command: radio, doctor, test-card, capture, BLE over BlueZ | Linux |

`hardware/` holds the card reports, `tests/captures/` real iPhone captures for replay tests, and `packaging/` the systemd, D-Bus and NetworkManager files.

`cargo build` builds only the crates that run everywhere. Add `--workspace` on Linux for the rest.

## License

GNU General Public License v3.0 or later. See [LICENSE](LICENSE). Crates marked MIT in their `Cargo.toml` carry their own `LICENSE`.
