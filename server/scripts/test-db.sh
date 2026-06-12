#!/usr/bin/env bash
# Runs dinnermate-db (and optionally other) tests against a disposable postgres.
# Usage:
#   ./scripts/test-db.sh                                   # cargo test -p dinnermate-db
#   ./scripts/test-db.sh "cargo test -p dinnermate-api"    # custom cargo test command
#
# The dev compose postgres on 55432 is untouched; this uses its own container on 55433.
set -euo pipefail

cd "$(dirname "$0")/.."

CONTAINER=dinnermate-test-db
PORT=55433

cleanup() { docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; }
trap cleanup EXIT
cleanup

docker run -d --name "$CONTAINER" \
    -e POSTGRES_USER=dinnermate_test \
    -e POSTGRES_PASSWORD=dinnermate_test \
    -e POSTGRES_DB=dinnermate_test \
    -p "$PORT:5432" \
    postgres:17-alpine >/dev/null

# -h 127.0.0.1 forces TCP: the image's init-phase temporary server only listens
# on the unix socket, so a socket check would report ready too early.
ready=0
for _ in $(seq 1 60); do
    if docker exec "$CONTAINER" pg_isready -h 127.0.0.1 -U dinnermate_test -d dinnermate_test >/dev/null 2>&1; then
        ready=1
        break
    fi
    sleep 1
done
if [ "$ready" -ne 1 ]; then
    echo "error: postgres did not become ready in time" >&2
    docker logs "$CONTAINER" >&2 || true
    exit 1
fi

export TEST_DATABASE_URL="postgres://dinnermate_test:dinnermate_test@localhost:$PORT/dinnermate_test"

if [ "$#" -gt 0 ]; then
    bash -c "$1"
else
    cargo test -p dinnermate-db -- --test-threads=1
fi
