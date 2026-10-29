#!/usr/bin/env python3
"""One-time secure SEPPYL user enrollment. Run as root on infra-static.

Requires the Nginx configuration to return HTTP 403 for public POST /api/register.
This script never prints or persists the account password.
"""
from __future__ import annotations

import getpass
import json
import os
import sqlite3
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

ENV = Path("/etc/seppyl/api.env")
DB = Path("/var/lib/seppyl/data/sappyl.db")
PUBLIC_REGISTER = "https://seppyl.donnarieix.fr/api/register"
LOCAL_API = "http://127.0.0.1:3101"


def post(url: str, payload: dict) -> int:
    request = Request(
        url,
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urlopen(request, timeout=8) as response:
        return response.status


def main() -> None:
    if os.geteuid() != 0:
        sys.exit("Execute as root: sudo python3 deploy/bootstrap-user.py")

    # Block any accidental public signup BEFORE momentarily enabling local signup.
    try:
        post(PUBLIC_REGISTER, {})
    except HTTPError as exc:
        if exc.code != 403:
            sys.exit(f"Aborted: public registration returned HTTP {exc.code}, expected 403")
    except (URLError, TimeoutError) as exc:
        sys.exit(f"Aborted: cannot verify Nginx public registration block: {exc}")
    else:
        sys.exit("Aborted: public registration is not blocked!")

    original_env = ENV.read_bytes()
    needle = b"SEPPYL_ALLOW_REGISTRATION=false"
    if original_env.count(needle) != 1:
        sys.exit("Aborted: expected one SEPPYL_ALLOW_REGISTRATION=false setting")
    if not DB.is_file():
        sys.exit("Aborted: persistent SQLite database not found")

    username = input("Nouvel identifiant SEPPYL (3-80 caractères) : ").strip()
    password = getpass.getpass("Mot de passe (12 caractères minimum) : ")
    confirm = getpass.getpass("Confirmer le mot de passe : ")
    if not (3 <= len(username) <= 80 and len(password) >= 12 and password == confirm):
        sys.exit("Aborted: invalid username, short password or mismatch")

    # SQLite's backup API yields a transaction-consistent backup even while the API runs.
    backup_dir = Path("/var/backups/seppyl")
    backup_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    destination = backup_dir / (
        "sappyl-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + ".db"
    )
    src = sqlite3.connect(f"file:{DB}?mode=ro", uri=True)
    dst = sqlite3.connect(str(destination))
    try:
        src.backup(dst)
    finally:
        src.close()
        dst.close()
    destination.chmod(0o600)
    print(f"SQLite sauvegardé dans {destination}")

    try:
        ENV.write_bytes(original_env.replace(needle, b"SEPPYL_ALLOW_REGISTRATION=true", 1))
        subprocess.run(["systemctl", "restart", "seppyl-api.service"], check=True)
        for attempt in range(10):
            try:
                status = post(f"{LOCAL_API}/register", {"username": username, "password": password})
                if status != 200:
                    raise RuntimeError(f"Unexpected register status {status}")
                print("Compte créé.")
                break
            except HTTPError as exc:
                raise RuntimeError(f"Registration returned HTTP {exc.code}") from exc
            except (URLError, TimeoutError):
                if attempt == 9:
                    raise
                time.sleep(0.5)
        # Ensure credentials actually work without printing the returned JWT.
        status = post(f"{LOCAL_API}/login", {"username": username, "password": password})
        if status != 200:
            raise RuntimeError(f"Login verification returned HTTP {status}")
        print("Connexion au compte vérifiée.")
    finally:
        ENV.write_bytes(original_env)
        subprocess.run(["systemctl", "restart", "seppyl-api.service"], check=True)
        print("Inscription désactivée à nouveau.")


if __name__ == "__main__":
    main()
