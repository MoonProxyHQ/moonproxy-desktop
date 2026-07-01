#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <json1> [json2] [json3]" >&2
  exit 1
fi

CLOUDFRONT_DOMAIN="${CLOUDFRONT_DOMAIN:?CLOUDFRONT_DOMAIN env var not set}"
TAG="${TAG:?TAG env var not set}"

BASE_JSON="$1"

MERGED_PLATFORMS=$(
  jq -s '
    [ .[].platforms // {} | to_entries[] ]
    | group_by(.key)
    | map({ key: .[0].key, value: .[0].value })
    | from_entries
  ' "$@"
)

REWRITTEN_PLATFORMS=$(
  echo "$MERGED_PLATFORMS" | jq --arg domain "$CLOUDFRONT_DOMAIN" --arg tag "$TAG" '
    . | to_entries | map(
      .value.url = (
        .value.url
        | gsub(
            "https://github\\.com/[^/]+/[^/]+/releases/download/[^/]+/(?<file>[^?#]+)";
            "https://\($domain)/releases/\($tag)/\(.file)"
          )
      )
    ) | from_entries
  '
)

jq -n \
  --argjson base "$(jq '{version, notes, pub_date}' "$BASE_JSON")" \
  --argjson platforms "$REWRITTEN_PLATFORMS" \
  '$base + { platforms: $platforms }'
