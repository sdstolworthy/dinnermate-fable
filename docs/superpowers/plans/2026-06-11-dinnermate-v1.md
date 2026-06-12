# Dinnermate v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Working Dinnermate v1 — Rust API + Flutter web client — covering room create/join/swipe/match and shareable lists, runnable via docker-compose and deployable to Coolify.

**Architecture:** Three-crate axum workspace (`dinnermate-core` domain + traits, `dinnermate-db` sqlx/Postgres repos, `dinnermate-api` HTTP) mirroring `/workplace/fulfilled/server`. Flutter web client built via the cirruslabs Docker image (no local SDK). Restaurant data behind a `RestaurantProvider` trait: `SeedProvider` default, `GooglePlacesProvider` env-gated.

**Tech Stack:** Rust 1.95, axum 0.8, sqlx 0.8 (postgres, runtime-tokio), tokio, async-trait, thiserror, uuid, chrono, rand; Flutter stable (go_router, provider, http, shared_preferences); Postgres 17-alpine; nginx.

**Spec:** `docs/superpowers/specs/2026-06-11-dinnermate-design.md` — read it first; it defines match semantics, identity model, and scope cuts.

---

## Shared contracts (referenced by all tasks — do not drift)

### Domain types (`dinnermate-core`)

```rust
pub struct Restaurant {
    pub id: String,            // provider-scoped id, e.g. "seed-001"
    pub name: String,
    pub cuisine: String,       // lowercase single tag, e.g. "thai"
    pub price_level: u8,       // 1..=4
    pub rating: f32,           // 0.0..=5.0
    pub rating_count: u32,
    pub address: String,
    pub photo_url: Option<String>,
    pub lat: f64,
    pub lng: f64,
}

pub struct RoomParams {
    pub lat: f64,
    pub lng: f64,
    pub location_label: String,
    pub radius_m: u32,             // 100..=40_000
    pub cuisines: Vec<String>,     // empty = all
    pub price_min: u8,             // 1..=4, min<=max
    pub price_max: u8,
    pub min_rating: f32,           // 0.0..=5.0
}

pub struct Room { pub id: Uuid, pub code: String, pub name: Option<String>,
    pub params: RoomParams, pub created_by: Uuid, pub created_at: DateTime<Utc> }

pub struct Participant { pub id: Uuid, pub room_id: Uuid, pub user_id: Uuid,
    pub display_name: String, pub joined_at: DateTime<Utc> }

pub struct MatchEntry { pub restaurant: Restaurant, pub like_count: i64,
    pub last_liked_at: DateTime<Utc> }

pub struct List { pub id: Uuid, pub code: String, pub name: String,
    pub owner_user_id: Uuid, pub created_at: DateTime<Utc> }

pub struct ListItem { pub id: Uuid, pub list_id: Uuid, pub name: String,
    pub cuisine: Option<String>, pub price_level: Option<u8>, pub rating: Option<f32>,
    pub address: Option<String>, pub photo_url: Option<String>,
    pub added_by_user_id: Uuid, pub source_restaurant_id: Option<String>,
    pub created_at: DateTime<Utc> }
```

### Core traits

```rust
#[async_trait]
pub trait RestaurantProvider: Send + Sync {
    async fn search(&self, params: &RoomParams) -> Result<Vec<Restaurant>, ProviderError>;
}

#[async_trait]
pub trait RoomRepo: Send + Sync {
    async fn create(&self, room: &Room, deck: &[Restaurant]) -> Result<(), RepoError>;
    async fn find_by_code(&self, code: &str) -> Result<Option<(Room, Vec<Restaurant>)>, RepoError>;
    async fn join(&self, room_id: Uuid, user_id: Uuid, display_name: &str) -> Result<Participant, RepoError>;
    async fn find_participant(&self, room_id: Uuid, user_id: Uuid) -> Result<Option<Participant>, RepoError>;
    async fn record_swipe(&self, room_id: Uuid, participant_id: Uuid, restaurant_id: &str, liked: bool) -> Result<(), RepoError>; // RepoError::Conflict on duplicate
    async fn matches(&self, room_id: Uuid) -> Result<Vec<MatchEntry>, RepoError>;
    async fn participant_count(&self, room_id: Uuid) -> Result<i64, RepoError>;
}

#[async_trait]
pub trait ListRepo: Send + Sync {
    async fn create(&self, list: &List) -> Result<(), RepoError>;
    async fn find_by_code(&self, code: &str) -> Result<Option<(List, Vec<ListItem>)>, RepoError>;
    async fn add_item(&self, item: &ListItem) -> Result<(), RepoError>;
    async fn lists_for_owner(&self, owner: Uuid) -> Result<Vec<List>, RepoError>;
}
```

