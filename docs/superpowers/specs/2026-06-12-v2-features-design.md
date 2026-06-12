# Dinnermate v2 Features — Design

**Date:** 2026-06-12
**Status:** Approved by Spencer (interactive brainstorm; option choices recorded below).

Four features: list membership & invites, walk/drive radius picker, hours on cards, restaurant details (card flip + details page).

## 1. List membership & invites

**Chosen:** explicit Join button (vs auto-join on open / auto-join on first edit).

### Data

```sql
-- migration 0002 (shared with §3)
CREATE TABLE list_members (
    list_id UUID NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (list_id, user_id)
);
CREATE INDEX idx_list_members_user ON list_members (user_id);
```

Owner gets a membership row inside `ListRepo::create` (same transaction). Uniform membership queries; no owner special-casing. Backfill: insert membership rows for existing list owners in the migration (`INSERT INTO list_members (list_id, user_id) SELECT id, owner_user_id FROM lists`).

### Behavior

- `POST /lists/{code}/join` — idempotent; creates membership; 200 with the list.
- `DELETE /lists/{code}/members/me` — leave; owner cannot leave (422 `OWNER_CANNOT_LEAVE`); 204.
- `GET /lists` — owned + joined, each `ListDto` gains `is_owner: bool`, ordered joined_at desc.
- `GET /lists/{code}` — still public with the code (read-only preview); response gains `is_member`, `is_owner`.
- `POST /lists/{code}/items` — now requires membership → 403 `NOT_LIST_MEMBER` otherwise. **Behavior change from v1** (was open to anyone with the code) — intended.
- Core: `ListService` gains `join`, `leave`, membership check in `add_item`; `mine` becomes `lists_for_member`. `CoreError` gains `NotListMember` (403) and `OwnerCannotLeave` (422).

### Client

- Non-member opening `/#/l/CODE`: read-only preview + full-width **Join this list** button → join → editable view, list now in My Lists.
- Owner detail screen: share affordance (bottom sheet: copy invite link `${origin}/#/l/CODE`, copy code).
- Member (non-owner) detail screen: leave action in overflow menu.
- My Lists: joined-but-not-owned lists show a "shared" badge.

## 2. Radius: Walk/Drive toggle

**Chosen:** mode toggle (vs logarithmic slider — recorded in `FUTURE.md` as a liked alternative; vs preset chips).

Client-only change in `create_room.dart`:

- `SegmentedButton` with 🚶 Walking / 🚗 Driving.
- Walking: 250–2,000m, step 250m, label "1.0 km · ~12 min walk" (80 m/min walking speed, rounded to nearest minute).
- Driving: 2–40km, step 1km, label "8 km".
- Default: Walking, 1km. Switching modes clamps the value into the new range (walking→driving sets 5km; driving→walking sets 1km).
- Server unchanged (`radius_m` already validates 100–40,000).

## 3. Hours on cards (no open-now filter)

**Chosen:** display hours, no exclusion filter (vs open-now toggle / open-at-meal-time). Meal-time selection recorded in `FUTURE.md`.

### Model

```rust
pub struct HoursPeriod { pub day: u8, /* 0=Sun..6=Sat */ pub open: String, /* "HH:MM" */ pub close: String }
// Restaurant gains:
pub hours: Option<Vec<HoursPeriod>>,      // None = unknown
pub utc_offset_minutes: Option<i32>,      // restaurant-local tz offset
```

Open/closed is computed **in the restaurant's local time** (`Utc::now() + utc_offset_minutes`), in `dinnermate-core` (`hours.rs`: `open_status(periods, offset, now) -> OpenStatus { Open { until }, Closed { opens_next }, Unknown }`). Handles midnight-crossing periods (close < open ⇒ closes next day) and days with no periods (closed). Client mirrors the same logic in Dart for live display (`hours.dart`); core's version serves any future server-side filtering.

### Plumbing

- Migration 0002: `room_restaurants` gains `hours JSONB`, `utc_offset_minutes INTEGER` (both nullable).
- Google provider: add `places.regularOpeningHours,places.utcOffsetMinutes` to the field mask; map `regularOpeningHours.periods[].{open,close}.{day,hour,minute}` → `HoursPeriod`. Missing → `None`.
- Seed data: synthetic hours for all 60 entries with variety — standard 11:00–22:00, lunch-only (11:00–14:30), late-night crossing midnight (17:00–01:00), a few closed Mondays, two 24h (00:00–24:00 represented as 00:00–23:59), a handful `null` (unknown) to exercise the Unknown UI state.
- Wire: `RestaurantDto` gains `hours`, `utc_offset_minutes`. OpenAPI updated.

