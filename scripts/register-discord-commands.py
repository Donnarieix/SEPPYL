#!/usr/bin/env python3
"""Register the SEPPYL slash command without replacing other guild commands.

Environment variables are local operator secrets; never commit them.
Requires DISCORD_BOT_TOKEN, DISCORD_APPLICATION_ID, SEPPYL_DISCORD_GUILD_ID.
"""
import json
import os
import sys
from urllib import request, error

required = ["DISCORD_BOT_TOKEN", "DISCORD_APPLICATION_ID", "SEPPYL_DISCORD_GUILD_ID"]
missing = [key for key in required if not os.environ.get(key)]
if missing:
    sys.exit("Variables manquantes : " + ", ".join(missing))
token = os.environ["DISCORD_BOT_TOKEN"]
app = os.environ["DISCORD_APPLICATION_ID"]
guild = os.environ["SEPPYL_DISCORD_GUILD_ID"]
if not app.isdecimal() or not guild.isdecimal():
    sys.exit("Identifiants Discord invalides")
payload = {
    "name": "seppyl",
    "description": "Assistance SEPPYL (réservée au personnel lié)",
    "type": 1,
    "options": [
        {
            "name": "lier",
            "description": "Lier votre compte Discord à SEPPYL",
            "type": 1,
            "options": [
                {"name": "code", "description": "Code temporaire de Mon compte > Discord",
                 "type": 3, "required": True, "min_length": 32, "max_length": 32}
            ]
        },
        {
            "name": "repondre",
            "description": "Répondre à un ticket SEPPYL",
            "type": 1,
            "options": [
                {"name": "ticket", "description": "Identifiant UUID du ticket",
                 "type": 3, "required": True, "min_length": 36, "max_length": 36},
                {"name": "message", "description": "Réponse au ticket (Markdown)",
                 "type": 3, "required": True, "max_length": 2000}
            ]
        }
    ]
}
url = f"https://discord.com/api/v10/applications/{app}/guilds/{guild}/commands"
req = request.Request(url, method="POST",
                      headers={"Authorization": f"Bot {token}", "Content-Type": "application/json"},
                      data=json.dumps(payload).encode())
try:
    with request.urlopen(req, timeout=15) as response:
        result = json.load(response)
        print("Commande enregistrée :", result["name"], result["id"])
except error.HTTPError as exc:
    # Discord errors can contain validation information, but never echo the token.
    message = exc.read(2048).decode("utf-8", "replace")
    sys.exit(f"Discord HTTP {exc.code} : {message}")
