#!/usr/bin/env python3
"""SEPPYL Discord Gateway development bot.

Receives real Discord slash commands/buttons over WebSocket and relays them to
the SAME Rust interaction dispatcher as production. No public URL or port
forwarding. Do not run this bot concurrently with the HTTP interactions
endpoint of the same Discord application.
"""
from __future__ import annotations

import asyncio
import json
import os
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

import aiohttp
import discord
from discord import app_commands
from dotenv import load_dotenv

load_dotenv(Path(__file__).resolve().parents[1] / ".env", override=False)

def needed(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise SystemExit(f"Variable manquante : {name} (fichier .env du projet)")
    return value

BOT_TOKEN = needed("SEPPYL_DISCORD_BOT_TOKEN")
GUILD_ID = int(needed("SEPPYL_DISCORD_GUILD_ID"))
APP_ID = int(needed("SEPPYL_DISCORD_APPLICATION_ID"))
BRIDGE_SECRET = needed("SEPPYL_DISCORD_LOCAL_BRIDGE_TOKEN")
if len(BRIDGE_SECRET) < 32:
    raise SystemExit("SEPPYL_DISCORD_LOCAL_BRIDGE_TOKEN doit contenir au moins 32 caractères.")
API_BASE = os.environ.get("SEPPYL_DISCORD_LOCAL_API", "http://127.0.0.1:3000").rstrip("/")
parsed = urlparse(API_BASE)
if parsed.scheme != "http" or parsed.hostname not in {"localhost", "127.0.0.1", "::1"} or parsed.path:
    raise SystemExit("SEPPYL_DISCORD_LOCAL_API doit être une URL HTTP locale, sans chemin.")
API_ENDPOINT = API_BASE + "/discord/local-gateway"
GUILD = discord.Object(id=GUILD_ID)
MENTIONS = discord.AllowedMentions.none()


def envelope(interaction: discord.Interaction, kind: int, data: dict[str, Any]) -> dict[str, Any]:
    """Only the Gateway-provided user, guild and application identities are trusted."""
    return {
        "type": kind,
        "id": str(interaction.id),
        "application_id": str(interaction.application_id),
        "guild_id": str(interaction.guild_id) if interaction.guild_id else None,
        "member": {"user": {"id": str(interaction.user.id)}},
        "data": data,
    }


def slash_payload(interaction: discord.Interaction, subcommand: str,
                  **options: str | bool) -> dict[str, Any]:
    return envelope(interaction, 2, {
        "name": "seppyl",
        "options": [{
            "type": 1, "name": subcommand,
            "options": [
                {"name": name, "type": 5 if isinstance(value, bool) else 3, "value": value}
                for name, value in options.items()
            ],
        }],
    })


def as_view(data: dict[str, Any]) -> discord.ui.View | None:
    rows = data.get("components", [])
    if not rows:
        return None
    view = discord.ui.View(timeout=900)
    for row in rows:
        for element in row.get("components", []):
            if element.get("type") != 2:
                continue
            custom_id = str(element.get("custom_id", ""))
            if not custom_id.startswith("seppyl|") or len(custom_id) > 100:
                continue
            style = discord.ButtonStyle(element.get("style", 2))
            button = discord.ui.Button(
                custom_id=custom_id,
                label=str(element.get("label", "Action"))[:80],
                style=style,
            )
            # on_interaction handles buttons, including pre-existing bot messages
            # from before a local bot restart.
            view.add_item(button)
    return view if view.children else None


class ReplyModal(discord.ui.Modal):
    def __init__(self, bot: "SeppylBot", custom_id: str) -> None:
        super().__init__(title="Répondre au ticket", custom_id=custom_id, timeout=300)
        self.bot = bot
        self.reply = discord.ui.TextInput(
            label="Réponse SEPPYL", custom_id="message",
            style=discord.TextStyle.paragraph,
            required=True, min_length=1, max_length=2000,
        )
        self.add_item(self.reply)

    async def on_submit(self, interaction: discord.Interaction) -> None:
        payload = envelope(interaction, 5, {
            "custom_id": self.custom_id,
            "components": [{"type": 1, "components": [
                {"type": 4, "custom_id": "message", "value": str(self.reply.value)}
            ]}],
        })
        await self.bot.forward(interaction, payload)

    async def on_error(self, interaction: discord.Interaction, error: Exception) -> None:
        print(f"[Discord] Erreur du formulaire : {type(error).__name__}")
        if not interaction.response.is_done():
            await interaction.response.send_message(
                "Impossible d'envoyer cette réponse. Vérifie le bot local.",
                ephemeral=True,
            )


class SeppylBot(discord.Client):
    def __init__(self) -> None:
        super().__init__(intents=discord.Intents.default())
        self.tree = app_commands.CommandTree(self)
        self.session: aiohttp.ClientSession | None = None

    async def bridge(self, payload: dict[str, Any]) -> dict[str, Any]:
        if self.session is None:
            raise RuntimeError("La session HTTP locale n'est pas initialisée.")
        async with self.session.post(
            API_ENDPOINT,
            json=payload,
            headers={"X-Seppyl-Bridge-Token": BRIDGE_SECRET},
        ) as result:
            if result.status != 200:
                raise RuntimeError(f"API locale HTTP {result.status} ({API_ENDPOINT})")
            return await result.json()

    async def setup_hook(self) -> None:
        self.session = aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=2.4))
        # Fail fast with a useful error instead of registering commands which
        # will systematically time out when the Rust API is not running.
        try:
            pong = await self.bridge({"type": 1, "application_id": str(APP_ID)})
            if pong.get("type") != 1:
                raise RuntimeError("La réponse PING de l'API est invalide.")
        except (aiohttp.ClientError, asyncio.TimeoutError, RuntimeError) as exc:
            raise RuntimeError(
                "La passerelle SEPPYL locale est inaccessible. Lance l'API Rust "
                "avec SEPPYL_BIND=127.0.0.1:3000 et la même clé "
                "SEPPYL_DISCORD_LOCAL_BRIDGE_TOKEN avant le bot."
            ) from exc
        self.tree.add_command(seppyl, guild=GUILD)
        synced = await self.tree.sync(guild=GUILD)
        print(f"[SEPPYL] {len(synced)} commande(s) Discord synchronisée(s).")

    async def close(self) -> None:
        if self.session is not None:
            await self.session.close()
        await super().close()

    async def on_ready(self) -> None:
        print(f"[SEPPYL] Bot connecté : {self.user}. Discord Gateway → {API_ENDPOINT}")
        print("[SEPPYL] Essaie /seppyl liste dans ton serveur privé.")

    async def forward(self, interaction: discord.Interaction,
                      payload: dict[str, Any]) -> None:
        try:
            result = await self.bridge(payload)
            if result.get("type") == 9:
                custom_id = result.get("data", {}).get("custom_id", "")
                if not custom_id.startswith("seppyl|modal|"):
                    raise RuntimeError("Formulaire invalide renvoyé par l'API.")
                await interaction.response.send_modal(ReplyModal(self, custom_id))
                return
            if result.get("type") != 4:
                raise RuntimeError("Réponse d'interaction inconnue.")
            data = result.get("data", {})
            content = str(data.get("content", "") or "Opération terminée.")
            view = as_view(data)
            await interaction.response.send_message(
                content[:2000], ephemeral=True, view=view,
                allowed_mentions=MENTIONS,
            )
        except (aiohttp.ClientError, asyncio.TimeoutError, RuntimeError, ValueError) as exc:
            print(f"[SEPPYL] Interaction non traitée : {type(exc).__name__}: {exc}")
            if not interaction.response.is_done():
                await interaction.response.send_message(
                    "SEPPYL local est indisponible. Vérifie que l'API Rust "
                    "tourne et que la clé de liaison est identique.",
                    ephemeral=True,
                )

    async def on_interaction(self, interaction: discord.Interaction) -> None:
        # Application commands go through CommandTree. Modals use their own
        # on_submit method; this handler covers REST-created buttons too.
        if interaction.type is not discord.InteractionType.component:
            return
        custom_id = (interaction.data or {}).get("custom_id", "")
        if not isinstance(custom_id, str) or not custom_id.startswith("seppyl|"):
            return
        await self.forward(interaction, envelope(interaction, 3, dict(interaction.data)))


