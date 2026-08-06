# CASIROS Web Dashboard

A single-page static dashboard for the CASIROS API.

## Usage

Start the API server from the repository root:

```bash
cargo run -p casiros-api
```

Then open <http://localhost:8080/dashboard> in your browser.

The dashboard supports:

- Health checks
- Graph evaluation with live bar charts
- Monte Carlo simulation with summary statistics
- Snapshot save/load/delete/list operations

When the dashboard is served from `http://localhost:8080/dashboard`, CORS is not
required. If you serve these files from another origin, the API's permissive CORS
configuration is enabled by default for local development.
