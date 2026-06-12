# Dinnermate — v1 Design

**Date:** 2026-06-11
**Source spec:** `/workplace/spencerspecs/src/dinnermate.md`
**Mode:** Autonomous overnight build. Spencer approved proceeding without interactive review ("use good judgment to unblock yourself during the night"). Every decision below that would normally be a clarifying question is self-answered with rationale, flagged **[DECISION]** for morning review.

## What we're building

Dinnermate is "Tinder for your next meal." Friends create a room with search parameters, share a room code, and swipe on restaurant cards. Liked restaurants appear in the room's match list, sorted by like count. Separately, users can keep shareable favorites lists.

v1 targets the browser-first flow from the spec: open a link, join a room, swipe — no app download, no account. The UX north star is **time to first swipe**.

## Scope

### In v1

- Create a room: location (lat/lng + free-text label), radius, cuisine filter, price range, minimum rating. Returns a 6-character room code.
- Join a room by code with just a display name (anonymous identity).
- Swipe deck: every participant swipes the same snapshot of restaurants.
- Matches list per room, sorted by like count descending, updating while you swipe.
- Lists: create a list, add restaurants, share via list code, anyone with the code can view and add.
- Flutter web client with the spec's breezy/soft visual direction.
- Deployable to Coolify (Docker compose, healthchecks, per global conventions).

### Out of v1 (deferred, documented so they're deliberate)

- Accounts/auth. Identity is a client-generated UUID (see below). No passwords, no OIDC.
- Native mobile builds. The Flutter codebase keeps mobile-compatible (no web-only plugins in core flows), but only web is built and verified tonight.
- Push notifications, room expiry/cleanup jobs, restaurant detail pages, maps.
- Live Google Places verification (no API key available overnight — see provider section).

## [DECISION] Match semantics

The source spec says both "when there's a match, it shows up in your matches list" and "any time a room is swiped on, it counts as a match... sorted by most to least swipes." These conflict under the Tinder reading (match = everyone likes it).

**Chosen:** a restaurant enters the match list as soon as it has ≥1 like; the list is sorted by like count descending, ties broken by most recent like. The UI shows the like count per match (e.g. "3 of 4 liked this") so unanimous picks are still obvious at the top.

**Why:** the spec's explicit sentence ("any time... counts as a match, sorted by most to least swipes") is the more specific instruction; the unanimous-only reading makes "sorted by most to least swipes" meaningless. This also degrades gracefully for solo users.

## [DECISION] Identity: anonymous UUID

Clients generate a UUID v4 on first launch, persist it locally (localStorage / shared_preferences), and send it as `X-Dinnermate-User` on every request. Display names are per-room (you give one when joining). Lists are owned by the UUID.

**Why:** "time to first swipe" rules out signup. A header UUID is enough to attribute swipes and list ownership. Known limitation (documented in the report): anyone who learns a UUID can act as that user; acceptable for v1's threat model (picking dinner). Clean upgrade path to real auth later — the UUID becomes a column on a users table.

## [DECISION] Restaurant data: provider trait, seed-first

The source spec leaves "what API to query restaurants" open. Options considered:

1. **Google Places (New)** — best data (ratings, price level, photos, open hours). Needs an API key + billing. Can't be exercised tonight.
2. **Yelp Fusion** — good data, needs a key, stricter ToS on caching.
3. **OSM/Overpass** — free, no key, but no ratings or price data, so the room filters in the spec can't work.
4. **Seeded dataset** — full control, works offline, but not real-world data.

**Chosen:** a `RestaurantProvider` trait in `dinnermate-core` (dependency-injected into the room service, per global conventions), with two implementations:

- `SeedProvider` (default): ~60 hand-written restaurants across cuisines/price tiers/ratings, loaded from an embedded JSON file, filtered in-process by the room parameters. Deterministic, fully testable, demoable tonight.
- `GooglePlacesProvider`: implemented against the Places API (New) `places:searchNearby` endpoint, selected when `GOOGLE_PLACES_API_KEY` is set. Code-complete with unit tests against a stubbed HTTP layer, but **not verified against the live API** — flagged in the overnight report.

Provider selection is config (`RESTAURANT_PROVIDER=seed|google`), so swapping to Yelp later is one new impl.

**Why not pick one real API now:** every keyed option is unverifiable overnight, and the trait is needed anyway for tests. Seed-first gives a working, demoable product by morning; Google is the recommended production choice and is one env var away.

## [DECISION] Deck snapshotting

When a room is created, the provider is queried once and the resulting restaurants are **copied into the room** (`room_restaurants` snapshot table). All participants swipe the same deck in the same order.

**Why:** live provider queries per participant would give different participants different decks (provider ordering/availability changes), breaking the matching concept entirely. Snapshot also caps provider cost at one query per room and makes match counts well-defined. Trade-off: data staleness over a room's life — irrelevant at "pick dinner tonight" timescales.

## [DECISION] Realtime: polling

The matches panel polls `GET /rooms/{code}/matches` every 3 seconds while the room screen is open.

**Why:** simplest thing that works everywhere (including Flutter web behind proxies). At v1 scale, polling cost is trivial. SSE/WebSocket is a contained upgrade later (one endpoint, one client service). Considered SSE — rejected for tonight because it complicates the nginx/Coolify proxy config and buys nothing at this scale.

## Architecture

