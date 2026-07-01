#!/usr/bin/env python3
"""
build_lot_lookup.py

Merges NYC Building Footprints and MapPLUTO into a compact binary
lookup table keyed by BIN (Building Identification Number).

Usage:
    python build_lot_lookup.py [footprints.csv] [pluto.csv] [output.bin.gz]

Defaults:
    FOOTPRINTS_CSV = './data/BUILDING_20260513.csv'
    PLUTO_CSV      = './data/pluto_25v4.csv'
    OUTPUT_FILE    = './res/lot_lookup.bin.gz'

Output binary format:

Header  (12 bytes, little-endian):
  magic       4s   b'LOTD'
  version     B    = 1
  _pad        B    = 0
  record_size H    = 92
  count       I    record count

Records (N × 92 bytes, sorted ascending by BIN, little-endian):
  bin         I    u32   Building Identification Number (BIN), sorted by key, matches nycdoitt:bin in MVT tiles
  bbl         Q    u64   NYC (Borough Block Lot) BBL from BASE_BBL
  height_m    f    f32   roof height in meters, converted from feet
  assessed    I    u32   assesstot in whole dollars, determined by Department of Finance. PLUTO May 2026
  exempt      I    u32   exempttot in whole dollars, how much of assesstot does not influence tax.
  lot_area    I    u32   lot area in square feet
  res_far     f    f32   residential floor area ratio (max buildable density)
  units_res   H    u16   residential unit count
  units_total H    u16   total unit count
  num_floors  B    u8    number of floors (rounded, capped at 255)
  landuse     B    u8    Department of City Planning land use code (01–11 as integer) For example 02 is for one & two family buildings

  Inferred:
  bldg_class  2s         2-char building class e.g. b'D4', zero-padded, categories like commercial, residential, industrial, or combination.
  ownership   B    u8    ownership tier (see OWN_* constants below)
  _pad        3x         3 padding bytes
  owner       48s        owner name, UTF-8, zero-padded, truncated at 48

Corresponding Rust struct (repr(C, packed), 92 bytes):
  pub struct LotRecord {
      pub bin:         u32,
      pub bbl:         u64,
      pub height_m:    f32,
      pub assessed:    u32,
      pub exempt:      u32,
      pub lot_area:    u32,
      pub res_far:     f32,
      pub units_res:   u16,
      pub units_total: u16,
      pub num_floors:  u8,
      pub landuse:     u8,
      pub bldg_class:  [u8; 2],
      pub ownership:   u8,
      pub _pad:        [u8; 3],
      pub owner:       [u8; 48],
  }

Ownership tiers based on ownertype field in PLUTO or custom string matching.

  0  MARKET          Private, market-rate (ownertype P or blank, no keywords)
  1  CITY            City-owned or mixed city/private (ownertype C or M)
  2  AUTHORITY       Public authority, state or federal (ownertype O)
  3  EXEMPT_RES      Fully exempt and residential units (ownertype X and unitsres > 0)
                     Catches HDFCs (Housing Development Fund Corporation), Article XI, faith-based affordable housing
  4  KEYWORD         Keyword match only like Mitchell-Lama, HDFC names, etc.
                     ownertype may be P or blank for these regulated buildings
  5  PARKING         Parking lot or garage with no residential units
                     Simulation construction target; not housing yet
"""

import csv
import gzip
import struct
import sys
from pathlib import Path

# File configuration

FOOTPRINTS_CSV = "./data/BUILDING_20260513.csv"
PLUTO_CSV = "./data/pluto_25v4.csv"
OUTPUT_FILE = "./res/lot_lookup.bin.gz"

# Binary layout

MAGIC = b"LOTD"
VERSION = 1

RECORD_FMT = "<IQfIIIfHHBB2sB3x48s"
RECORD_SIZE = struct.calcsize(RECORD_FMT)  # must be 92

HEADER_FMT = "<4sBBHI"  # magic version pad record_size count
HEADER_SIZE = struct.calcsize(HEADER_FMT)  # 12

assert RECORD_SIZE == 92, (
    f"Record format mismatch — expected 92 bytes, got {RECORD_SIZE}. Check RECORD_FMT."
)

# Ownership tier constants

OWN_MARKET = 0
OWN_CITY = 1
OWN_AUTHORITY = 2
OWN_EXEMPT_RES = 3
OWN_KEYWORD = 4
OWN_PARKING = 5

