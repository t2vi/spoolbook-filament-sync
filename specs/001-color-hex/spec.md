# 001 — Color hex

**Status:** Implemented (t2vi/spoolbook-filament-sync#3)
**Code:** `src/color_hex_resolver.rs`, `src/main.rs` (`dedupe_and_resolve_hex`)

## Summary

Every catalog entry carries a best-effort `Hex` for its color name, so spoolbook can draw a
swatch instead of a flat gray placeholder. A filament with two or more colors gets one hex per
color, so its swatch shows each of them (spoolbook splits a hex list into diagonal bands).

## Output

- `Hex` is `null` (no idea), one `#RRGGBB` (uppercase), or two or more of them joined by
  `", "`, in the order the colors appear in the name. spoolbook reads this as the color's hex
  (spoolbook spec 007/FR-014, FR-023).

## Functional requirements

### One color

- **FR-001** — A name is looked up in the CSS named colors, then a table of common filament
  color words (kraft, rose gold, bronze, …), ignoring case and anything but letters.
  *Verify: unit*
- **FR-002** — If that fails, finish/product words (matte, silk, translucent, glow, …) are
  dropped and the rest is looked up again; then each remaining word from the last to the first.
  No match → `null`. *Verify: unit*

### Several colors

- **FR-010** — A name that names its colors — separated by `+`, `&`, `/`, ` and `, ` to `, or
  a `-` between two words — gets the hex of each part (each resolved as FR-001–002), in order,
  when at least two parts resolve; parts that don't resolve are left out (e.g. a misspelt
  "Bule+Orange+Green" → orange, green). `Black+Gold` → `#000000, #FFD700`; `Gradient Celestial
  Blue-Purple` → blue, purple; `Black to the Fuchsia` → black, fuchsia. Fewer than two parts
  resolve → treated as one color (FR-001–002). *Verify: unit*
- **FR-011** — A name containing the word "rainbow" (any case, e.g. Rainbow, Rainbow-03, Dark
  Rainbow, Gradient Luminous Rainbow, Candy Rainbow) gets the six rainbow bands `#E40303,
  #FF8C00, #FFED00, #008026, #24408E, #732982`. *Verify: unit*
- **FR-012** — A multi-color filament whose name doesn't name its colors (e.g. "Amethyst Aria",
  "Citrus Sunrise Orange") is treated as one color (FR-001–002) — there's nothing in the name
  to split. *Verify: unit*
- **FR-013** — An entry whose source already gave it a hex (SpoolmanDB, spec 003) keeps it.
  Every other entry gets its color name's hex, resolved once per name, so entries sharing a
  name share it. *Verify: unit*

## Notes

- A two-word color joined by a dash whose halves are both colors (e.g. "Navy-Blue") reads as
  two colors; both bands are blue-ish, so the swatch still looks right.
- Store titles that leak sizes or SKUs into the color ("… 1.75 / 2000") don't produce two
  resolvable parts, so they stay single — cleaning those is a separate fix (#2).
