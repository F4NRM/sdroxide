# SYNOP / SHIP — the research, and what stopped the build

**Status: researched and specified; the decoder is drafted but **not** correct
yet, and nothing is wired.** This file is the part that is solid, so the next
session does not re-derive it.

Requested by **Kevin (F4NRM)** on 2026-10-14: coded weather reports broadcast on
HF in RTTY, every three hours.

---

## The schedule — the published one, not a guess

DWD's [RTTY broadcast for shipping](https://www.dwd.de/EN/specialusers/shipping/broadcast_en/broadcast_rtty_1_092023.pdf)
is the clearest published schedule. **50 baud F1B, ±225 Hz** — a mode this fork
already decodes — on **4583**, **7646** and **10100.8 kHz**.

| UTC | Code | What |
|---|---|---|
| 00:35, 06:30, 12:10, 18:30 | **SYNOP** — WMO FM 12 | land stations |
| 02:00, 07:35, 13:35, 19:35 | **SHIP** — WMO FM 13 | ships |
| 08:15, 20:15 | BUOY — FM 13 | buoys |

**"Every three hours" is Kevin's shorthand and is nearly right** — the slots run
roughly every six hours with two different codes, SHIP at :35 and SYNOP at
:10/:30. Worth saying plainly rather than shipping his phrasing as the spec.

## The code forms

Two, and they differ mainly in the oceanographic groups:

- **FM 12 SYNOP** — land stations.
- **FM 13 SHIP** — ships. 23 groups.

Source for both: WMO Manual on Codes as reproduced in the NOAA *Observing
Handbook* (vos.noaa.gov) and JMA's *Guide to Ships' Weather Reports*
(marine.kishou.go.jp). A third, very readable group-by-group walkthrough is
NPS's [FM 13-IX SHIP decode sheet](https://met.nps.edu/~bcreasey/mr3222/files/helpful/DecodeShipObservationCode.pdf)
— it is the one the draft was written against, because it lists the code tables
inline.

**Structure (FM 13):**

```
BBXX DDDD YYGGiw 99LaLaLa QcLoLoLoLo iRixhVV Nddff 00fff 1snTTT 2snTdTdTd
4PPPP 5appp 7wwW1W2 8NhCLCMCH 9GGgg 222DsVs 0ssTwTwTw 2PwPwHwHw
3dw1dw1dw2dw2 4Pw1Pw1Hw1Hw1 5Pw2Pw2Hw2Hw2 6IsEsEsRs 8swTbTbTb ICE ciSibiDizi
```

Section 0 = identification (first five groups, mandatory). Section 1 = the
weather. Section 2 = oceanographic and ice.

## The code tables a decoder needs

| Table | Group | What it holds |
|---|---|---|
| 1855 | `iw` | wind-speed unit — **m/s / knots / km/h** |
| 3333 | `Qc` | longitude quadrant (1 NE, 3 SE, 5 SW, 7 NW) |
| 4377 | `VV` | visibility, groups **90-99** only (ship codes) |
| 2700 | `N`, `Nh` | cloud amount in oktas, 9 = sky obscured |
| 4677 | `ww` | present weather |
| 4561 | `W1W2` | past weather |
| 0200 | `a` | characteristic of pressure tendency |
| 0513 / 0515 / 0509 | `CL CM CH` | cloud genera |
| 4451 | `vs` | ship's speed — **a range**, not a value |

## Two things the draft got wrong, which a fixture made up would have hidden

This is the whole lesson of the attempt, and it is the same one the satellite
work hit.

1. **`iw` is a two-figure code, not one digit.** Reading one digit puts the
   wind in the wrong unit and *nothing looks wrong* — the number is still a
   number.
2. **`QcLoLoLoLo` is nine figures** — one quadrant plus **eight** longitude
   digits, because `LoLoLoLo` is in tenths of a degree. A five-digit reading
   looks exactly like a position and is not one.

And the structural fault behind the failing tests:

3. **The dispatch guards were too loose.** The guard written for the
   `YYGGiw` time group — *six digits, first two a day 1-31, next two an hour* —
   also matches the nine-digit longitude group `100023` (day `10`, hour `00`).
   Groups must be told apart **by their length and their position in the
   message**, not by a loose numeric range. That is the same "the indicator is
   not enough" rule the SHIP groups obey, since `1` opens both a temperature
   group and a sea-wave group.

## Why it stopped here

**The test fixture was invented.** `BBXX GBRY 151204 99545 100023 …` was written
from the published *structure* with made-up values. That is enough to test
parsing and **not enough to claim it decodes anything** — and it is exactly the
error this whole task was meant to avoid, repeated by me inside the attempt to
avoid it.

Six of eleven tests failed against it. **A red tree was not going to be
committed**, so the module is out of the tree; the draft is at
`/tmp/opencode/synop-draft/synop.rs` and should be pasted over by the next
session rather than rewritten, since the code tables in it are sound.

## What is left, in order

1. **Capture a real broadcast.** 4583 or 7646 kHz at 02:00 UTC (SHIP) or
   00:35 UTC (SYNOP). That single recording is worth more than any amount of
   reading — it settles the framing, the line terminator, whether a callsign
   appears, and how much of a report survives at HF.
2. **Fix the dispatch** — group identification by length and position, the two
   bugs above, and the shared leading digits (`1`, `2`, `4` open more than one
   group).
3. **Then** the rest: RTTY wiring, a window, the manual, a quick-start.