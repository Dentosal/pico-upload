# Minimal private file upload service

Only run behind an authenticated proxy, e.g. nginx with http basic auth.

## Missing features:
* Autoremoval
* Index

## Running

Requires Rust nightly.

```bash
PICO_UPLOADS="/tmp/" cargo run
```