OWN_LABELS = {
    OWN_MARKET: "Market rate",
    OWN_CITY: "City-owned (C/M)",
    OWN_AUTHORITY: "Public authority (O)",
    OWN_EXEMPT_RES: "Exempt residential (X+units)",
    OWN_KEYWORD: "Keyword match",
    OWN_PARKING: "Parking / garage",
}

# Non-market keyword lists
# Keyword matching is the last-resort tier for Mitchell-Lama and similar
# programs that are privately listed (ownertype P or blank) but regulated.

NYCHA_KEYWORDS = [
    "NYCHA",
    "HOUSING AUTHORITY",
    "NYC HOUSING",
    "N Y C H A",
    "CITY HOUSING AUTH",
]
AFFORDABLE_KEYWORDS = [
    "HDFC",
    "HOUSING DEV FUND",
    "HOUSING DEVELOPMENT FUND",
    "MITCHELL LAMA",
    "MUTUAL HOUSING",
    "LIMITED DIVIDEND",
    "REDEVELOPMENT CO",
]
ALL_KEYWORDS = NYCHA_KEYWORDS + AFFORDABLE_KEYWORDS

# BIN values that are NYC placeholders for "unknown building".
# Skip these as they cannot be matched to a tile feature.
INVALID_BINS = {0, 1_000_000, 2_000_000, 3_000_000, 4_000_000, 5_000_000}

# Helpers


def clean_int(val: str, default: int = 0) -> int:
    """
    Parse integers that may carry thousands-separator commas in the
    Building Footprints export, for example '507,357' converted to 507357.
    """
    try:
        return int(val.replace(",", "").strip())
    except (ValueError, AttributeError):
        return default


def clean_float(val: str, default: float = 0.0) -> float:
    """
    Same as clean_int but returns float. Handles both '1,950' and '1950'.
    """
    try:
        return float(val.replace(",", "").strip())
    except (ValueError, AttributeError):
        return default


def bbl_to_int(val: str) -> int:
    """
    PLUTO stores BBL as a float string like '4061730023.00000000' which is converted to 4061730023.
    Building Footprints BASE_BBL is a plain integer string: '4061730023'.
    Both are handled safely here.
    """
    try:
        return int(float(val.replace(",", "").strip()))
    except (ValueError, AttributeError):
        return 0


def encode_owner(name: str) -> bytes:
    """UTF-8 encode owner name, hard-truncate and zero-pad to exactly 48 bytes."""
    return name.encode("utf-8", errors="replace")[:48].ljust(48, b"\x00")


def encode_bldg_class(cls: str) -> bytes:
    """
    2-byte building class code.
    'D4' to b'D4',  'A' to b'A\x00',  '' to b'\x00\x00'
    """
    return cls.strip().encode("ascii", errors="replace")[:2].ljust(2, b"\x00")


def cap_u32(val: int) -> int:
    """Clamp to u32 range. Assessed values on large commercial lots can be huge."""
    return min(max(val, 0), 0xFFFF_FFFF)


def cap_u16(val: int) -> int:
    return min(max(val, 0), 0xFFFF)


def cap_u8(val: int) -> int:
    return min(max(val, 0), 0xFF)


def classify_ownership(
    ownertype: str, ownername: str, units_res: int, landuse: int, bldg_class: str
) -> int:
    """
    Determine the ownership tier for a lot.

    Tier priority:
      1. City or mixed city-private     = ownertype C or M
      2. Public authority or state/fed  = ownertype O
      3. Exempt and residential         = ownertype X and units_res > 0
      4. Keyword match                  = name contains known affordable operator
      5. Parking lot or garage          = landuse 10 or bldgclass G*, no units
      6. Market rate                    = everything else
    """
    ot = ownertype.strip().upper()
    name = ownername.upper()

    if ot in ("C", "M"):
        return OWN_CITY

    if ot == "O":
        return OWN_AUTHORITY

    if ot == "X" and units_res > 0:
        return OWN_EXEMPT_RES

    if any(kw in name for kw in ALL_KEYWORDS):
        return OWN_KEYWORD

    # Parking lots that have no residential units.
    if units_res == 0 and (landuse == 10 or bldg_class.upper().startswith("G")):
        return OWN_PARKING

    return OWN_MARKET


def should_keep(units_res: int, landuse: int, bldg_class: str) -> bool:
    """
    Filter to only the records relevant to the housing simulation.
    Drops bridges, parks, infrastructure, pure commercial, warehouses, etc.
    """
    if units_res > 0:
        return True  # residential building
    if landuse == 10:
        return True  # parking facility
    if bldg_class.upper().startswith("G"):
        return True  # garage
    return False


