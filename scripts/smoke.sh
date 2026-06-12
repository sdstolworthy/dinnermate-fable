#!/usr/bin/env bash
# End-to-end smoke test: brings the full compose stack up and exercises the
# room/swipe/match flow, the lists flow, and the web->api nginx proxy path.
# Leaves the stack RUNNING so you can click around afterwards.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API=http://localhost:18080
WEB=http://localhost:8888
HEADER_NAME=X-Dinnermate-User

STEP="(startup)"
fail() {
    echo "FAIL at step: $STEP" >&2
    echo "response status: ${STATUS:-<none>}" >&2
    echo "response body:   ${BODY:-<none>}" >&2
    exit 1
}
trap fail ERR

if command -v jq >/dev/null 2>&1; then
    json_get() { printf '%s' "$1" | jq -er "$2"; }
else
    json_get() {
        printf '%s' "$1" | python3 -c '
import json, sys, functools
data = json.load(sys.stdin)
# Supports the tiny subset of jq paths used below: .a.b[0].c
out = data
for part in sys.argv[1].strip(".").replace("]", "").split("."):
    name, _, idx = part.partition("[")
    if name:
        out = out[name]
    if idx:
        out = out[int(idx)]
print(out)
' "$2"
    }
fi

if command -v uuidgen >/dev/null 2>&1; then
    new_uuid() { uuidgen; }
else
    new_uuid() { python3 -c 'import uuid; print(uuid.uuid4())'; }
fi

# request METHOD URL USER_UUID [JSON_BODY] -> sets STATUS and BODY
request() {
    local method=$1 url=$2 user=$3 body=${4:-}
    local args=(-sS -o /tmp/smoke_body.$$ -w '%{http_code}' -X "$method" -H "$HEADER_NAME: $user")
    if [[ -n "$body" ]]; then
        args+=(-H 'Content-Type: application/json' -d "$body")
    fi
    STATUS=$(curl "${args[@]}" "$url")
    BODY=$(cat /tmp/smoke_body.$$)
    rm -f /tmp/smoke_body.$$
}

assert_status() {
    [[ "$STATUS" == "$1" ]] || fail
}

echo "==> docker compose up -d --build"
STEP="docker compose up"
docker compose -f "$REPO_ROOT/docker-compose.yml" up -d --build

echo "==> waiting for api to be healthy (timeout 120s)"
STEP="wait for api healthy"
deadline=$((SECONDS + 120))
until curl -fsS "$API/healthz" >/dev/null 2>&1; do
    if (( SECONDS >= deadline )); then
        BODY="api did not become healthy within 120s"
        fail
    fi
    sleep 2
done
echo "    api healthy"

USER_A=$(new_uuid)
USER_B=$(new_uuid)

STEP="create room"
request POST "$API/api/v1/rooms" "$USER_A" '{
    "location_label": "Salt Lake City",
    "lat": 40.7600, "lng": -111.8900, "radius_m": 40000,
    "cuisines": [], "price_min": 1, "price_max": 4, "min_rating": 0
}'
assert_status 201
CODE=$(json_get "$BODY" '.room.code')
DECK0=$(json_get "$BODY" '.deck[0].id')
DECK1=$(json_get "$BODY" '.deck[1].id')
echo "==> room created: code=$CODE deck0=$DECK0 deck1=$DECK1"

STEP="user A joins room"
request POST "$API/api/v1/rooms/$CODE/join" "$USER_A" '{"display_name": "Alice"}'
assert_status 200

STEP="user B joins room"
request POST "$API/api/v1/rooms/$CODE/join" "$USER_B" '{"display_name": "Bob"}'
assert_status 200
echo "==> both users joined"

STEP="user A likes deck[0]"
request POST "$API/api/v1/rooms/$CODE/swipes" "$USER_A" \
    "{\"restaurant_id\": \"$DECK0\", \"liked\": true}"
assert_status 201

STEP="user A likes deck[1]"
request POST "$API/api/v1/rooms/$CODE/swipes" "$USER_A" \
    "{\"restaurant_id\": \"$DECK1\", \"liked\": true}"
assert_status 201

STEP="user B likes deck[0]"
request POST "$API/api/v1/rooms/$CODE/swipes" "$USER_B" \
    "{\"restaurant_id\": \"$DECK0\", \"liked\": true}"
assert_status 201
echo "==> swipes recorded"

STEP="matches: first entry like_count==2, participant_count==2"
request GET "$API/api/v1/rooms/$CODE/matches" "$USER_A"
assert_status 200
LIKE_COUNT=$(json_get "$BODY" '.matches[0].like_count')
PARTICIPANT_COUNT=$(json_get "$BODY" '.participant_count')
TOP_ID=$(json_get "$BODY" '.matches[0].restaurant.id')
[[ "$LIKE_COUNT" == "2" && "$PARTICIPANT_COUNT" == "2" && "$TOP_ID" == "$DECK0" ]] || fail
echo "==> matches OK (top match $TOP_ID, like_count=$LIKE_COUNT, participants=$PARTICIPANT_COUNT)"

STEP="web proxy: GET /api/v1/rooms/CODE through nginx"
request GET "$WEB/api/v1/rooms/$CODE" "$USER_A"
assert_status 200
echo "==> web /api/ proxy -> api OK"

STEP="web /health.txt"
request GET "$WEB/health.txt" "$USER_A"
[[ "$STATUS" == "200" && "$BODY" == "ok" ]] || fail
echo "==> web /health.txt OK"

STEP="web / serves the SPA shell"
request GET "$WEB/" "$USER_A"
assert_status 200
grep -qi '<!DOCTYPE' <<<"$BODY" || fail
echo "==> web index OK"

STEP="create list"
request POST "$API/api/v1/lists" "$USER_A" '{"name": "Date night"}'
assert_status 201
LIST_CODE=$(json_get "$BODY" '.list.code')

STEP="add list item"
request POST "$API/api/v1/lists/$LIST_CODE/items" "$USER_A" '{"name": "Pago"}'
assert_status 201

STEP="fetch list by code: item present"
request GET "$API/api/v1/lists/$LIST_CODE" "$USER_B"
assert_status 200
ITEM_NAME=$(json_get "$BODY" '.items[0].name')
[[ "$ITEM_NAME" == "Pago" ]] || fail
echo "==> lists OK (code=$LIST_CODE, item=$ITEM_NAME)"

echo
echo "SMOKE OK"
echo "Stack left running:"
echo "  web: $WEB"
echo "  api: $API"
