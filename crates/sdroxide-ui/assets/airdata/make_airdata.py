#!/usr/bin/env python3
"""Build the aircraft-navigation tables the survey map draws from.

    python3 crates/sdroxide-ui/assets/airdata/make_airdata.py
    python3 crates/sdroxide-ui/assets/airdata/make_airdata.py --from ~/Downloads

Writes two committed assets beside this script:

    navaids.bin    every radio navigation aid in the world
    airports.bin   the large and medium airports

**Source: OurAirports, which is public domain.** Their data repository carries
the Unlicense — *"free and unencumbered software released into the public
domain… sell, or distribute… for any purpose"* — so there is no attribution
obligation and no share-alike to reason about. That is the whole reason this
source and not another: the paid and keyed ones are out (see the house rule in
`AGENTS.md`), OpenFlights is ODbL with a share-alike that reaches a derived
database, and OpenAIP is non-commercial.

Files, from <https://ourairports.com/data/> (mirrored on GitHub Pages):

    https://davidmegginson.github.io/ourairports-data/navaids.csv
    https://davidmegginson.github.io/ourairports-data/airports.csv

Nothing fetches at build time or at run time — this script is run by a person
when the data wants refreshing, exactly like `make_earth_maps.py` beside the
Earth rasters. The assets it writes are committed.

**`--from DIR` reads CSVs already on disk instead of downloading them**, which
is the route to use when the machine's Python has no certificate store (an
Anaconda install often does not) or when the machine is offline: save
`navaids.csv` and `airports.csv` from <https://ourairports.com/data/> into one
directory and point the script at it.

## Why these tables and not the whole thing

`airports.csv` is 86 258 rows and most of them are heliports and grass strips:
at map scale they are noise, and every one is bytes in the binary. Only
`large_airport` and `medium_airport` are kept — 5 280 of them, which is what a
person could actually fly to.

Every navaid is kept, because they are not just map furniture: **6 700 of them
are NDBs below 1 800 kHz**, which are receivable on an ordinary medium-wave
receiver. A list of them is a listening target list as much as a map layer.

## Format

A magic, then a count, then fixed-size records — little-endian, strings
NUL-padded, `make_earth_maps.py`'s convention. Fixed size rather than
variable so a reader can index a record without walking the file.

    navaids.bin   "SDXNAV01" u16 count
                  { ident[5] name[16] type u8 country[2] freq_khz u32 lat f32 lon f32 }
    airports.bin  "SDXAPT01" u16 count
                  { ident[8] name[20] iata[3] icao[4] type u8 lat f32 lon f32 elev_ft i16 }

Latitude and longitude are `f32`: about a metre at the equator, and these are
map positions. `freq_khz` is `u32` because a VOR at 116 300 kHz and an NDB at
291 kHz are the same field, and kHz is the unit both are published in.
"""

from __future__ import annotations

import csv
import io
import os
import struct
import sys
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
BASE = "https://davidmegginson.github.io/ourairports-data"

# The navaid types, in the order they are numbered in the file. An index rather
# than a string: this is the field the map colours and sizes its symbols by.
NAVAID_TYPES = ["NDB", "VOR", "VOR-DME", "VORTAC", "TACAN", "DME", "NDB-DME"]

AIRPORT_TYPES = ["large_airport", "medium_airport"]


#: Where `--from` points, or None to download.
LOCAL: str | None = None


def fetch(name: str) -> str:
    if LOCAL:
        path = os.path.join(LOCAL, name)
        sys.stderr.write(f"reading {path}\n")
        return open(path, encoding="utf-8").read()
    url = f"{BASE}/{name}"
    sys.stderr.write(f"fetching {url}\n")
    try:
        with urllib.request.urlopen(url, timeout=120) as r:
            return r.read().decode("utf-8")
    except urllib.error.URLError as e:
        raise SystemExit(
            f"could not download {url}: {e}\n\n"
            "If this machine's Python has no certificate store, save navaids.csv "
            "and airports.csv from https://ourairports.com/data/ into a directory "
            "and run this again with --from <that directory>."
        ) from e