def load_pluto(path: str) -> dict:
    """
    Reads MapPLUTO CSV and returns a dict keyed by BBL integer.
    Only the fields needed for LotRecord are retained to keep memory low.
    """
    print(f"Loading PLUTO: {path}")
    pluto = {}
    skipped = 0

    with open(path, newline="", encoding="utf-8-sig") as f:
        reader = csv.DictReader(f)

        for row in reader:
            bbl = bbl_to_int(row.get("bbl", ""))
            if bbl == 0:
                skipped += 1
                continue

            floors_raw = clean_float(row.get("numfloors", "0"))
            landuse_raw = clean_int(row.get("landuse", "0"))

            pluto[bbl] = {
                "assessed": cap_u32(int(clean_float(row.get("assesstot", "0")))),
                "exempt": cap_u32(int(clean_float(row.get("exempttot", "0")))),
                "lot_area": cap_u32(int(clean_float(row.get("lotarea", "0")))),
                "res_far": clean_float(row.get("residfar", "0")),
                "units_res": cap_u16(clean_int(row.get("unitsres", "0"))),
                "units_total": cap_u16(clean_int(row.get("unitstotal", "0"))),
                "num_floors": cap_u8(round(floors_raw)),
                "landuse": cap_u8(landuse_raw),
                "bldg_class": row.get("bldgclass", "").strip()[:2],
                "ownertype": row.get("ownertype", "").strip(),
                "ownername": row.get("ownername", "").strip(),
            }

    print(f"  {len(pluto):>10,} lots loaded  ({skipped:,} skipped — no valid BBL)")
    return pluto


def build_records(footprints_path: str, pluto: dict) -> tuple:
    """
    Streams the Building Footprints CSV row by row, joins each building to
    its PLUTO lot via the 'Map Pluto BBL' column, filters, and packs into
    92-byte binary records.

    Returns (records, stats) where records is a list of (bin_int, bytes).
    """
    print(f"Processing Building Footprints: {footprints_path}")

    records = []
    stats = {
        "total": 0,
        "skipped_no_bin": 0,
        "skipped_bad_bin": 0,
        "skipped_no_pluto": 0,
        "skipped_filter": 0,
        OWN_MARKET: 0,
        OWN_CITY: 0,
        OWN_AUTHORITY: 0,
        OWN_EXEMPT_RES: 0,
        OWN_KEYWORD: 0,
        OWN_PARKING: 0,
        "missing_height": 0,
        "duplicate_bin": 0,
    }
    stats["units_res_nonmarket"] = 0
    stats["units_res_total"] = 0

    seen_bins = set()

    with open(footprints_path, newline="", encoding="utf-8-sig") as f:
        reader = csv.DictReader(f)

        for row in reader:
            stats["total"] += 1

            # BIN
            raw_bin = row.get("BIN", "").strip()
            if not raw_bin:
                stats["skipped_no_bin"] += 1
                continue

            bin_int = clean_int(raw_bin)
            if bin_int in INVALID_BINS or bin_int <= 0:
                stats["skipped_bad_bin"] += 1
                continue

            # Footprints can have duplicate BINs for multi-polygon buildings.
            # Keep the first occurrence which is typically the largest polygon.
            if bin_int in seen_bins:
                stats["duplicate_bin"] += 1
                continue
            seen_bins.add(bin_int)

            # PLUTO join
            # Use 'Map Pluto BBL', not 'BASE_BBL' as they differ for condos.
            map_bbl = bbl_to_int(row.get("Map Pluto BBL", ""))
            p = pluto.get(map_bbl)
            if p is None:
                stats["skipped_no_pluto"] += 1
                continue

            # Filter
            if not should_keep(p["units_res"], p["landuse"], p["bldg_class"]):
                stats["skipped_filter"] += 1
                continue

            # Height: feet to meters
            height_ft = clean_float(row.get("Height Roof", "0"))
            if height_ft <= 0.0:
                stats["missing_height"] += 1
            height_m = height_ft * 0.3048

            # BASE_BBL for the stored BBL field.
            base_bbl = bbl_to_int(row.get("BASE_BBL", ""))

            # Ownership tier
            tier = classify_ownership(
                p["ownertype"],
                p["ownername"],
                p["units_res"],
                p["landuse"],
                p["bldg_class"],
            )
            stats[tier] += 1
            stats["units_res_total"] += p["units_res"]
            if tier != OWN_MARKET and tier != OWN_PARKING:
                stats["units_res_nonmarket"] += p["units_res"]

            # Pack 92-byte record
            record = struct.pack(
                RECORD_FMT,
                bin_int,
                base_bbl,
                height_m,
                p["assessed"],
                p["exempt"],
                p["lot_area"],
                p["res_far"],
                p["units_res"],
                p["units_total"],
                p["num_floors"],
                p["landuse"],
                encode_bldg_class(p["bldg_class"]),
                tier,
                encode_owner(p["ownername"]),
            )
            records.append((bin_int, record))

            if stats["total"] % 100_000 == 0:
                print(
                    f"  {stats['total']:>10,} rows scanned  "
                    f"/ {len(records):,} kept so far ..."
                )

    return records, stats


