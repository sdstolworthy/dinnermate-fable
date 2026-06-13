# Future ideas

Parked deliberately — each came out of a design discussion and was liked but cut for scope.

## Logarithmic radius slider (2026-06-12)

v2 shipped the 🚶 Walking / 🚗 Driving mode toggle. The liked alternative: one slider, 250m–40km on a log scale, where half the slider travel covers ≤2km. No mode concept to explain, and fine walking-distance resolution falls out naturally. Worth revisiting if the toggle feels clunky or a third persona (cyclists? transit?) appears.

## Broader OSM amenity coverage (2026-06-12)

The Overpass query only matches `amenity=restaurant`. Cafes, fast food, and food courts (`amenity~"^(restaurant|fast_food|cafe|food_court)$"`) are one query change away — needs a product decision about whether Dinnermate is dinner-only.

---

# Shipped

## Meal-time selection + open-at-time filtering (2026-06-12) — **shipped 2026-06-12**

v2 shows open/closed hours on cards and lets humans decide. The bigger idea: room creation gets an optional "When are you eating?" (now / tonight / pick a time), and the deck filter excludes restaurants closed at that time — the Friday-date persona often creates a room at 3pm for 7pm, where "open now" is the wrong question. Requires hours coverage to be good (i.e., the Google provider in production) before filtering on it is fair to restaurants with unknown hours.

## Timezone offsets for OSM restaurants (2026-06-12) — **shipped 2026-06-12**

OSM gives opening hours but no timezone, so OSM-sourced cards show weekly hours without the live open/closed badge (`utc_offset_minutes` is None). `tzf-rs` (pure-Rust lat/lng → timezone) would fix this at the cost of a chunky embedded dataset. Revisit if the missing badge annoys.

