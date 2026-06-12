# Agent instructions

## Keep the OpenAPI spec in sync with HTTP routes

`specs/openapi.yaml` is the source of truth for the public HTTP surface and is
consumed by the Flutter client. **Whenever you change anything that affects the
wire shape, update `specs/openapi.yaml` in the same change.**

This applies to any of the following:

- Adding, removing, or renaming a route in `crates/dinnermate-api/src/routes/`.
- Changing a path parameter, query parameter, or HTTP method.
- Adding, removing, or renaming a field on a request or response DTO
  (anything `#[derive(Serialize)]` or `#[derive(Deserialize)]` in the routes
  module, or any flattened/nested type it references).
- Changing a field's type, nullability, or whether it's required.
- Adding or changing a status code, or changing the mapping in
  `crates/dinnermate-api/src/error.rs` between `CoreError` and HTTP responses.
- Changing the `X-Dinnermate-User` header requirement on a route.

When updating the spec:

1. Edit `specs/openapi.yaml` alongside the code change — not in a follow-up.
2. Reuse existing `components/schemas` entries where possible; only add a
   new schema when the shape is genuinely new.
3. Validate the spec parses before committing:

   ```sh
   python3 -c "import yaml; yaml.safe_load(open('specs/openapi.yaml'))"
   ```

4. If you add or remove a route, also confirm the `tags` and the path's
   placement under the correct tag still make sense.

If a change is purely internal (service plumbing, repo layer, SQL,
non-wire DTOs) it does not require a spec update.