### UI

Badge on card back (§4) and details page: "Open · until 22:00" (green-tinted chip) / "Closed · opens 17:00" (neutral) / nothing when unknown.

## 4. Restaurant details: card flip + details page

**Chosen:** snapshot + on-demand details endpoint with server cache (vs fatter snapshot / client-side Google calls — the latter rejected outright: leaks the API key).

### Card flip (swipe screen)

Tap toggles a 3D Y-axis flip (`AnimatedBuilder` + `Transform`, ~300ms) between card front (unchanged) and back: name, open/closed badge + today's hours, full address, cuisine chip, $-signs, "★ 4.6 (1,204 ratings)". Snapshot data only — no network. Swipe gestures and the ❤️/✖️ buttons keep working while flipped; advancing the deck resets to front.

### Details endpoint

`GET /api/v1/rooms/{code}/restaurants/{restaurant_id}/details` (auth header as usual; restaurant must be in the room's deck → 404 otherwise):

```json
{
  "restaurant": { ...RestaurantDto from snapshot... },
  "website": "https://…" | null,
  "phone": "+1 …" | null,
  "maps_url": "https://maps.google.com/…" | null,
  "reviews": [{ "author": "…", "rating": 5, "text": "…", "relative_time": "2 months ago" }]
}
```

- `RestaurantProvider` trait gains `async fn details(&self, restaurant_id: &str) -> Result<ProviderDetails, ProviderError>` where `ProviderDetails { website, phone, maps_url, reviews }`.
- Seed: returns embedded website/phone for ~half the entries, empty reviews (never fabricate review text).
- Google: Place Details `GET /v1/places/{id}` with field mask `websiteUri,nationalPhoneNumber,googleMapsUri,reviews`; reviews truncated to 5.
- Cache: `restaurant_details_cache (restaurant_id TEXT PRIMARY KEY, payload JSONB NOT NULL, fetched_at TIMESTAMPTZ NOT NULL)` (migration 0002), TTL 24h checked in the service; provider hit on miss/stale. Provider failure with a warm-but-stale cache entry serves the stale copy.

### Details page (client)

Route `/r/:code/d/:restaurantId`, reached by tapping a match tile (match tiles keep their existing add-to-list trailing button). Layout: hero header (emoji/photo gradient), name + open badge, map (`flutter_map` + OSM standard tiles, single pin, ~200px, non-interactive lite mode), hours for the week (today bolded), action row (website / call / directions — launch URLs; hidden when null), reviews section (hidden when empty). Loading + friendly-error states as elsewhere.

New client deps: `flutter_map` (+ `latlong2`), `url_launcher`.

## Cross-cutting

- **OpenAPI:** all new/changed routes and DTOs in the same commits (per `server/AGENTS.md`).
- **FUTURE.md** (new, repo root): logarithmic radius slider; meal-time selection + open-at-time filtering; both attributed to this design discussion.
- **Compatibility:** existing rooms have `NULL` hours — UI shows no badge (Unknown). Existing lists get owner membership backfilled in 0002. The `add_item` permission tightening is the only behavior break.
- **Deploy:** push to `main` auto-deploys via the verified webhook. Google-only fields stay empty until `RESTAURANT_PROVIDER=google` is enabled in Coolify.

## Testing

- **Core:** table-driven `open_status` tests (open span, closed day, midnight crossing, 24h, unknown, boundary minutes); membership rules (join idempotent, leave, owner-cannot-leave, add_item gated); details caching (fresh hit no provider call, stale → refetch, provider error → stale served).
- **DB:** migration applies on a v1 schema with data (backfill verified); list_members CRUD; cache upsert.
- **API:** join/leave/members flows incl. 403 on non-member add; details endpoint (in-deck 200, foreign id 404); `GET /lists` includes joined.
- **Client:** widget tests — card flip toggles faces and swipe still fires; join-button vs editable states; hours badge rendering for each OpenStatus; Dart `open_status` table mirroring core's cases.
- **Smoke:** extend with list join (user B joins A's list, adds item, A sees it) and details endpoint fetch.
