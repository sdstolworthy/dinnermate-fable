# Deploying Dinnermate on Coolify

Deployed 2026-06-12 as a Docker Compose application on Coolify v4 (`https://coolify.stolworthy.co`). Live at **<https://dinnermate.coolify.stolworthy.co>**.

## Shape

| | |
|---|---|
| Project | `dinnermate` (`n11qh7w4h0h2tcyl5t910t2c`) |
| Application | `dinnermate` (`rpvmi67177g457wq6b1e89oj`), environment `production` |
| Source | public GitHub repo `sdstolworthy/dinnermate-fable`, branch `main` |
| Compose file | `/compose.coolify.yaml` (services `api` + `web`) |
| FQDN | single: `https://dinnermate.coolify.stolworthy.co` → `web`; nginx inside `web` proxies `/api/` → `api:8080`, so no separate API FQDN and no CORS |

## Environment variables (Coolify → app → Environment Variables)

| Variable | Value | Notes |
|---|---|---|
| `DATABASE_URL` | `postgres://dinnermate:…@10.0.0.86:5555/dinnermate` | **Secret.** See Database below. |
| `RESTAURANT_PROVIDER` | `seed` | Flip to `google` once a Places key exists. |
| `GOOGLE_PLACES_API_KEY` | _(empty)_ | Required iff provider=google. **Secret.** |
| `RUST_LOG` | `info,tower_http=info,sqlx=warn` | |

## Database

Same pattern as fulfilled: the shared Coolify Postgres resource (`postgresql-database-e7t5cd5eqc7m6p0iuiwz4375`, public on `10.0.0.86:5555`) with a per-app database. Role + database `dinnermate` (role owns the db, so `sqlx::migrate!` on boot can create tables). Credentials live only in the Coolify env var.

Coolify does **not** back up this volume — room/swipe data is treated as disposable for v1. If lists become precious, add a pg_dump cron per the homelab backup pattern.

## Deploys

- Push to `main` → GitHub webhook (`…/webhooks/source/github/events/manual`, hook id `640257567`) → Coolify auto-deploys. Per house rules: **verify the deploy actually fired** in Coolify's Deployments tab; the webhook has gone silent on other projects before.
- Manual: deploy button in UI, or `GET /api/v1/deploy?uuid=rpvmi67177g457wq6b1e89oj` with a bearer token.
- Container names on the VM: `api-rpvmi67177g457wq6b1e89oj-<deploy-id>` / `web-…`; find current via `sudo docker ps | grep rpvmi`.

## Health

- `https://dinnermate.coolify.stolworthy.co/health.txt` → `ok` (web)
- api container has an internal `/healthz` HEALTHCHECK that gates rolling deploys; it is not exposed through the web FQDN (nginx only proxies `/api/`).

## v3: restaurant provider in production

`RESTAURANT_PROVIDER=osm` (OpenStreetMap via Overpass) is the production provider as of v3 — real restaurants, no API key. `OVERPASS_URL` overrides the default `https://overpass-api.de` if the public instance gets slow (alternatives: `https://overpass.kumi.systems`). Fallbacks: `seed` (demo data, always works), `google` (best data; needs `GOOGLE_PLACES_API_KEY`). OSM caveat: no ratings/price/photos, hours coverage partial — the UI renders unknowns gracefully.