Errors: `ProviderError { Unavailable(String), InvalidResponse(String) }`;
`RepoError { NotFound, Conflict, Database(String) }`;
`CoreError { RoomNotFound, ListNotFound, NotInRoom, AlreadySwiped, UnknownRestaurant, InvalidParams(String), Provider(ProviderError), Repo(RepoError) }`.

API error mapping: `RoomNotFound`/`ListNotFound` → 404 · `AlreadySwiped` → 409 · `NotInRoom` → 403 · `UnknownRestaurant`/`InvalidParams` → 422 · `Provider(_)` → 502 · `Repo(_)` → 500. Body: `{"error": {"code": "ALREADY_SWIPED", "message": "..."}}`.

### Database schema (single migration `0001_init.sql`)

```sql
CREATE TABLE rooms (
    id UUID PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT,
    location_lat DOUBLE PRECISION NOT NULL,
    location_lng DOUBLE PRECISION NOT NULL,
    location_label TEXT NOT NULL,
    radius_m INTEGER NOT NULL,
    cuisines TEXT[] NOT NULL DEFAULT '{}',
    price_min SMALLINT NOT NULL,
    price_max SMALLINT NOT NULL,
    min_rating REAL NOT NULL,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE room_restaurants (
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    restaurant_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    cuisine TEXT NOT NULL,
    price_level SMALLINT NOT NULL,
    rating REAL NOT NULL,
    rating_count INTEGER NOT NULL,
    address TEXT NOT NULL,
    photo_url TEXT,
    lat DOUBLE PRECISION NOT NULL,
    lng DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (room_id, restaurant_id)
);

CREATE TABLE participants (
    id UUID PRIMARY KEY,
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    display_name TEXT NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (room_id, user_id)
);

CREATE TABLE swipes (
    room_id UUID NOT NULL,
    participant_id UUID NOT NULL REFERENCES participants(id) ON DELETE CASCADE,
    restaurant_id TEXT NOT NULL,
    liked BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (room_id, participant_id, restaurant_id),
    FOREIGN KEY (room_id, restaurant_id)
        REFERENCES room_restaurants(room_id, restaurant_id) ON DELETE CASCADE
);

CREATE TABLE lists (
    id UUID PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    owner_user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE list_items (
    id UUID PRIMARY KEY,
    list_id UUID NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    cuisine TEXT,
    price_level SMALLINT,
    rating REAL,
    address TEXT,
    photo_url TEXT,
    added_by_user_id UUID NOT NULL,
    source_restaurant_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_swipes_room_liked ON swipes (room_id) WHERE liked;
CREATE INDEX idx_lists_owner ON lists (owner_user_id);
```

Matches query (lives in `RoomRepo::matches`):

```sql
SELECT rr.*, count(*) AS like_count, max(s.created_at) AS last_liked_at
FROM swipes s
JOIN room_restaurants rr ON rr.room_id = s.room_id AND rr.restaurant_id = s.restaurant_id
WHERE s.room_id = $1 AND s.liked
GROUP BY rr.room_id, rr.restaurant_id
ORDER BY like_count DESC, last_liked_at DESC;
```

### HTTP surface (`/api/v1`, all non-health routes require header `X-Dinnermate-User: <uuid>`)

