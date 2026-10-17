# 002 — Sources

**Status:** Implemented (t2vi/spoolbook-filament-sync#2)
**Code:** `src/main.rs`, `src/cloak_browser_client.rs`, `src/sources/*.rs`

## Summary

The daily scrape runs every store's source and publishes what they found. A store that can't
be scraped must never silently vanish from the catalog: every failure is named, on the run.

## Functional requirements

- **FR-001** — Each source runs on its own: one failing never stops the others, and the catalog
  is written from the sources that succeeded. *Verify: unit*
- **FR-002** — A source that errors, or yields no entries, is skipped and reported: a
  `Skipping <brand>: <reason>` line, and a GitHub Actions warning annotation
  (`::warning title=Brand skipped::<brand>: <reason>`) so it shows on the run's summary. The run
  ends with `Scraped <n> of <m> sources`. *Verify: unit*
- **FR-003** — A page fetched through the stealth browser that is a bot challenge (DataDome's
  `captcha-delivery.com`, Cloudflare's challenge) is an error — "blocked by a bot challenge" —
  not an empty page. *Verify: unit*
- **FR-004** — The daily scrape runs on GitHub's runners with these sources: SpoolmanDB
  (spec 003) and our store scrapers for Bambu Lab, eSUN, Hatchbox, Prusament and Colorfabb.
  *Verify: unit*

## Notes

- Why the store scrapers were retired (2026-10-17): from GitHub's shared IPs every Shopify store
  (Elegoo, SUNLU, Polymaker, Overture, Creality, Protopasta, Fillamentum) answered
  `429 Too Many Requests` and Jaycar (Slic3D) served a DataDome challenge, while from a
  residential IP all of them answered normally. A self-hosted runner was considered; SpoolmanDB
  (spec 003) covers those brands without one.
