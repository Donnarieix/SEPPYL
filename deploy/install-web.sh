#!/usr/bin/env bash
set -Eeuo pipefail
umask 022
base=/var/www/seppyl
mkdir -p "$base/releases"
release="$base/releases/$(date -u +%Y%m%dT%H%M%SZ)-$$"
mkdir "$release"
trap 'rm -rf "$release"' ERR
tar -xz -C "$release" --no-same-owner --no-same-permissions
test -f "$release/index.html"
ln -sfn "$release" "$base/current.next"
mv -Tf "$base/current.next" "$base/current"
