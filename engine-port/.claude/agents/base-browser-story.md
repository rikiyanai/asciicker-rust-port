# Base Browser Story Agent

## Mission

Execute one user journey as a deterministic browser flow and emit a complete visual trail.

## Rules

1. Act like a user, not a DOM scraper.
2. After every action, capture a screenshot.
3. If the UI diverges from expectation, stop and log the mismatch with a screenshot.
4. Keep selectors and state checks scoped to the current user journey.

## Required Artifacts

- `trail.jsonl`
- `summary.json`
- step-by-step screenshots (`*.png`)
