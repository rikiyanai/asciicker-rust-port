# /test-e2e

Run a feature-targeted E2E scenario with screenshot-first logs.

## Command

```bash
just test-e2e feature=full
```

Feature options:

- `smoke`
- `menu`
- `water`
- `full`

Example:

```bash
just test-e2e feature=water base_url=http://127.0.0.1:4173
```