| Method/Path | Request body | Response 2xx |
|---|---|---|
| `POST /rooms` | `{name?, location_label, lat, lng, radius_m, cuisines[], price_min, price_max, min_rating}` | 201 `{room: RoomDto, deck: [RestaurantDto]}` |
| `GET /rooms/{code}` | — | 200 `{room, deck, me: ParticipantDto?}` |
| `POST /rooms/{code}/join` | `{display_name}` | 200 `{participant}` (idempotent re-join returns existing) |
| `POST /rooms/{code}/swipes` | `{restaurant_id, liked}` | 201 `{}` |
| `GET /rooms/{code}/matches` | — | 200 `{matches: [{restaurant, like_count, last_liked_at}], participant_count}` |
| `POST /lists` | `{name}` | 201 `{list}` |
| `GET /lists` | — | 200 `{lists: [ListDto]}` (owned by caller) |
| `GET /lists/{code}` | — | 200 `{list, items: [ListItemDto]}` |
| `POST /lists/{code}/items` | `{name, cuisine?, price_level?, rating?, address?, photo_url?, source_restaurant_id?}` | 201 `{item}` |
| `GET /healthz` | — | 200 `ok` |

DTOs serialize snake_case; `RoomDto` flattens params (same field names as request). Room creator is **not** auto-joined; the client calls join after create.

### Room/list code generation

`generate_code(rng) -> String`: 6 chars from alphabet `ABCDEFGHJKMNPQRSTUVWXYZ23456789` (no I/L/O/0/1). Collision handling: retry on unique violation, max 5 attempts.

### Config (env)

`DATABASE_URL`, `BIND_ADDR` (default `0.0.0.0:8080`), `RESTAURANT_PROVIDER` (`seed` default | `google`), `GOOGLE_PLACES_API_KEY` (required iff provider=google), `CORS_ALLOWED_ORIGINS` (comma-separated, default `*` for v1).

---

## Task 0: Repo scaffolding

**Files:** Create `server/Cargo.toml` (workspace), `server/rust-toolchain.toml`, `server/crates/{dinnermate-core,dinnermate-db,dinnermate-api}/Cargo.toml` + `src/lib.rs` stubs (api gets `src/main.rs`), `.gitignore` (target/, build/, .dart_tool/, pubspec.lock, web/), `README.md` (one-paragraph stub), `docker-compose.yml` (postgres 17-alpine on 55432 + api + web), `server/AGENTS.md` (copy the openapi-sync convention from fulfilled, adapted).

- [ ] Workspace builds: `cd server && cargo build` → success
- [ ] `docker compose up -d postgres` → healthy; `docker compose exec postgres pg_isready` → accepting
- [ ] Commit: `chore: scaffold workspace and dev compose`

Workspace deps (pin in `[workspace.dependencies]`): axum 0.8 (macros), tokio 1 (full), sqlx 0.8 (runtime-tokio-rustls, postgres, uuid, chrono, migrate), serde 1 (derive), serde_json 1, uuid 1 (v4, serde), chrono 0.4 (serde, clock), thiserror 2, async-trait 0.1, rand 0.9, tracing + tracing-subscriber, tower-http 0.6 (cors, trace), reqwest 0.12 (json, rustls-tls, no default features).

## Task 1: dinnermate-core — types, errors, code gen, deck filtering

**Files:** Create `crates/dinnermate-core/src/{lib.rs,model.rs,error.rs,code.rs,provider.rs,filter.rs}`; tests inline `#[cfg(test)]`.

- [x] Write table-driven tests first: `RoomParams::validate` (radius bounds, price ordering, rating range, lat/lng range — each case one assertion), `generate_code` (length 6, alphabet-only, deterministic with seeded `StdRng`), `filter::apply(params, restaurants)` (cuisine empty=all, cuisine match, price window, min_rating, haversine radius cut, stable ordering by rating desc then name)
- [x] Run: `cargo test -p dinnermate-core` → FAIL (unimplemented)
- [x] Implement model.rs (types above), error.rs, code.rs, provider.rs (trait + ProviderError), filter.rs (haversine + predicate chain)
- [x] `cargo test -p dinnermate-core` → PASS; commit `feat(core): domain types, validation, code gen, deck filter`

## Task 2: dinnermate-core — services with fake repos

**Files:** Create `crates/dinnermate-core/src/{repo.rs,service/mod.rs,service/rooms.rs,service/lists.rs}`, `crates/dinnermate-core/src/testing.rs` (in-crate fakes: `FakeRoomRepo`, `FakeListRepo`, `FakeProvider` — HashMap/Mutex backed, behind `#[cfg(feature = "testing")]` or plain module used by tests).

