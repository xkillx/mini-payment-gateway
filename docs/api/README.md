# API Documentation

The OpenAPI contract is at `docs/api/openapi.yaml`.

This file is currently manually maintained. As domain behavior is implemented in future tickets, this spec should be updated to reflect the actual request/response schemas.

## How to Refresh

1. Update route handlers and DTOs in each domain crate.
2. Add matching schemas to the `components/schemas` section.
3. Add/update path definitions with proper request bodies and responses.

Future automation may use `utoipa` for code-first OpenAPI generation.