seppyl = app_commands.Group(
    name="seppyl", description="Gérer les tickets SEPPYL depuis Discord",
)


@seppyl.command(name="lier", description="Lier ton compte SEPPYL à Discord")
async def lier(interaction: discord.Interaction, code: str) -> None:
    await client.forward(interaction, slash_payload(interaction, "lier", code=code))


@seppyl.command(name="delier", description="Retirer la liaison entre Discord et SEPPYL")
async def delier(interaction: discord.Interaction) -> None:
    await client.forward(interaction, slash_payload(interaction, "delier"))


@seppyl.command(name="liste", description="Afficher les tickets à traiter")
async def liste(interaction: discord.Interaction) -> None:
    await client.forward(interaction, slash_payload(interaction, "liste"))


@seppyl.command(name="voir", description="Voir un ticket et ses dernières réponses")
async def voir(interaction: discord.Interaction, ticket: str) -> None:
    await client.forward(interaction, slash_payload(interaction, "voir", ticket=ticket))


@seppyl.command(name="repondre", description="Ajouter une réponse à un ticket")
async def repondre(interaction: discord.Interaction, ticket: str, message: str) -> None:
    await client.forward(interaction, slash_payload(
        interaction, "repondre", ticket=ticket, message=message,
    ))


@seppyl.command(name="statut", description="Changer le statut d'un ticket")
@app_commands.choices(etat=[
    app_commands.Choice(name="Ouvert", value="open"),
    app_commands.Choice(name="En cours", value="in_progress"),
    app_commands.Choice(name="Résolu", value="resolved"),
    app_commands.Choice(name="Fermé", value="closed"),
])
async def statut(interaction: discord.Interaction, ticket: str,
                 etat: app_commands.Choice[str]) -> None:
    await client.forward(interaction, slash_payload(
        interaction, "statut", ticket=ticket, etat=etat.value,
    ))


@seppyl.command(name="maintainer", description="Activer ou retirer l'escalade Maintainer")
async def maintainer(interaction: discord.Interaction, ticket: str,
                     activer: bool) -> None:
    await client.forward(interaction, slash_payload(
        interaction, "maintainer", ticket=ticket, activer=activer,
    ))


if __name__ == "__main__":
    client = SeppylBot()
    print("[SEPPYL] Mode développement Discord Gateway (aucune URL publique).")
    client.run(BOT_TOKEN, log_handler=None)