def write_output(records: list, output_path: str) -> None:
    """
    Sorts records by BIN ascending, writes 12-byte header followed by
    all records, compressed with gzip level 9.
    """
    print(f"Sorting {len(records):,} records by BIN ...")
    records.sort(key=lambda r: r[0])

    print(f"Writing {output_path} ...")
    header = struct.pack(
        HEADER_FMT,
        MAGIC,
        VERSION,
        0,
        RECORD_SIZE,
        len(records),
    )

    with gzip.open(output_path, "wb", compresslevel=9) as gz:
        gz.write(header)
        for _, record in records:
            gz.write(record)


def print_stats(stats: dict, record_count: int, output_path: str) -> None:
    n = max(record_count, 1)
    sep = "─" * 56
    dsep = "═" * 56

    def pct(v):
        return f"{v / n * 100:5.1f}%"

    print()
    print(dsep)
    print(f"  Footprint rows scanned         : {stats['total']:>10,}")
    print(sep)
    print(f"  Skipped — no BIN               : {stats['skipped_no_bin']:>10,}")
    print(f"  Skipped — invalid/placeholder  : {stats['skipped_bad_bin']:>10,}")
    print(f"  Skipped — duplicate BIN        : {stats['duplicate_bin']:>10,}")
    print(f"  Skipped — no PLUTO match       : {stats['skipped_no_pluto']:>10,}")
    print(f"  Skipped — not residential/park : {stats['skipped_filter']:>10,}")
    print(sep)
    print(f"  Records written                : {record_count:>10,}")
    print(sep)
    for tier, label in OWN_LABELS.items():
        count = stats[tier]
        print(f"    {label:<32} : {count:>7,}  {pct(count)}")
    print(sep)
    print(
        f"  Missing height data            : {stats['missing_height']:>10,}  "
        f"{pct(stats['missing_height'])}"
    )
    print(sep)
    print(f"  Record size                    : {RECORD_SIZE} bytes")
    print(
        f"  Uncompressed size              : "
        f"{record_count * RECORD_SIZE / 1_048_576:.1f} MB"
    )
    compressed = Path(output_path).stat().st_size
    print(f"  Compressed size (gzip -9)      : {compressed / 1_048_576:.1f} MB")
    print(dsep)

    # Check if non-market housing is roughly 30–50% of residential records in NYC.
    units_total = stats.get("units_res_total", 0)
    units_nonmarket = stats.get("units_res_nonmarket", 0)
    if units_total > 0:
        nm_unit_pct = units_nonmarket / units_total * 100
        flag = "  <- verify" if not (15 <= nm_unit_pct <= 55) else ""
        print(f"  Non-market share by units      : {nm_unit_pct:.1f}%{flag}")
        print(
            f"  Non-market units               : {units_nonmarket:,} of {units_total:,}"
        )
    print()


def main() -> None:
    footprints_path = sys.argv[1] if len(sys.argv) > 1 else FOOTPRINTS_CSV
    pluto_path = sys.argv[2] if len(sys.argv) > 2 else PLUTO_CSV
    output_path = sys.argv[3] if len(sys.argv) > 3 else OUTPUT_FILE

    missing = [p for p in (footprints_path, pluto_path) if not Path(p).exists()]
    if missing:
        for m in missing:
            print(f"Error: file not found: {m}")
        sys.exit(1)

    pluto = load_pluto(pluto_path)
    records, stats = build_records(footprints_path, pluto)
    write_output(records, output_path)
    print_stats(stats, len(records), output_path)
    print(f"Done → {output_path}")


if __name__ == "__main__":
    main()
