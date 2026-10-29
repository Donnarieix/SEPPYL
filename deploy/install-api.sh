#!/usr/bin/env bash
set -Eeuo pipefail
umask 022
base=/opt/seppyl/api
mkdir -p "$base/releases"
release="$base/releases/$(date -u +%Y%m%dT%H%M%SZ)-$$"
mkdir "$release"
trap 'rm -rf "$release"' ERR
tar -xz -C "$release" --no-same-owner --no-same-permissions
test -x "$release/api"
old="$(readlink -f "$base/current" || true)"
ln -sfn "$release" "$base/current.next"
mv -Tf "$base/current.next" "$base/current"
if ! sudo -n systemctl restart seppyl-api.service || ! curl --fail --silent --show-error --retry 8 --retry-delay 2 http://127.0.0.1:3101/health >/dev/null; then
  if [ -n "$old" ]; then ln -sfn "$old" "$base/current.next"; mv -Tf "$base/current.next" "$base/current"; sudo -n systemctl restart seppyl-api.service; fi
  exit 1
fi
