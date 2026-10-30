#!/usr/bin/env python3
"""Synthetic legacy schema for CI migration tests. Contains no user information.

Usage (in CI only): python3 scripts/create-legacy-fixture.py data/sappyl.db
Never run this command against the real production database.
"""
from __future__ import annotations
import os
import sqlite3
import sys
from pathlib import Path

if os.environ.get("CI") != "true":
    raise SystemExit("Refusing to create/replace a database outside CI")
if len(sys.argv)!=2 or Path(sys.argv[1]).as_posix()!="data/sappyl.db":
    raise SystemExit("Expected fixture path: data/sappyl.db")
path=Path(sys.argv[1]);path.parent.mkdir(parents=True,exist_ok=True)
if path.exists():path.unlink()
with sqlite3.connect(path) as db:
    # Original v0 schema including the inline UNIQUE(firstname) constraint.
    db.executescript("""
CREATE TABLE users (
  id uuid_text NOT NULL PRIMARY KEY,
  username varchar NOT NULL UNIQUE,
  password varchar NOT NULL
);
CREATE TABLE players (
  id uuid_text NOT NULL PRIMARY KEY,
  firstname varchar NOT NULL UNIQUE,
  lastname varchar NOT NULL,
  number integer NOT NULL
);
CREATE TABLE catalog_activities (
  id uuid_text NOT NULL PRIMARY KEY,
  user_id uuid_text NOT NULL,
  name varchar NOT NULL,
  description text NOT NULL,
  image_url text
);
CREATE TABLE trainings (
  id uuid_text NOT NULL PRIMARY KEY,
  user_id uuid_text NOT NULL,
  name varchar NOT NULL,
  date timestamp NOT NULL,
  notes text NOT NULL
);
CREATE TABLE training_activities (
  id uuid_text NOT NULL PRIMARY KEY,
  training_id uuid_text NOT NULL,
  catalog_id uuid_text,
  name varchar NOT NULL,
  description text NOT NULL,
  image_url text,
  sort_order integer NOT NULL
);
INSERT INTO users VALUES
 ('aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa','fixture-coach','not-a-real-password');
INSERT INTO players VALUES
 ('bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb','Lina','Fixture',9);
INSERT INTO catalog_activities VALUES
 ('cccccccc-cccc-4ccc-cccc-cccccccccccc',
  'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa',
  'Passes et déplacements','Exercice synthétique de CI',NULL);
INSERT INTO trainings VALUES
 ('dddddddd-dddd-4ddd-dddd-dddddddddddd',
  'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa',
  'Séance synthétique','2026-10-01 18:00:00','Notes de test');
INSERT INTO training_activities VALUES
 ('eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee',
  'dddddddd-dddd-4ddd-dddd-dddddddddddd',
  'cccccccc-cccc-4ccc-cccc-cccccccccccc',
  'Passes et déplacements','Historique synthétique',NULL,0);
""")
print("Synthetic legacy DB created (one player, one training, one activity)")
