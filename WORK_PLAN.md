# WORK_PLAN.md

## 2026-01-29 — On-Demand Package Backfill (Goal: build the core feature in small, shippable steps)

### Definition and boundaries
- Define “package backfill” done criteria in one sentence.
- Decide the single source of truth for “latest checkpoint” in backfill.
- Decide the dedupe rule for `sui_objects` inserts (idempotency).
- Explicitly separate framework watermarks from custom backfill watermarks.
- Ensure custom watermarks live in dedicated tables only.
- Add a rule: never read/write the framework watermark from backfill code.

### Database schema
- Draft `package_backfill_requests` columns and types.
- Draft `package_backfill_watermarks` columns and types.
- Decide unique constraints for request dedupe.
- Confirm table names do not overlap framework-managed tables.
- Write migration filenames and create empty up/down SQL files.
- Fill `up.sql` for requests table.
- Fill `up.sql` for watermarks table.
- Fill `down.sql` for requests table.
- Fill `down.sql` for watermarks table.

### Request intake (backend)
- Add a backend request endpoint to enqueue a package.
- Validate `package_id` input and reject empty values.
- Query for existing request state before enqueue.
- Insert a new request when safe to enqueue.
- Return a stable response payload (status + request id).
- Add backend tests for enqueue behavior.

### Worker skeleton
- Add a backfill worker entrypoint (new binary/command).
- Implement worker loop: fetch request → process or sleep.
- Implement oldest-first selection with row locking.
- Mark request as running before work starts.
- Set request status to done when finished.

### Watermark processing
- Create watermark on first run for a package.
- Load start checkpoint from `sui_packages`.
- Process checkpoints in small fixed-size chunks.
- Update watermark after each chunk.
- Stop when reaching latest checkpoint.

### Data backfill write path
- Fetch objects per chunk and filter by package `object_type`.
- Insert filtered objects into `sui_objects` with upsert/ignore.
- Record inserted row count per chunk (for visibility).
