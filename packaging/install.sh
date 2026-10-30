#!/bin/sh
set -eu
umask 022

ROOT=$(cd "$(dirname "$0")/.." && pwd)
LIST=
if [ "${1:-}" = --list ]; then
  LIST=1
  shift
fi
PKG=${1:-}
case "$PKG" in
  crosswind) ;;
  *)
    echo "usage: packaging/install.sh [--list] crosswind" >&2
    exit 2
    ;;
esac

DESTDIR=${DESTDIR:-}
PREFIX=${PREFIX:-/usr}
BINDIR=${BINDIR:-$PREFIX/bin}
LIBDIR=${LIBDIR:-$PREFIX/lib}
DATADIR=${DATADIR:-$PREFIX/share}
LICENSEDIR=${LICENSEDIR:-$DATADIR/licenses/$PKG}
DOCDIR=${DOCDIR:-$DATADIR/doc/$PKG}
TARGET=${TARGET:-${CARGO_TARGET_DIR:-$ROOT/target}}
BUILT=$TARGET/release

put() {
  if [ -n "$LIST" ]; then
    echo "$3"
    return
  fi
  [ -f "$2" ] || { echo "packaging/install.sh: $2 is missing; build it first with: cargo build --release --locked -p crosswindd -p crosswind-linux" >&2; exit 1; }
  install -D -m "$1" "$2" "$DESTDIR$3"
}

daemon_unit() {
  if [ -n "$LIST" ]; then
    echo "$1"
    return
  fi
  install -d "$DESTDIR$(dirname "$1")"
  sed "s|^ExecStart=/usr/lib/crosswind/crosswindd$|ExecStart=$LIBDIR/crosswind/crosswindd|" \
    "$ROOT/packaging/crosswindd.service" > "$DESTDIR$1"
  chmod 644 "$DESTDIR$1"
  grep -q "^ExecStart=$LIBDIR/crosswind/crosswindd$" "$DESTDIR$1" \
    || { echo "packaging/install.sh: crosswindd.service has no ExecStart=/usr/lib/crosswind/crosswindd to rewrite" >&2; exit 1; }
}

put 755 "$BUILT/crosswindd" "$LIBDIR/crosswind/crosswindd"
put 755 "$BUILT/crosswind-linux" "$BINDIR/crosswind-linux"
daemon_unit "$LIBDIR/systemd/system/crosswindd.service"
put 644 "$ROOT/packaging/io.github.mrhafid.Crosswind.conf" "$DATADIR/dbus-1/system.d/io.github.mrhafid.Crosswind.conf"
put 644 "$ROOT/packaging/io.github.mrhafid.Crosswind.service" "$DATADIR/dbus-1/system-services/io.github.mrhafid.Crosswind.service"
put 644 "$ROOT/packaging/io.github.mrhafid.Crosswind.policy" "$DATADIR/polkit-1/actions/io.github.mrhafid.Crosswind.policy"
put 644 "$ROOT/packaging/90-crosswind-unmanaged.conf" "$LIBDIR/NetworkManager/conf.d/90-crosswind-unmanaged.conf"
put 644 "$ROOT/packaging/10-crosswind-awdl0.link" "$LIBDIR/systemd/network/10-crosswind-awdl0.link"
put 644 "$ROOT/README.md" "$DOCDIR/README.md"
put 644 "$ROOT/LICENSE" "$LICENSEDIR/LICENSE"
