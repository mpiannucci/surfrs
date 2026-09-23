# ECMWF Open Data: wave products vs GFS / GEFS wave

Investigated against the `20260921/12z` run in `gs://ecmwf-open-data`
(HTTPS: `https://storage.googleapis.com/ecmwf-open-data/...`). Values were
checked with gribberish 1.8.0 (crates.io) and eccodes 2.49 as a reference
decoder.

## Layout

```
{YYYYMMDD}/{HH}z/{model}/0p25/{stream}/{YYYYMMDDHH}0000-{step}h-{stream}-{type}.grib2
                                                                        .index
```

| model         | stream | type(s)        | what                                   |
|---------------|--------|----------------|----------------------------------------|
| `ifs`         | `wave` | `fc`           | IFS HRES wave (ECWAM), deterministic   |
| `ifs`         | `waef` | `ef`, `ep`     | IFS ENS wave: 50 perturbed members; `ep` = probabilities |
| `aifs-single` | `wave` | `fc`           | AIFS (ML) deterministic wave           |
| `aifs-ens`    | `waef` | `cf`, `pf`, `ep` | AIFS ENS wave: control + 50 members, probabilities |

The `.index` files are JSON lines with MARS keys plus `_offset`/`_length`, so
you can use them for HTTP range reads the same way we use NOAA `.idx`
files. `number` is the ensemble member number. Range reads matter a lot for
`waef`: a full step is ~530 MB (650 messages), while one field for one member
is ~0.75–1 MB.

All grids are regular 0.25° lat/lon, 1440×721. Rows go north→south (90 → -90).
Longitudes start at **180°** and wrap to 179.75° (so the grid is really
-180..179.75). Packing is CCSDS (`grid_ccsds`, 16 bit) with a bitmap for land
(~665.6k of 1.04M points are valid).

## Cycles and steps

| product            | cycles       | steps                                  |
|--------------------|--------------|----------------------------------------|
| IFS HRES `wave`    | 00z, 12z     | 0–144h every 3h, 150–360h every 6h (85 files) |
| IFS HRES `wave`    | 06z, 18z     | 0–144h every 3h (49 files)             |
| IFS ENS `waef` ef  | 00z, 12z     | same as HRES 00/12z (85 files)         |
| IFS ENS `waef` ef  | 06z, 18z     | 0–144h every 3h                        |
| IFS ENS `waef` ep  | 00z, 12z     | `240h` file = 12..240 every 12h; `360h` file = 252..360 every 12h |
| AIFS single `wave` | all 4        | 0–360h every 6h (61 files)             |
| AIFS ENS `waef`    | all 4        | 0–360h every 6h; `ep` at 240h and 360h |

For comparison, the GFS wave run for the same cycle has 0–120h hourly and
123–384h every 3h (209 files). GEFS wave has 0–240h every 3h and 246–384h every
6h (105 files per member).

## Parameters

### IFS HRES `wave` / IFS ENS `waef` (13 fields per member per step)

| shortName | paramId | GRIB2 (d,c,n) | PDT | meaning |
|-----------|---------|---------------|-----|---------|
| `swh`   | 140229 | 10,0,3  | 0 / 1 | Significant height of combined wind waves and swell |
| `mwd`   | 140230 | 10,0,14 | 0 / 1 | Mean wave direction (total sea), °true |
| `mwp`   | 140232 | 10,0,15 | 0 / 1 | Mean wave period (ECMWF Tm-1,0) |
| `mp2`   | 140221 | 10,0,28 | 0 / 1 | Mean zero-crossing period (Tm02) |
| `pp1d`  | 140231 | 10,0,34 | 0 / 1 | Peak wave period |
| `h1012` | 140114 | 10,0,3  | 104   | Hs of waves with periods 10–12 s |
| `h1214` | 140115 | 10,0,3  | 104   | Hs, 12–14 s |
| `h1417` | 140116 | 10,0,3  | 104   | Hs, 14–17 s |
| `h1721` | 140117 | 10,0,3  | 104   | Hs, 17–21 s |
| `h2125` | 140118 | 10,0,3  | 104   | Hs, 21–25 s |
| `h2530` | 140119 | 10,0,3  | 104   | Hs, 25–30 s |
| `wmb`   | 140219 | 10,4,7  | 0 / 1 | Model bathymetry (m, capped at 999) |
| `cdww`  | 140233 | 10,0,16 | 0 / 1 | Coefficient of drag with waves |

HRES uses PDT 0 for the bulk fields. ENS members use PDT 1. The period-band
heights use PDT **4.104** in both HRES and ENS (HRES is encoded as member 0),
and AIFS uses **4.103**. All band fields share the same parameter (10,0,3 = Hs).
Only the PDT tells them apart: `typeOfWavePeriodInterval=7` plus the
lower/upper period limits. Any code that keys on (discipline, category,
number) alone will think it has seven `swh` fields.

IFS ENS `ep` (PDT 5, probability): `swhg2`, `swhg4`, `swhg6`, `swhg8` (% chance that
Hs ≥ 2/4/6/8 m).

### AIFS

- `wave` / `waef`: 10 fields. Same as IFS minus `pp1d` and `mp2`. `wmb` is
  only published at step 0 (IFS has it at every step).
- `ep`: `swhg2/4/6/8` plus `mwpg8/10/12/15` (mean period ≥ N s). It also has
  time-window probabilities (`120-168`, `120-240`, `168-240`).

