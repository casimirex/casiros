# End-to-End Smoke Tests

These tests launch the compiled `casiros-api` and `casiros-worker` binaries and
drive them over real HTTP. They exist to cover the layer the other test suites
structurally cannot reach.

## Why they exist

Every other suite builds an Actix `App` in-process and calls handlers directly.
That is fast and precise, but it *rebuilds the application by hand* and so never
executes `main.rs`. Anything decided there is invisible:

- which storage backend was selected
- whether an environment variable was spelled in a form the config crate accepts
- whether a route was registered at all
- whether the API and the worker agree on where jobs live

Six defects shipped past a fully green test suite for exactly this reason:

| Defect | Why in-process tests missed it |
|---|---|
| `/v1/healthz` returned 401 | Tests registered routes manually, never through the `/v1` scope |
| Jobs queued forever | Tests injected a job store; `main.rs` hardcoded the in-memory one |
| Config prefix ignored | Tests set struct fields directly, never went through env parsing |
| Container bound to localhost | No test ever read `CASIROS__BIND_ADDR` |
| Dashboard returned 401 | No test requested a static asset |
| Five formulas uncallable | No test posted a series-valued port |

Each now has a test in `tests/smoke.rs` that fails if it regresses.

## Running them

The tests need PostgreSQL and the two binaries.

```bash
# 1. Database
docker compose up -d postgres

# 2. Binaries — the tests spawn these as subprocesses
cargo build -p casiros-api -p casiros-worker

# 3. Tests
cargo test -p casiros-e2e
```

Point at a different database with `CASIROS__POSTGRES__URL`.

Each test claims its own ephemeral port, so they can run concurrently. The job
pipeline test is the slow one — it waits for a real worker to claim and finish a
500-universe simulation, which can take up to 90 seconds on a cold start.

## Browser smoke test

`scripts/browser-smoke.js` covers what no Rust test can see: uncaught exceptions
in page scripts, assets that 404, charts that fail to render.

```bash
npm install puppeteer-core@23 --no-save
cargo run --release -p casiros-api &
node scripts/browser-smoke.js http://localhost:8080 your-api-key
```

It loads the dashboard, checks that styles and scripts actually applied, runs a
health check, an evaluation, and a simulation, then re-runs the evaluation —
the second render is what exposed the original chart-teardown crash.

## Adding a test

When a defect escapes to production, ask whether an in-process test *could* have
caught it. If the answer is no because the bug lives in wiring, configuration,
or the browser, it belongs here.