`RoomService::new(repo: Arc<dyn RoomRepo>, provider: Arc<dyn RestaurantProvider>, rng: ...)`. Methods: `create_room(user, CreateRoom) -> (Room, Vec<Restaurant>)` (validate → provider.search → filter → error InvalidParams("no restaurants match") if empty → snapshot via repo, code-retry loop), `get_room(code, user)`, `join(code, user, display_name)` (idempotent: return existing participant), `swipe(code, user, restaurant_id, liked)` (must be participant → NotInRoom; unknown id → UnknownRestaurant; repo Conflict → AlreadySwiped), `matches(code)`. `ListService`: `create(owner, name)`, `get(code)`, `add_item(code, user, NewItem)`, `mine(owner)`.

- [x] Tests first (fakes): create_room happy path snapshots filtered deck; empty deck → InvalidParams; join twice → same participant id; swipe before join → NotInRoom; duplicate swipe → AlreadySwiped; unknown restaurant → UnknownRestaurant; matches sorted by like_count desc (seed fake with swipes from 3 participants); list add by non-owner succeeds (shared lists are open by code)
- [x] `cargo test -p dinnermate-core` → PASS; commit `feat(core): room and list services`

## Task 3: SeedProvider

**Files:** Create `crates/dinnermate-core/src/seed.rs`, `crates/dinnermate-core/data/seed_restaurants.json` (embed via `include_str!`).

Dataset: 60 restaurants, ids `seed-001..seed-060`, ≥8 cuisines (mexican, thai, italian, japanese, indian, american, chinese, mediterranean, korean, vietnamese), price levels 1–4 distributed, ratings 3.2–4.9 with varied rating_count, photo_url null (client renders cuisine-colored placeholder cards), coordinates clustered around a fictional downtown (40.7600±0.05, -111.8900±0.05) so radius filtering is exercisable. SeedProvider ignores lat/lng *center* (returns all; filter applies radius) — document in rustdoc.

- [x] Tests: JSON parses; 60 entries; all ids unique; all fields within validation ranges; `search` honors filter via service path
- [x] `cargo test -p dinnermate-core` → PASS; commit `feat(core): embedded seed restaurant provider`

## Task 4: dinnermate-db — migrations + repos

**Files:** Create `server/migrations/0001_init.sql` (schema above), `crates/dinnermate-db/src/{lib.rs,pool.rs,room_repo.rs,list_repo.rs,error.rs}`, `crates/dinnermate-db/tests/repo_test.rs`, `server/scripts/test-db.sh` (starts disposable postgres on 55433, exports `TEST_DATABASE_URL`, runs `cargo test -p dinnermate-db -- --test-threads=1`, tears down).

`PgRoomRepo`/`PgListRepo` implement core traits with `sqlx::query` (runtime-checked — no sqlx offline/prepare complexity). `record_swipe`: map unique-violation (`23505`) to `RepoError::Conflict`. `pool.rs`: `connect_and_migrate(url)` runs `sqlx::migrate!("../../migrations")`.

- [x] Tests (each in fresh schema or serialized): create+find_by_code roundtrip incl. deck order by position; join unique(room,user) → second join via `find_participant`; record_swipe duplicate → Conflict; matches query ordering (3 participants, varied likes — assert exact order and counts); lists roundtrip + add_item + lists_for_owner
- [x] Run: `./scripts/test-db.sh` → PASS; commit `feat(db): postgres repos and initial migration`

## Task 5: dinnermate-api — axum server + integration tests

**Files:** Create `crates/dinnermate-api/src/{main.rs,server.rs,config.rs,error.rs,extract.rs,routes/mod.rs,routes/rooms.rs,routes/lists.rs,routes/health.rs}`, `crates/dinnermate-api/tests/api_test.rs`.

`server.rs` builds `Router` from injected `AppState { rooms: RoomService, lists: ListService }` so tests construct it with fakes/seed + test DB. `extract.rs`: `UserId` extractor reading `X-Dinnermate-User` (400 `MISSING_USER` if absent/invalid). CORS from config. `main.rs`: tracing init, config from env, pool+migrate, provider select (`seed`/`google` — google wired in Task 7, until then `unimplemented` arm returns config error), serve.

