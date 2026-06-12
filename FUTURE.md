# Future ideas

Parked deliberately — each came out of a design discussion and was liked but cut for scope.

## Logarithmic radius slider (2026-06-12)

v2 shipped the 🚶 Walking / 🚗 Driving mode toggle. The liked alternative: one slider, 250m–40km on a log scale, where half the slider travel covers ≤2km. No mode concept to explain, and fine walking-distance resolution falls out naturally. Worth revisiting if the toggle feels clunky or a third persona (cyclists? transit?) appears.

## Meal-time selection + open-at-time filtering (2026-06-12)

v2 shows open/closed hours on cards and lets humans decide. The bigger idea: room creation gets an optional "When are you eating?" (now / tonight / pick a time), and the deck filter excludes restaurants closed at that time — the Friday-date persona often creates a room at 3pm for 7pm, where "open now" is the wrong question. Requires hours coverage to be good (i.e., the Google provider in production) before filtering on it is fair to restaurants with unknown hours.
