# crosswind-radio

AWDL, the Wi-Fi link AirDrop runs over, on ordinary Linux Wi-Fi cards, without Apple's Broadcom chip. It's the radio under [Crosswind](../../README.md), and runs [crosswind-awdl](../crosswind-awdl) over nl80211. Linux only.

- `radio::Backend` is the trait a radio service runs: `up` brings the link up (an `awdl0` interface with an IPv6 link-local address, and the AWDL peers heard on it), `down` gives Wi-Fi back, and `recover` undoes what a session left behind when its process died, from a journal.
- `injection::Injection` implements it with crosswind-awdl, on a monitor interface and a TAP `awdl0`, in the mode the card needs: shared, which keeps Wi-Fi connected; exclusive, with an ad-hoc cell or a hidden AP in the station's place; or active monitor.
- `probe` finds what the card can do, without root. `quirks` holds what the probe can't detect, per driver: the `drivers.toml` packaged with the crate, with `/etc/crosswind/drivers.toml` read over it. `config` reads `/etc/crosswind/radio.toml`.
- `fake` is a backend for testing what runs on top.

Bringing the radio up needs `CAP_NET_ADMIN` and `CAP_NET_RAW`, and runs NetworkManager's `nmcli`, `ip`, `ping` and, on cards without ad-hoc mode, `hostapd`. crosswindd, Crosswind's system service, runs it and lends the link out over D-Bus. [Crosswind's README](https://github.com/Mr-Hafid/crosswind#supported-hardware) says which cards work in which mode.

## Status

On 0.x until Crosswind 1.0: the API can change between minor versions.

## License

MIT
