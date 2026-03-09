# /test-parallel

Run multiple user stories in parallel with isolated browser contexts.

## Command

```bash
just test-parallel suite=core workers=3
```

Suites:

- `smoke`
- `menu`
- `water`
- `core` (smoke + menu + water)

Example:

```bash
just test-parallel suite=core workers=4 base_url=http://127.0.0.1:4173
```
