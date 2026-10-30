#!/usr/bin/env bash
# Run BEFORE the production Web build, in the same environment as cargo/dx.
set -euo pipefail
variables=(
  SEPPYL_LEGAL_PUBLISHER
  SEPPYL_LEGAL_ADDRESS
  SEPPYL_LEGAL_PHONE
  SEPPYL_LEGAL_DIRECTOR
  SEPPYL_LEGAL_HOST
  SEPPYL_LEGAL_HOST_ADDRESS
  SEPPYL_LEGAL_HOST_PHONE
  SEPPYL_LEGAL_DATA_CONTROLLER
  SEPPYL_LEGAL_RETENTION
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
