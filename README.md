# Dinnermate

Dinnermate is Tinder for your next meal: friends create a room with search parameters (location, radius, cuisine, price, rating), share a 6-character code, and swipe on restaurant cards — liked restaurants land in the room's match list, sorted by like count. It's a Rust (axum + sqlx/Postgres) API in `server/` and a Flutter web client in `client/`, runnable locally with `docker compose up`. Design spec and implementation plan live in `docs/superpowers/specs/` and `docs/superpowers/plans/`.

## Running locally

```sh
docker compose up -d --build
```

- Web: <http://localhost:8888> (nginx proxies `/api/` to the api container)
- API: <http://localhost:18080> (`GET /healthz`, REST under `/api/v1`)
- Postgres: `localhost:55432` (`dinnermate`/`dinnermate`)

End-to-end smoke test (builds + starts the stack, exercises rooms, swipes,
matches, lists, and the web proxy; leaves everything running):

```sh
./scripts/smoke.sh
```

## Tests

```sh
# Core + API unit tests (no DB needed)
cd server && cargo test

# DB repo tests against a disposable postgres
server/scripts/test-db.sh

# Flutter analyze + test without a local Flutter install
docker run --rm -v "$PWD/client":/work -w /work ghcr.io/cirruslabs/flutter:stable \
  sh -c 'flutter create . --platforms=web && flutter pub get && flutter analyze && flutter test'
```

## Deploying

`compose.coolify.yaml` is the Coolify deployment compose (api + web, external
database via `DATABASE_URL` env). See the comments in that file for FQDN and
routing options.
