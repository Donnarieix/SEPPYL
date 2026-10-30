#!/usr/bin/env python3
"""Seed a synthetic CI-only user into the legacy migration fixture.
Never use against real data: it refuses to run outside CI and requires
an exact disposable fixture filename.
"""
import os
import sqlite3
import sys
import uuid
from pathlib import Path

if os.environ.get("CI") != "true" or len(sys.argv) != 2:
    raise SystemExit("CI-only fixture script")
db_path = Path(sys.argv[1])
if db_path.as_posix() != "data/sappyl.db" or not db_path.is_file():
    raise SystemExit("Expected existing disposable data/sappyl.db")
# Generated specifically for the synthetic test password:
# e2e-ci-only-secret! (no credentials or production data are involved).
PHC = "$argon2id$v=19$m=65536,t=3,p=4$kNmPrhVzlz9myIkBW5+vdg$1crwdUfs8rfq3YPSLjSJUeA3JGDWDv58MZvGO+VSH6Y"
with sqlite3.connect(db_path) as db:
    # The very first SQLite snapshot stored UUID-shaped text in its legacy
    # fixture. Current SeaORM + SQLx decode `Uuid` from 16-byte BLOB values.
    # Normalize only this disposable test fixture, preserving all references.
    for table, column in [
        ("users", "id"),
        ("players", "id"),
        ("catalog_activities", "id"),
        ("catalog_activities", "user_id"),
        ("trainings", "id"),
        ("trainings", "user_id"),
        ("training_activities", "id"),
        ("training_activities", "training_id"),
        ("training_activities", "catalog_id"),
    ]:
        rows = db.execute(f"SELECT rowid, {column} FROM {table}").fetchall()
        for rowid, value in rows:
            if value is not None and isinstance(value, str):
                db.execute(f"UPDATE {table} SET {column}=? WHERE rowid=?",
                           (uuid.UUID(value).bytes, rowid))
    db.execute("ALTER TABLE users ADD COLUMN role TEXT")
    db.execute("UPDATE users SET password=?, role='maintainer' WHERE username='fixture-coach'", (PHC,))
    assert db.execute("SELECT COUNT(*) FROM users WHERE role='maintainer'").fetchone()[0] == 1
print("Synthetic E2E account prepared")