Mirrors the proven layout of `fulfilled` (Spencer's existing axum + Flutter project):

```
dinnermate-fable/
├── server/                  Rust workspace
│   ├── crates/
│   │   ├── dinnermate-core  domain types, services, RestaurantProvider trait
│   │   ├── dinnermate-db    sqlx/Postgres repositories, pool, migrations runner
│   │   └── dinnermate-api   axum HTTP layer, DTOs, error mapping, /healthz
│   ├── migrations/          sqlx migrations
│   └── specs/openapi.yaml   source of truth for the wire surface
├── client/                  Flutter app (web-first)
├── compose.coolify.yaml     two services: api + web (nginx), external Postgres
└── docker-compose.yml       local dev: postgres + api + web
```

- **dinnermate-core** has no I/O dependencies. Services take repositories and the provider as trait objects (constructor injection). All matching/filter logic lives here and is unit-tested here.
- **dinnermate-db** implements the repository traits with sqlx against Postgres 17. Migrations run on API boot (same pattern as fulfilled).
- **dinnermate-api** is thin: parse/validate DTOs, call services, map domain errors to status codes. Routes versioned under `/api/v1`.

### Data model

```
rooms            id, code (unique, 6 chars A-Z0-9 minus ambiguous), name,
                 location_lat, location_lng, location_label, radius_m,
                 cuisines text[], price_min, price_max (1-4),
                 min_rating, created_by, created_at
room_restaurants room_id, restaurant_id, position, name, cuisine, price_level,
                 rating, rating_count, address, photo_url, lat, lng
                 (snapshot — no FK to a global restaurants table)
participants     id, room_id, user_id, display_name, joined_at
                 (unique on room_id+user_id)
swipes           room_id, participant_id, restaurant_id, liked, created_at
                 (unique on room_id+participant_id+restaurant_id)
lists            id, code (unique), name, owner_user_id, created_at
list_items       id, list_id, name, cuisine, price_level, rating, address,
                 photo_url, added_by_user_id, source_restaurant_id, created_at
```

Matches are a query, not a table: `count(*) filter (where liked)` over swipes grouped by restaurant, `having count > 0`, ordered by count desc, latest like desc.

### HTTP surface (v1)

```
POST /api/v1/rooms                      create room (params) → {code, room}
GET  /api/v1/rooms/{code}               room info + deck
POST /api/v1/rooms/{code}/join          {display_name} → participant
POST /api/v1/rooms/{code}/swipes        {restaurant_id, liked}
GET  /api/v1/rooms/{code}/matches       sorted matches + like counts + participant count
POST /api/v1/lists                      {name} → {code, list}
GET  /api/v1/lists/{code}               list + items
POST /api/v1/lists/{code}/items         add item (from a room restaurant or free-form)
GET  /api/v1/lists?mine=true            lists owned by X-Dinnermate-User
GET  /healthz                           liveness for Coolify
```

All non-health routes require the `X-Dinnermate-User` header.

### Error handling

Domain errors in core (`RoomNotFound`, `AlreadySwiped`, `InvalidParams`, …) map to 404/409/422 in the api crate's single error mapper. Provider failures at room creation return 502 with a friendly message; the room is not created (no half-initialized rooms).

## Flutter client

Web-first, mobile-compatible. Screens:

1. **Home** — two big actions: "Start a room" / "Join a room", plus "My lists". Joining = enter code (or arrive via `/#/r/CODE` deep link from a shared URL) + display name → straight into swiping.
2. **Create room** — single scrollable form (location text + preset city fallback, radius slider, cuisine chips, price range, min rating), big "Create" button → share sheet with code + copyable link.
3. **Swipe** — card stack (photo, name, cuisine, price $-signs, rating), swipe right/left + explicit ❤️/✖️ buttons (large targets per the vibe), running match ticker at the bottom; tapping it opens matches.
4. **Matches** — sorted list with like counts, "add to list" action per match.
5. **Lists** — my lists, list detail with items + adds, share code.

State: `provider`-style injection with plain `ChangeNotifier`s (no heavy framework); an `ApiClient` injected at the root. Visual direction per spec: soft palette (warm neutrals + a coral/peach accent), large rounded cards, generous spacing, airy page transitions. The Figma link in the source doc is auth-walled and labeled "old design" — treated as inspiration-only, not followed.

**[DECISION] Location input:** v1 uses a text label + a small set of preset cities with coordinates, plus browser geolocation when available. No geocoding API dependency tonight (same no-key constraint as Places). With the Google provider enabled later, geocoding can ride the same key.

## Testing

- Core: table-driven unit tests for filter logic, match sorting, code generation, swipe idempotency rules.
- DB: repository tests against a dockerized Postgres (same pattern as fulfilled), run in CI-style script.
- API: integration tests with seed provider + test DB covering the full room flow and error mapping.
- Google provider: unit tests against stubbed HTTP responses only.
- Client: `flutter analyze` + widget tests for the swipe deck and join flow, run inside the cirruslabs Flutter Docker image (no local SDK).
- End-to-end: docker-compose up, scripted curl smoke test of create→join→swipe→match; client smoke-checked by hitting the built web bundle.

## Deploy

Per global Coolify conventions: API Dockerfile exposes 8080 with a `HEALTHCHECK` on `/healthz`; web Dockerfile is the two-stage cirruslabs-flutter→nginx build; `compose.coolify.yaml` with external Postgres. Config baked into images; secrets via Coolify env vars. Actual Coolify app creation is left for Spencer (outward-facing).
