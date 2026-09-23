# Spec: ECMWF wave products and model refactor

Status: all phases implemented (PR #6, branch `ecmwf-wave-model`), 2026-09-23.
Background: `doc/ecmwf-open-data-waves.md`.

## Goals

1. Rename and reshape `NOAAModel` into `GriddedModel`. It is no longer NOAA-only.
2. Add one correct grid-to-point extractor that the whole crate uses. The
   current one is broken (see "Why the point code is replaced" below).
3. Treat ECMWF open-data waves as their own product. Only expose what the data
   actually contains: totals, period-band energy, and a single mean direction.
   **Do not build swell partitions or per-component directions from it.**
4. Build ensemble statistics from the IFS ENS / AIFS ENS members.

Also done on the same branch: 10 m wind from the `oper`/`enfo` streams,
sampling on projected grids (checked on HRRR), a `Location::distance` fix, and
live examples including a comparison against NDBC buoys.

## Why the point code is replaced

Findings at NDBC 44097 (see [Validation](#validation)):

- `query_location_data` flips the longitude with `abs()`. On global grids it reads
  the wrong hemisphere: −71.13° reads 71.25°E, which is land and returns NaN.
- `interp_location_data` always panics. It indexes a 1-D latitude axis with
  `row * nx`. It also has a typo in the lng/lat check and divides the step by
  `n` instead of `n-1`.
- `query_location_tolerance` is the only one in use. It takes an unweighted box
  mean, so it depends on where the box falls against the grid. It was 4–7% off
  the station output. It also averages directions arithmetically, so 350° and
  10° give 180°.
- All three decode the whole field again on every call: ~8 ms per call for
  ECMWF CCSDS and 40–110 ms for GFS JPEG2000.

WW3 builds its station output (`w3iopomd.F90`, `W3IOPE`) with bilinear weights on
the computational grid. It zeroes the weights on land cells, renormalises, and
applies them to the **spectrum**. For total Hs this is `sqrt(Σ wᵢ·Hsᵢ²)`. On GFS
`global.0p16` (the native `gnh_10m` grid) this reproduces the bulletin `Hst` to
within 0.02 m over 0–144 h.

## 1. Model trait refactor

### Rename

| Current | New | Notes |
|---|---|---|
| `model/noaa_model.rs` | `model/gridded_model.rs` | |
| `trait NOAAModel` | `trait GriddedModel` | covers GFS, GEFS, NWPS and ECMWF |
| `enum ModelDataSource` | `enum NOAADataSource` | Still NOAA mirrors only. `buoy_station.rs` uses it too. Clean break: no deprecated aliases. |
| `ModelTimeOutputResolution` | unchanged | Kept as a helper for NOAA models. Add a `SixHourly` variant. |

### Trait shape

```rust
pub trait GriddedModel {
    /// The mirrors this model family is published on (NOAADataSource, ECMWFDataSource).
    type Source;

    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    /// Most recent run expected to be fully published at `date`.
    fn closest_model_run_date(&self, date: &DateTime<Utc>) -> DateTime<Utc>;

    /// The output hours to pass to `create_url` for the run starting at `run`.
    /// This depends on the run: IFS 06/18z stops at 144h. NWPS returns `[0]`
    /// because one file holds the whole run.
    fn forecast_hours(&self, run: &DateTime<Utc>) -> Vec<usize>;

    fn url_root(&self, source: &Self::Source) -> &'static str;
    fn create_url(&self, source: &Self::Source, hour: usize, run: Option<DateTime<Utc>>) -> String;

    /// Sidecar index URL. NOAA default: `{url}.idx`. ECMWF: `.grib2` replaced by `.index`.
    fn create_index_url(&self, source: &Self::Source, hour: usize, run: Option<DateTime<Utc>>) -> String;
}
```

Removed from the trait: `query_location_tolerance`, `query_location_data`,
`interp_location_data`, `contour_data`, `time_resolution`, `hour_for_index` and
`index_for_hour`. The first three are replaced by the sampler (section 2).
`contour_data` does not use the model, so it becomes the free function
`tools::contour::contour_message(message, …)`. The time helpers stay on
`ModelTimeOutputResolution`. GFS and GEFS keep `time_resolution()` as an
inherent method.

This is a breaking change. Bump the crate to `0.2.0`.

## 2. Grid point sampler (`tools/grid_sampler.rs`)

```rust
pub enum SampleMethod {
    Nearest,
    /// Bilinear on the value. Use for periods and other scalars.
    Bilinear,
    /// Bilinear on value², then sqrt. Use for wave heights (energy). Matches WW3
    /// station output.
    BilinearEnergy,
    /// Bilinear on unit vectors (sin/cos). Use for directions.
    BilinearAngular,
}

pub enum SampleSource {
    /// Interpolated from `sea_cells` (1–4) valid corners, with weights renormalised.
    Interpolated { sea_cells: u8 },
    /// `Nearest` method: the closest cell was valid.
    NearestCell,
    /// Every candidate cell is land. Nearest valid cell within `max_fallback_km`.
    NearestSeaCell { distance_km: f64 },
    Missing,
}

pub struct PointSample { pub value: Option<f64>, pub source: SampleSource }

pub struct GridSampler { /* geometry (regular or projected), rows, columns, data */ }

impl GridSampler {
    /// Decodes once. Regular lat/lng grids and grids regular in projected metres.
    pub fn from_message(message: &Message) -> Result<Self, GribberishError>;
    /// Geometry plus row-major data, for synthetic grids and tests.
    pub fn from_regular_grid(lat_start, lat_step, lat_count, lng_start, lng_step, lng_count, data) -> Result<Self, GribberishError>;
    pub fn sample(&self, location: &Location, method: SampleMethod) -> PointSample;
    pub fn sample_many(&self, locations: &[Location], method: SampleMethod) -> Vec<PointSample>;
    pub fn with_max_fallback_km(self, km: f64) -> Self; // default 30 km
}
```

`from_message` also accepts grids that are regular in projected metres (Lambert
conformal, polar stereographic, Mercator). The point is projected with the
message's own projection and interpolated in grid-index space. gribberish keeps
its projection types private, so the sampler holds the projector inside
closures (`project_xy(lng, lat)` returns `(y, x)`; `project_latlng(y, x)`
returns `(lat, lng)`). Exporting `LatLngProjection` from gribberish would make
this simpler.

Checked on HRRR (Lambert conformal, 3 km): nearest values match eccodes at five
points across CONUS, in both longitude conventions, and points outside the grid
(Hawaii, London, Sydney) are `Missing`. NWPS could not be checked: its NOMADS
directory currently lists nothing.

Rules:
- Column index is `((lon − lon0).rem_euclid(360)) / dlon`. It wraps on global
  grids and returns `Missing` off the edge of regional grids. It works for grids
  that start at 0°, at 180° (ECMWF), or at a regional offset (`atlocn` at 260°).
- Step sizes come from the grid increments, not `(end − start) / n`.
- NaN (bitmap/land) corners get zero weight and the rest are renormalised, as
  in WW3.
- Energy weighting keeps heights consistent with each other. If Hsᵢ² ≥ Σ bandᵢ²
  holds at every corner, it still holds after interpolation. Linear Hs
  interpolation does not guarantee this.

### Migrating GFS

`GFSWaveGribPointDataRecord::from_messages(model, messages, location, tolerance)`
becomes `from_messages(messages, location)`:
- Build one sampler per message.
- Heights (`HTSGW`, `WVHGT`, `SWELL_n`) use `BilinearEnergy`.
- Periods use `Bilinear`.
- Directions (`DIRPW`, `WVDIR`, `SWDIR_n`, `WDIR`) use `BilinearAngular`.
- Wind speed uses `Bilinear`.

`examples/gen_surf_forecast_gfs.rs` is updated to match.

## 3. ECMWF model and sources (`model/ecmwf_wave.rs`)

```rust
pub enum ECMWFDataSource { ECMWF, GCP, AWS }
// ECMWFWaveModel reads the wave streams (wave/waef); ECMWFWindModel reads the
// matching atmosphere streams (oper/enfo) for 10 m wind. Both share
// ECMWFProduct, so steps, delays and URLs stay in step.
// ECMWF: https://data.ecmwf.int/forecasts
// GCP:   https://storage.googleapis.com/ecmwf-open-data
// AWS:   https://ecmwf-forecasts.s3.eu-central-1.amazonaws.com
// (Azure mirror not yet verified; add when confirmed.)

pub enum ECMWFProduct {   // renamed from ECMWFWaveProduct
    IfsHres,            // ifs/0p25/wave,         type fc
    IfsEnsMembers,      // ifs/0p25/waef,         type ef (members 1–50, no control)
    IfsEnsProbability,  // ifs/0p25/waef,         type ep
    AifsSingle,         // aifs-single/0p25/wave, type fc
    AifsEnsControl,     // aifs-ens/0p25/waef,    type cf
    AifsEnsMembers,     // aifs-ens/0p25/waef,    type pf (members 1–50)
    AifsEnsProbability, // aifs-ens/0p25/waef,    type ep
}

pub struct ECMWFWaveModel { pub product: ECMWFProduct, /* id, name, description */ }
pub struct ECMWFWindModel { pub product: ECMWFProduct, /* ... */ }
impl ECMWFWindModel {
    // ifs_hres(), ifs_ens_members(), aifs_single(), aifs_ens_control(), aifs_ens_members()
    /// The wind model for the same run and members; None for probability products.
    pub fn for_wave_model(wave: &ECMWFWaveModel) -> Option<Self>;
}
// Constructors: ifs_hres(), ifs_ens_members(), ifs_ens_probability(), aifs_single(),
// aifs_ens_control(), aifs_ens_members(), aifs_ens_probability()
```

URL: `{root}/{YYYYMMDD}/{HH}z/{model}/0p25/{stream}/{YYYYMMDDHH}0000-{step}h-{stream}-{type}.grib2`

`forecast_hours(run)`:

| product | 00z / 12z | 06z / 18z |
|---|---|---|
| IFS HRES, IFS ENS members | 0–144 every 3h, then 150–360 every 6h | 0–144 every 3h |
| IFS ENS probabilities | 240, 360 | none |
| AIFS (all) | 0–360 every 6h | 0–360 every 6h |
| AIFS ENS probabilities | 240, 360 | 240, 360 |

`closest_model_run_date`: subtract a per-product publication delay, then floor
to the run interval (12h for IFS ENS probabilities, which only come from 00z/12z;
6h otherwise). The delays come from when the last file of each 2026-09-22 run
was written, plus a margin:

| product | last file after run start | delay used |
|---|---|---|
| IFS HRES | 6.5–7.6 h | 8 h |
| IFS ENS members / probabilities | 7.7–9.0 h | 10 h |
| AIFS single | 5.5 h | 6 h |
| AIFS ENS | 6.9–7.1 h | 8 h |

The atmosphere streams publish the same steps a minute or two before the wave
streams, so `ECMWFWindModel` uses the same delays.

### Selecting fields from the index

A full `waef` step is ~530 MB, so range reads are required.

```rust
// model/byte_range.rs, usable for NOAA .idx too
pub struct ByteRange { pub offset: u64, pub length: u64 }
impl ByteRange {
    pub fn from_index_entry(entry: &IndexEntry) -> Option<Self>;
    pub fn end(&self) -> u64;
    pub fn http_range_header(&self) -> String; // "bytes=a-b"
}
/// Merge overlapping ranges or ranges at most `max_gap` apart into fewer requests.
pub fn coalesce_ranges(ranges: Vec<ByteRange>, max_gap: u64) -> Vec<ByteRange>;

// model/ecmwf_wave.rs
pub enum MemberSelection { All, Only(Vec<u8>) }

/// Filters entries from `gribberish::index::parse_ecmwf_index`.
pub struct ECMWFIndexQuery {
    pub fields: Vec<ECMWFField>,   // empty = every field
    pub members: MemberSelection,      // entries without `number` always match
    pub steps: Option<Vec<String>>,    // e.g. "24", "120-168"; None = every step
}
impl ECMWFIndexQuery {
    pub fn matches(&self, entry: &IndexEntry) -> bool;
    pub fn select(&self, entries: &[IndexEntry]) -> Vec<ByteRange>;
    pub fn select_coalesced(&self, entries: &[IndexEntry]) -> Vec<ByteRange>;
}
```

Only `ef`/`pf` index entries carry `number`.

All 50 members are in one file per step (IFS `ef`: 650 messages, ~537 MB; AIFS
`pf`: 500), but the messages are in no particular order: neither a field's 50
members nor a member's fields are adjacent. One file means one index per step,
but every member's field is still its own request of ~0.85 MB (`swh` for all 50
members is 42.7 MB per step), and coalescing does not help. Probability (`ep`) files hold every
step of the run (12–240h or 252–360h), and AIFS adds time windows such as
`120-168`, so `steps` is needed to fetch one.

surfrs keeps building URLs and ranges only. Fetching stays with the caller, as
it does for GFS.

## 4. Field classification (`data/ecmwf_field.rs`)

`ECMWFWaveField` was renamed `ECMWFField` (and `ECMWFWaveMessageInfo`
`ECMWFMessageInfo`) when 10 m wind was added: `WindU` (`10u`, 0,2,2) and
`WindV` (`10v`, 0,2,3), matched only at 10 m above ground so 100 m and
pressure-level winds are skipped.

```rust
pub enum ECMWFField {
    SignificantHeight,               // swh   10,0,3   PDT 0/1
    MeanDirection,                   // mwd   10,0,14
    EnergyPeriod,                    // mwp   10,0,15  Tm-1,0
    ZeroCrossingPeriod,              // mp2   10,0,28  Tm02  (IFS only)
    PeakPeriod,                      // pp1d  10,0,34        (IFS only)
    PeriodBandHeight { min_period: u8, max_period: u8 }, // h1012…h2530  10,0,3  PDT 4.103/4.104
    ModelBathymetry,                 // wmb   10,4,7
    DragCoefficient,                 // cdww  10,0,16
    HeightExceedance { threshold: u8 },  // swhg2/4/6/8     10,0,3      PDT 4.5 / 4.9
    PeriodExceedance { threshold: u8 },  // mwpg8/10/12/15  255,131,79  PDT 4.5 / 4.9 (AIFS only)
}

impl ECMWFField {
    pub fn from_message(m: &Message) -> Option<Self>;
    pub fn period_bands() -> Vec<Self>;
    pub fn mars_param(&self) -> String;         // for index selection
    pub fn sample_method(&self) -> SampleMethod;
    pub fn unit(&self) -> Unit;
}

pub enum ForecastMember { Deterministic, Control, Perturbed(u8) }

pub struct ECMWFMessageInfo {
    pub field: ECMWFField,
    pub member: ForecastMember,
    pub reference_date: DateTime<Utc>,
    pub valid_date: DateTime<Utc>,              // window start for PDT 4.9
    pub valid_end_date: Option<DateTime<Utc>>,  // window end for PDT 4.9
}
impl ECMWFMessageInfo { pub fn from_message(m: &Message) -> Option<Self>; }
```

Thresholds and band limits are whole numbers, so the enum is `Eq + Hash` and can
key a map.

Classification uses `(discipline_value, category_value, parameter_value)` plus
`wave_period_range()`, `perturbation_number()`, `probability_type()` and
`probability_lower_limit()`. **Never use the abbreviation alone**: all six bands
and all Hs probabilities decode as `HTSGW`.

Found when decoding the fixtures:
- IFS HRES encodes its bands with PDT 4.104 as member 0 of an ensemble of
  **0**. The AIFS ENS control is member 0 of 51. So `ForecastMember` is
  `Deterministic` when the ensemble size is missing or 0, `Control` for member 0
  of a real ensemble, and `Perturbed(n)` otherwise.
- AIFS `mwpg*` probabilities use ECMWF local parameter (255, 131, 79) with no
  abbreviation, not (10, 0, 15).
- AIFS time-window probabilities use PDT 4.9, with `forecast_end_date()` set.

## 5. Point record (`data/ecmwf_wave_point_data_record.rs`)

```rust
pub struct PeriodBandHeight {
    pub min_period: u8,                // s
    pub max_period: u8,                // s
    pub height: DimensionalData<f64>,  // Hs of energy in [min, max)
}

pub struct ECMWFWavePointDataRecord {
    pub reference_date: DateTime<Utc>,
    pub date: DateTime<Utc>,                     // valid time
    pub member: ForecastMember,
    pub significant_wave_height: DimensionalData<f64>,
    pub mean_direction: DimensionalData<Direction>,   // whole sea state
    pub energy_period: DimensionalData<f64>,          // Tm-1,0
    pub zero_crossing_period: DimensionalData<f64>,   // Tm02, None for AIFS
    pub peak_period: DimensionalData<f64>,            // Tp,   None for AIFS
    pub period_bands: Vec<PeriodBandHeight>,          // bands present, ascending period
    pub wind_speed: DimensionalData<f64>,             // 10 m, when wind messages are passed
    pub wind_direction: DimensionalData<Direction>,   // 10 m, coming from
    pub sample_source: SampleSource,                  // how swh was sampled at the location
}

impl ECMWFWavePointDataRecord {
    /// One record per (member, valid time) in `messages`, sorted by member then time.
    /// Errors if a group has no swh. Probability, bathymetry and drag are ignored.
    pub fn from_messages(messages: &[Message], location: &Location) -> Result<Vec<Self>, DataRecordParsingError>;
    /// Same, decoding each message once for all locations. One Vec per location.
    pub fn from_messages_many(messages: &[Message], locations: &[Location]) -> Result<Vec<Vec<Self>>, DataRecordParsingError>;
    /// sqrt(Σ band²): height from energy at periods of 10–30 s. None unless all 6 bands are present.
    pub fn long_period_height(&self) -> DimensionalData<f64>;
    /// sqrt(max(0, Hs² − Σ band²)): height from energy outside 10–30 s (in practice < 10 s).
    /// None unless all 6 bands are present.
    pub fn short_period_height(&self) -> DimensionalData<f64>;
    /// Deep-water energy flux ρg²/(64π)·Hs²·Tm-1,0 in kW/m (ρ = 1029, g = 9.81, as in
    /// `tools::waves::wave_energy`). Uses Hs in metres whatever the record's units.
    pub fn energy_flux(&self) -> DimensionalData<f64>;
}
```

Also implements `UnitConvertible`. It does **not** implement `SwellProvider`.
Adds `Unit::KiloWattsPerMeter`.

Wave and wind messages for the same member and time can be passed together and
land on one record. u and v are sampled with `Bilinear` and combined into speed
and a meteorological (coming from) direction, as WW3 does for its station wind.

A location with no sea cell within the fallback distance still gets a record,
with every value `None` and `sample_source: Missing`.

Checked on the IFS HRES fixture (2026-09-23 00z, 24h):
- At 44097: Hs 3.61 m from 81°, Tp 8.8 s ≥ Tm-1,0 7.4 s ≥ Tm02 5.6 s. The
  10–30 s bands give 1.44 m, and energy outside them gives 3.31 m. Energy flux
  is 47.5 kW/m.
- Over a 1° lattice (36,859 ocean points), band energy exceeds the total at 43
  points, by at most 1.2e-4 m². That is packing noise, and
  `short_period_height` clamps it to 0.

## 6. Ensemble (`data/ecmwf_wave_ensemble_point_data_record.rs`)

```rust
pub struct EnsembleStatistics {
    pub member_count: usize,        // members with a value for this quantity
    pub mean: f64,
    pub spread: f64,                // population standard deviation
    pub min: f64, pub max: f64,
    pub p10: f64, pub p50: f64, pub p90: f64,   // linear interpolation between members
    pub unit: Unit,
}
pub struct AngularEnsembleStatistics {
    pub member_count: usize,
    pub mean: f64,                  // direction of the mean unit vector
    pub spread: f64,                // circular standard deviation sqrt(-2 ln R), degrees
    pub resultant_length: f64,      // R, 0..1
}
pub struct PeriodBandStatistics { pub min_period: u8, pub max_period: u8, pub height: EnsembleStatistics }

pub struct ECMWFWaveEnsemblePointDataRecord {
    pub reference_date: DateTime<Utc>,
    pub date: DateTime<Utc>,
    pub member_count: usize,
    pub significant_wave_height: Option<EnsembleStatistics>,
    pub mean_direction: Option<AngularEnsembleStatistics>,
    pub energy_period: Option<EnsembleStatistics>,
    pub zero_crossing_period: Option<EnsembleStatistics>,
    pub peak_period: Option<EnsembleStatistics>,
    pub period_bands: Vec<PeriodBandStatistics>,
    pub long_period_height: Option<EnsembleStatistics>,
    pub short_period_height: Option<EnsembleStatistics>,
    pub energy_flux: Option<EnsembleStatistics>,
    pub wind_speed: Option<EnsembleStatistics>,
    pub wind_direction: Option<AngularEnsembleStatistics>,
}

impl ECMWFWaveEnsemblePointDataRecord {
    /// Members must share a valid time and be Control or Perturbed.
    pub fn from_members(members: &[ECMWFWavePointDataRecord]) -> Result<Self, DataRecordParsingError>;
    /// Groups by valid time (in order) and skips deterministic records.
    pub fn from_member_records(records: &[ECMWFWavePointDataRecord]) -> Result<Vec<Self>, DataRecordParsingError>;
}
/// Fraction of members (0-1) for which `predicate` holds, e.g. 14–17 s band > 0.5 m.
pub fn exceedance_probability(members: &[ECMWFWavePointDataRecord], predicate: impl Fn(&ECMWFWavePointDataRecord) -> bool) -> Option<f64>;
```

Choices:
- Each quantity uses the members that have a value, and records how many.
- Mean Hs is the plain average of member Hs, as GEFS publishes it.
- Heights are converted to the first member's unit before the statistics.

The published `ep` probabilities (`swhg*`, `mwpg*`) are read as they are, with
`GridSampler` using `Bilinear`.

Live check at 44097 (IFS ENS 2026-09-23 06z, all 50 members): Hs 3.60 ± 0.17 m
at 24h and 3.25 ± 0.77 m at 72h, with direction agreement going from ±2° to
±17°.

## 7. Wind

Done in phase 5: see `ECMWFWindModel` (section 3), `ECMWFField::WindU/WindV`
(section 4) and the wind fields on the point record (section 5).

## Validation

- **Regression test for the sampler:** GFS `global.0p16` HTSGW at 44097 using
  `BilinearEnergy` must be within 0.02 m of the bulletin `Hst`, for one step
  stored as a fixture. Measured over 0–144h: bias −0.007 m, max error 0.018 m.
- **Longitude wrapping:** the same location sampled on grids starting at 0°, 180°
  and 260° must give identical results. Checked against eccodes nearest-point.
- **Coastal:** a station whose four corners are all land returns `NearestSeaCell`
  with a distance, never a silent NaN or a panic.
- **ECMWF consistency:** Hs² ≥ Σ band² (to packing precision) at every station
  in a full-grid sweep.
- **Classification:** each fixture message maps to exactly one `ECMWFField`.
  The 6 bands, and `swh` vs `swhg*`, are told apart.

- **Projected grids:** HRRR nearest values match eccodes (see section 2).

### Against NDBC buoys

`examples/validate_against_ndbc.rs` scores IFS HRES (AWS mirror) and GFS-Wave
`global.0p16` against NDBC realtime observations, both sampled with
`GridSampler`. Run 2026-09-20 00z, 0–72 h every 6 h, 15 buoys (bias / RMSE,
forecast − observed):

| | n | bias | RMSE |
|---|---|---|---|
| IFS Hs (m) | 195 | +0.08 | 0.30 |
| GFS Hs (m) | 195 | −0.10 | 0.25 |
| IFS Tp vs DPD (s) | 76 | −0.23 | 1.63 |
| GFS Tp vs DPD (s) | 76 | +1.44 | 3.72 |
| IFS Tm02 vs APD (s) | 195 | −0.72 | 0.87 |

- IFS Hs at 51201 (Waimea, Oahu north shore) is +0.80 m. The two southern
  corners are land on the 0.25° grid, so the value comes from the exposed cells
  to the north (`Interpolated { sea_cells: 2 }`). GFS has a station there on its
  finer grid. Without it, IFS Hs is within about ±0.25 m everywhere.
- Peak period is unstable in mixed seas. Off California, GFS follows the long
  southern swell while the buoy's DPD follows the local wind sea (+6 to +9 s).
  DPD is often missing at these hours, so n is small.
- IFS Tm02 is about 0.7 s shorter than buoy APD. A likely cause, not verified:
  NDBC computes APD from a spectrum truncated at high frequencies, while the
  model spectrum extends further, which shortens Tm02.

Fixtures (`mock/ecmwf/`, 2026-09-23 00z, 15 MB):
- IFS HRES 24h: the whole file (13 messages, 10.7 MB) and its index, so index
  offsets apply to it directly.
- IFS ENS 24h: `swh` for members 1 and 2, `h1417` for member 1, and the full index.
- IFS ENS probabilities: `swhg2` at 24h.
- AIFS single and AIFS ENS control: `h1417` at 24h (PDT 4.103 and control).
- AIFS ENS probabilities: `swhg2` at 24h and `mwpg10` over 120–168h, plus the
  full index.
- IFS HRES `oper` 24h: `10u` and `10v` (1.5 MB) and the full index.

Also `mock/hrrr.20260923.t00z.wrfsfcf00.tmp2m.grib2` (1.2 MB) for projected grids.

## Examples

- `examples/ecmwf_point_forecast.rs`: IFS HRES wave and wind table at a point,
  and IFS ENS statistics for chosen hours (`MAX_HOUR`, `ENS_HOURS`), using index
  selection and range reads. About 37 s for 0–72 h plus two ensemble steps.
- `examples/validate_against_ndbc.rs`: the buoy comparison above (`RUN`,
  `MAX_HOUR`).
- `examples/gen_surf_forecast_gfs.rs`: now takes `END_HOUR` to shorten the run.

## Phases

All in PR #6, one commit per phase.

1. Trait rename, `NOAADataSource`, `GridSampler`, migrate the GFS record and
   example, delete the broken query functions, add the regression tests.
   Crate → 0.2.0.
2. `ECMWFWaveModel`/`ECMWFDataSource`, forecast hours, index selection and
   range merging, `ECMWFField`, fixtures.
3. `ECMWFWavePointDataRecord` and derived quantities.
4. Ensemble record and exceedance probabilities.
5. Wind from `oper`/`enfo`.
6. Follow-ups: projected-grid sampling, `Location::distance` fix,
   `DataRecordParsingError` implements `std::error::Error`, examples, buoy
   validation.

## Decisions

- Trait name: `GriddedModel`.
- Clean break, no deprecated re-exports. Crate is `0.2.0`.
- Coastal fallback defaults to 30 km (about one 0.25° cell; the diagonal is
  ~35 km at 41°N).
- Fixtures of about 10 MB in `mock/` are fine. The branch adds about 20 MB in
  total, mostly the full IFS HRES wave step (10.7 MB).

## Other changes

- `Location::distance` used `sin((Δ/2)²)` instead of `sin²(Δ/2)`, and converted
  latitudes with `absolute_latitude()` (−30° became 150°). Both fixed, with
  tests.
- `DataRecordParsingError` implements `std::error::Error`, so `?` works in
  functions returning `Box<dyn Error>`.