- [x] Integration tests (axum `Router` + `tower::ServiceExt::oneshot`, test DB from `TEST_DATABASE_URL`, seed provider): full flow create room → join 2 users → 3 swipes → matches sorted with participant_count=2; status-code table: missing header 400, bad code 404, duplicate swipe 409, swipe w/o join 403, bad params 422; list flow create → other user adds item → owner GET sees it; `GET /healthz` 200 without header
- [x] Run: `./scripts/test-db.sh` extended to also run `cargo test -p dinnermate-api`; PASS; commit `feat(api): http server with room and list routes`

## Task 6: OpenAPI spec

**Files:** Create `server/specs/openapi.yaml` covering every route/DTO/status code from the table above.

- [ ] Validate: `python3 -c "import yaml; yaml.safe_load(open('server/specs/openapi.yaml'))"` → no output
- [ ] Commit `docs(api): openapi spec`

## Task 7: GooglePlacesProvider

**Files:** Create `crates/dinnermate-core/src/google.rs` (or `dinnermate-api/src/google.rs` if reqwest dep should stay out of core — put it in **api crate**: core stays I/O-free), wire into provider select in `main.rs`.

POST `https://places.googleapis.com/v1/places:searchNearby` with field mask `places.id,places.displayName,places.formattedAddress,places.rating,places.userRatingCount,places.priceLevel,places.location,places.primaryType,places.photos`; map `priceLevel` enum (`PRICE_LEVEL_INEXPENSIVE`=1 … `PRICE_LEVEL_VERY_EXPENSIVE`=4, missing→2), `includedTypes:["restaurant"]`, radius+center from params; photo url via `https://places.googleapis.com/v1/{photo.name}/media?maxWidthPx=800&key=...`. Cuisine from `primaryType` (strip `_restaurant` suffix). Base URL injectable for tests.

- [ ] Tests: stub HTTP server (use `axum` itself as stub on ephemeral port) returning canned JSON → mapping correct incl. missing rating/priceLevel defaults; 403 from API → `ProviderError::Unavailable`
- [ ] `cargo test -p dinnermate-api` → PASS; commit `feat(api): google places provider (unverified against live API)`

## Task 8: Flutter scaffold + API client + models

**Files:** Create `client/pubspec.yaml` (deps: go_router, provider, http, shared_preferences, cupertino_icons; dev: flutter_test, flutter_lints), `client/analysis_options.yaml`, `client/lib/main.dart`, `client/lib/src/{api/api_client.dart,api/models.dart,identity.dart,theme.dart,router.dart}`.

`ApiClient(baseUrl, http.Client, Identity)` — injected; methods mirror the HTTP table, throw `ApiException(code, message, status)`. `Identity` lazily creates/persists UUID via injected `KeyValueStore` abstraction (shared_preferences impl + in-memory for tests). `theme.dart`: ColorScheme.fromSeed coral seed `#FF7E6B`, warm off-white `#FAF6F1` surfaces, `CardTheme` radius 24, chunky `FilledButton` (min height 56), google-fonts NOT used (offline build) — system font with weight hierarchy. Routes: `/` home, `/create`, `/r/:code` room (join-gate), `/r/:code/matches`, `/lists`, `/l/:code`.

All Flutter commands run in Docker:
`docker run --rm -v /workplace/dinnermate-fable/client:/work -w /work ghcr.io/cirruslabs/flutter:stable sh -c "flutter create . --platforms=web && flutter pub get && flutter analyze && flutter test"`

- [ ] Tests: models JSON roundtrip (table-driven per DTO); ApiClient happy/error paths with a fake `http.Client`; Identity persists UUID across instances via in-memory store
- [ ] `flutter analyze` clean, `flutter test` PASS (in Docker); commit `feat(client): scaffold, api client, identity, theme`

## Task 9: Flutter screens

**Files:** Create `client/lib/src/screens/{home.dart,create_room.dart,join_room.dart,swipe.dart,matches.dart,lists.dart,list_detail.dart}`, `client/lib/src/widgets/{restaurant_card.dart,swipe_deck.dart,match_tile.dart,big_button.dart}`, `client/lib/src/state/{room_state.dart,lists_state.dart}` (ChangeNotifiers, ApiClient injected).