def fixed(text: str, n: int) -> bytes:
    """`text` as `n` bytes, NUL-padded — and cut, not truncated mid-character."""
    raw = (text or "").encode("utf-8")
    return raw[:n].ljust(n, b"\0")


def number(value, default=0.0) -> float:
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def build_navaids() -> bytes:
    rows = list(csv.DictReader(io.StringIO(fetch("navaids.csv"))))
    keep = [r for r in rows if r["type"] in NAVAID_TYPES and r["latitude_deg"]]
    out = io.BytesIO()
    out.write(b"SDXNAV01")
    out.write(struct.pack("<H", len(keep)))
    for r in keep:
        out.write(fixed(r["ident"], 5))
        out.write(fixed(r["name"], 16))
        out.write(struct.pack("<B", NAVAID_TYPES.index(r["type"])))
        out.write(fixed(r["iso_country"], 2))
        # Some rows carry a sentinel rather than a frequency; 0 means "not
        # published", and the map draws those as an unfrequency'd symbol rather
        # than at DC.
        out.write(struct.pack("<I", max(0, int(number(r["frequency_khz"])))))
        out.write(struct.pack("<f", number(r["latitude_deg"])))
        out.write(struct.pack("<f", number(r["longitude_deg"])))
    silent = sum(1 for r in keep if int(number(r["frequency_khz"])) <= 0)
    sys.stderr.write(
        f"  {len(keep)} navaids of {len(rows)} rows, {silent} with no frequency\n"
    )
    return out.getvalue()


def build_airports() -> bytes:
    rows = list(csv.DictReader(io.StringIO(fetch("airports.csv"))))
    keep = [r for r in rows if r["type"] in AIRPORT_TYPES and r["latitude_deg"]]
    out = io.BytesIO()
    out.write(b"SDXAPT01")
    out.write(struct.pack("<H", len(keep)))
    for r in keep:
        out.write(fixed(r["ident"], 8))
        out.write(fixed(r["name"], 20))
        out.write(fixed(r["iata_code"], 3))
        out.write(fixed(r["icao_code"] or r["gps_code"], 4))
        out.write(struct.pack("<B", AIRPORT_TYPES.index(r["type"])))
        out.write(struct.pack("<f", number(r["latitude_deg"])))
        out.write(struct.pack("<f", number(r["longitude_deg"])))
        out.write(struct.pack("<h", int(number(r["elevation_ft"]))))
    sys.stderr.write(f"  {len(keep)} airports of {len(rows)} rows\n")
    return out.getvalue()


def verify(path: str, magic: bytes, size: int, headers: int = 2) -> None:
    """Read it back the way the Rust reader will, and say what came out.

    Nothing here is a test — the reader has those — but a format that cannot be
    walked is a format that will fail later, and the walk is four lines.
    """
    blob = open(path, "rb").read()
    assert blob[:8] == magic, f"{path}: bad magic {blob[:8]!r}"
    (count,) = struct.unpack_from("<H", blob, 8)
    body = len(blob) - 10
    assert body == count * size, f"{path}: {body} bytes for {count} × {size}"
    sys.stderr.write(f"  wrote {path}: {len(blob) / 1024:.0f} KiB, {count} records\n")


def main() -> None:
    global LOCAL
    args = sys.argv[1:]
    if args[:1] == ["--from"]:
        LOCAL = os.path.expanduser(args[1]) if len(args) > 1 else "."
    navaids = build_navaids()
    airports = build_airports()
    open(os.path.join(HERE, "navaids.bin"), "wb").write(navaids)
    open(os.path.join(HERE, "airports.bin"), "wb").write(airports)

    # Record sizes, kept in step with the structs above and with the Rust
    # reader: 5+16+1+2+4+4+4 and 8+20+3+4+1+4+4+2.
    verify(os.path.join(HERE, "navaids.bin"), b"SDXNAV01", 36)
    verify(os.path.join(HERE, "airports.bin"), b"SDXAPT01", 46)


if __name__ == "__main__":
    main()