### Not in ECMWF open data

- No wind-sea / swell partitions (no height, period or direction for wind sea
  or swell 1/2/3).
- **No direction other than `mwd`**. The period bands carry height only.
- No wave-stream wind. `10u`/`10v` are in the separate `oper`/`enfo` streams
  (same 0.25° grid).
- No 2D spectra, point spectra or station bulletins.
- No precomputed ensemble mean or spread. Compute them from the members.
- No IFS ENS control (`cf`) for waves. `numberOfForecastsInEnsemble=51`, but
  only `pf` 1–50 are published. AIFS ENS does publish `cf`.

## Side by side

|                      | GFS wave (GFSv16 WW3) | GEFS wave | IFS HRES wave | IFS ENS wave | AIFS / AIFS ENS wave |
|----------------------|-----------------------|-----------|---------------|--------------|----------------------|
| Grid                 | global 0.25 & 0.16, regional 0.16 (atlocn, wcoast, epacif…) | global 0.25 | global 0.25 | global 0.25 | global 0.25 |
| Members              | 1 | c00 + p01–p30, plus mean and spread | 1 | 50 (no control) | 1 / cf + 50 |
| Max lead             | 384h | 384h | 360h (00/12z), 144h (06/18z) | same as HRES | 360h, all cycles |
| Time step            | 1h to 120h, then 3h | 3h to 240h, then 6h | 3h to 144h, then 6h | same as HRES | 6h |
| Total Hs / Tp / Dp   | HTSGW, PERPW, DIRPW | yes | swh, pp1d, **mwd (mean, not peak dir)** | same as HRES | swh, mwd (no Tp) |
| Mean periods         | — (member files have MWSPER) | MWSPER, IMWF | mwp (Tm-1,0), mp2 (Tm02) | same as HRES | mwp |
| Wind-sea partition   | WVHGT/WVPER/WVDIR | yes | **no** | no | no |
| Swell partitions     | 3× SWELL/SWPER/SWDIR | 3× per member; mean has 2 without direction | **no** | no | no |
| Period-band Hs       | no | no | **yes, 6 bands from 10 to 30 s** | yes | yes |
| Wind in wave file    | WIND/WDIR/UGRD/VGRD | yes | no (in `oper`) | no (in `enfo`) | no |
| Point spectra/bull.  | `station/` bull, cbull, spec | GEFS bulletins | no | no | no |
| Probabilities        | — | — | — | Hs ≥ 2/4/6/8 m | Hs and Tm thresholds |

## What this means for surfrs

- **The period bands are the most useful surf-specific field ECMWF gives us.**
  Each band gives the long-period swell energy with no need to partition
  anything. The Hs below 10 s is the residual
  `sqrt(swh² − Σ hband²)`. Example at 40.75N, 71.5W, step 24:
  swh = 1.76 m, bands 10–30 s ≈ 0.23 m combined. So ~98% of the energy is
  below 10 s, which matches a 6.3 s peak period.
- **Direction is the big gap.** We only have one energy-weighted mean direction
  for the whole sea state. In a mixed sea (for example an E wind swell plus an S
  groundswell) we can't give each component a direction the way
  `GFSWaveGribPointDataRecord` does with SWDIR 1–3. A pragmatic workaround is to
  use `mwd` from a band-dominated state, or take direction from GFS.
- **The ensemble is richer than GEFS on members (50 vs 31) and has the bands,**
  but we'd have to compute mean and spread ourselves. We also need range reads
  from the index; the files are far too big to download whole.
- The ECMWF `.index` format is JSON lines, not the NOAA `.idx` text.
  gribberish already parses it: `gribberish::index::parse_ecmwf_index` (or
  `parse_index`, which auto-detects NOAA vs ECMWF) returns `IndexEntry`s with
  `offset`/`length` for range reads and all MARS keys (`param`, `number`,
  `step`, …) in `keys`.

## gribberish compatibility

**Requires gribberish ≥ 1.8.0.** Older versions panic or return wrong values
in the CCSDS decoder (≤ 0.21), or can't read the period-band metadata (1.7
lacks PDT 4.104). surfrs builds and passes all tests on 1.8.0 with no code
changes.

With 1.8.0, all 37 sample messages (IFS HRES, IFS ENS, AIFS, IFS ENS `ep`)
decode identically to eccodes, and the metadata is complete:

| product | PDT | how to identify it |
|---------|-----|--------------------|
| bulk fields | 0 / 1 | abbrev (`HTSGW`, `WWSDIR`, `PWPER`, `MWSPER`, `MZWPER`, `WMB`, `CDWW`); `perturbation_number()` for ENS members |
| period bands (AIFS) | 4.103 | `wave_period_range()` → `(Some(10.0), Some(12.0))`, … |
| period bands (IFS HRES/ENS) | 4.104 | `wave_period_range()`, plus `perturbation_number()` (0 for HRES, 1–50 for ENS) |
| probabilities | 4.5 | `probability_type()` = `AboveLowerLimit`, `probability_lower_limit()`, unit `%` |

The period bands and probabilities all use abbrev `HTSGW`, so don't key on
the abbrev alone. `key()` disambiguates them (e.g.
`HTSGW:…:ens1:per14-17s:ensemble forecast`, `…:probt3_4.00000:…`).
`bbox()` reports longitudes as 0 → 359.75; point lookups match eccodes.
