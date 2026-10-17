# 003 — SpoolmanDB

**Status:** Implemented (t2vi/spoolbook-filament-sync#2)
**Code:** `src/sources/spoolmandb.rs`

## Summary

Most brands come from [SpoolmanDB](https://github.com/Donkie/SpoolmanDB) (MIT), the community
filament database Spoolman uses: one static `filaments.json` with a real hex — or a hex list
for a multi-color filament — per filament. It's a single file on GitHub Pages, so unlike the
stores it never rate-limits or challenges the daily run. Our own store scrapers stay only for
the brands they reach and describe better (product lines, more colors): Bambu Lab, eSUN,
Hatchbox, Prusament and Colorfabb.

## Functional requirements

- **FR-001** — The source reads `https://donkie.github.io/SpoolmanDB/filaments.json`. Every
  filament whose brand isn't one of ours (Bambu Lab, eSUN, Hatchbox, Prusament, Colorfabb —
  compared ignoring case, spaces and punctuation) becomes an entry: Brand = `manufacturer`,
  Material = `material`, Variant = none, Color = `name`. *Verify: unit*
- **FR-002** — A brand we published before under another spelling keeps ours (compared the same
  way): SpoolmanDB's `ELEGOO` → `Elegoo`, `Sunlu` → `SUNLU`. Every other brand keeps SpoolmanDB's
  spelling. *Verify: unit*
- **FR-003** — Hex: `color_hexes` (two or more) → `#RRGGBB, #RRGGBB, …`; otherwise `color_hex`
  → `#RRGGBB`; uppercase, `#` added. Neither → none (the color name is resolved instead,
  001/FR-013). *Verify: unit*
- **FR-004** — SpoolmanDB lists a filament once per spool weight and diameter; those collapse
  to one entry (brand, material, variant and color all equal). *Verify: unit*
- **FR-005** — The store scrapers for Elegoo, SUNLU, Polymaker, Overture, Creality, Protopasta,
  Fillamentum and Slic3D are retired; those brands come from SpoolmanDB (Slic3D isn't in it and
  drops out of the catalog). *Verify: unit*

## Notes

- 2026-10-17: SpoolmanDB had 8,120 rows (4,707 unique filaments) from 67 brands, 104 of them
  multi-color; it covers every retired brand except Slic3D.
- SpoolmanDB folds a product line into the name (`PolyLite™ ABS Black`) or leaves it out
  (Bambu's PLA Basic and PLA Matte are both `PLA`) — the reason our richer scrapers win for the
  brands they reach.
- Rows spoolbook imported from a retired scraper stay in its database (the app never prunes the
  catalog); their SpoolmanDB counterparts arrive as new rows under the same brand.