Behavior requirements (see spec §Flutter client + §vibe):
- Home: two stacked `BigButton`s (Start a room / Join a room) + lists entry; join sheet takes code + display name.
- Create: form per RoomParams; location = text label + preset city dropdown (Salt Lake City default 40.760, -111.890; SF; NYC; Austin) + "use my location" via `geolocator`-free browser API? — **No: use preset + manual lat/lng advanced expander; no geolocation dep in v1.** On success → share screen with code, copyable `https://<host>/#/r/CODE`, then auto-join → swipe.
- Swipe deck: top card draggable horizontally (custom `GestureDetector` + `AnimatedBuilder`, rotation ±8°, like/nope badge opacity by drag offset), ❤️/✖️ buttons 64px, optimistic POST swipe (409 silently ignored), bottom `MatchTicker` polling matches every 3s showing top match + count, tap → matches screen. Deck end → "You're done — see matches".
- Matches: ranked list, like counts ("3 liked"), add-to-list bottom sheet (picks/creates list, POSTs item).
- Lists: my lists grid + join-by-code; detail shows items, FAB add free-form item, share code chip.
- RestaurantCard with photo if photo_url else cuisine-keyed soft gradient + emoji (🌮🍜🍕🍣🍛🍔🥡🥙🥘🍲 map).

- [ ] Widget tests: swipe deck advances + fires callback on drag past threshold and on button tap; join flow validates empty name; match tile renders count. Fake ApiClient injected via provider.
- [ ] `flutter analyze` clean, `flutter test` PASS (Docker); commit `feat(client): all v1 screens`

## Task 10: Client Dockerfile + nginx

**Files:** Create `client/Dockerfile` (copy fulfilled's two-stage pattern verbatim, ARG `API_BASE_URL=/api`), `client/nginx.conf` (SPA fallback to index.html; `location /api/ { proxy_pass http://api:8080/api/; }` so same-origin in compose; gzip on).

- [x] `docker build -t dinnermate-web client/` → succeeds (this also proves `flutter build web --release` compiles)
- [x] Commit `build(client): web dockerfile and nginx`

## Task 11: API Dockerfile + compose + e2e smoke

**Files:** Create `server/Dockerfile` (multi-stage rust:1.95-slim → debian:bookworm-slim, EXPOSE 8080, `HEALTHCHECK CMD curl -f http://localhost:8080/healthz`), finalize `docker-compose.yml` (postgres + api(seed provider) + web on :8888), `compose.coolify.yaml` (api + web, external DB env), `scripts/smoke.sh`: curl create room → join two users → swipe both → assert matches JSON has expected like_count=2 first entry, assert web `/health.txt` 200, assert deep-link `/#/r/CODE` serves index.html.

- [ ] `docker compose up -d --build` → all healthy; `./scripts/smoke.sh` → "SMOKE OK"
- [ ] Commit `build: compose for dev and coolify, e2e smoke script`

## Task 12: Overnight report + final review

**Files:** Create `OVERNIGHT_REPORT.md` (decisions + rationale links to spec, what's verified with command output, what's NOT verified (Google provider live, Coolify deploy), how to run, suggested next steps), finalize `README.md`.

- [ ] Run full verification suite once more (core+db+api tests, flutter analyze+test, smoke) and paste real output into report
- [ ] Commit `docs: overnight report`

---

## Self-review notes

- **Spec coverage:** rooms ✓ (T1-5), match semantics ✓ (matches query, T4/T5 tests), lists ✓ (T2/T4/T5/T9), browser-no-download ✓ (web client + deep link, T9/T10), time-to-first-swipe ✓ (join = code+name only), provider decision ✓ (T3/T7), vibe ✓ (T8 theme + T9), deploy ✓ (T10/T11). Native mobile builds: out of scope per spec.
- **Type consistency:** `MatchEntry.like_count` is `i64` (SQL count) everywhere; participant idempotency via `find_participant` used by both T2 service and T5 join route; `X-Dinnermate-User` header name consistent.
- **Sequencing:** T1→T2→T3 core-only (no DB needed); T4 needs docker postgres; T5 needs T2+T4; T7 after T5 (lives in api crate); T8→T9→T10 client track is independent of T4-T7 except the HTTP contract (frozen above) — client work may run in parallel with backend.
