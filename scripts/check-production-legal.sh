#!/usr/bin/env bash
# Run BEFORE building the Web client; the application is self-hosted, not hosted by o2switch.
# Keep the publisher's actual identity in the private build environment.
set -euo pipefail
variables=(
  SEPPYL_LEGAL_PUBLISHER
  SEPPYL_LEGAL_ADDRESS
  SEPPYL_LEGAL_PHONE
  SEPPYL_LEGAL_DIRECTOR
)
missing=0
for var in "${variables[@]}"; do
  value="${!var:-}"
  if [[ -z "${value//[[:space:]]/}" || "$value" == *"À renseigner"* || "$value" == *"À définir"* ]]; then
    printf 'Manquant : %s\n' "$var" >&2
    missing=1
  fi
done
if [[ "$missing" -ne 0 ]]; then
  printf 'Configuration légale incomplète : NE PAS publier ce build.\n' >&2
  exit 1
fi
printf 'Champs de publication présents. Vérifier manuellement leur exactitude avant déploiement.\n'
