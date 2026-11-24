#!/usr/bin/env python3
"""Register SEPPYL's one guild-scoped Discord command (idempotent).
Uses a bot token only from local environment. Does NOT change other commands.
"""
import json
import os
import sys
from urllib import error, request

keys = ("DISCORD_APPLICATION_ID", "SEPPYL_DISCORD_GUILD_ID", "DISCORD_BOT_TOKEN")
missing = [key for key in keys if not os.environ.get(key)]
if missing:
    sys.exit("Variables manquantes : " + ", ".join(missing))
application_id = os.environ["DISCORD_APPLICATION_ID"]
guild_id = os.environ["SEPPYL_DISCORD_GUILD_ID"]
token = os.environ["DISCORD_BOT_TOKEN"]
if not (application_id.isdecimal() and guild_id.isdecimal()):
    sys.exit("IDs Discord invalides")

def string_option(name, description, required=True, maximum=2000):
    return {"name":name, "description":description, "type":3,
            "required":required, "max_length":maximum}

ticket = lambda: string_option("ticket", "UUID du ticket SEPPYL", maximum=36)
statuses = [
    ("Ouvert", "open"), ("En cours", "in_progress"),
    ("Résolu", "resolved"), ("Fermé", "closed"),
]
command = {
    "name":"seppyl",
    "description":"Gérer les tickets de support SEPPYL",
    "type":1,
    "options":[
        {"name":"lier","type":1,"description":"Associer votre compte SEPPYL",
         "options":[string_option("code","Code temporaire depuis Mon compte",maximum=32)]},
        {"name":"delier","type":1,"description":"Supprimer la liaison de votre compte"},
        {"name":"liste","type":1,"description":"Afficher les tickets à traiter"},
        {"name":"voir","type":1,"description":"Voir un ticket et ses dernières réponses",
         "options":[ticket()]},
        {"name":"repondre","type":1,"description":"Envoyer une réponse dans un ticket",
         "options":[ticket(),string_option("message","Réponse au ticket",maximum=2000)]},
        {"name":"statut","type":1,"description":"Changer le statut d'un ticket",
         "options":[ticket(),{"name":"etat","description":"Nouveau statut","type":3,
                              "required":True,
                              "choices":[{"name":label,"value":value} for label,value in statuses]}]},
        {"name":"maintainer","type":1,"description":"Activer ou retirer une escalade Maintainer",
         "options":[ticket(),{"name":"activer","type":5,"required":True,
                              "description":"Activer (oui) ou retirer (non)"}]},
    ]
}
url = f"https://discord.com/api/v10/applications/{application_id}/guilds/{guild_id}/commands"
req = request.Request(url, data=json.dumps(command).encode("utf-8"), method="POST",
    headers={"Authorization":f"Bot {token}","Content-Type":"application/json"})
try:
    with request.urlopen(req, timeout=15) as result:
        data=json.load(result)
    print(f"Commande /{data['name']} enregistrée sur le serveur {guild_id}.")
except error.HTTPError as exc:
    # Only show the API validation response, never print the token or headers.
    detail=exc.read(2048).decode("utf-8","replace")
    sys.exit(f"Discord HTTP {exc.code} : {detail}")
