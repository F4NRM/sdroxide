# SWL handover — schedule sources, and the globe's propagation paths

A fork-only piece of work in progress. **Nothing here has been built yet** — this
file exists so the plan, the evidence and the decisions survive a session, and so
the work can be picked up by anyone (or any agent) cold.

**Status: designed and scoped, no code written.** Last updated **2026-10-13**.

The trigger is fork **issue #69** (bebepip): *"import dB other than EiBi"*. The
scope grew when we found **shortwave-atlas**, an independent MIT project eight
days old whose `atlas-core` already solves the architecture we need.

---

## What this must not become

It is **fork-only**. Upstream has no audience for a second broadcast schedule,
and the maintainer's queue is long. Do not open a PR for any of it. If a piece
later turns out to be general, that is a separate decision, made then.

---

## 1. What we already have (verified in the tree, 2026-10-13)

Almost all of the *display* side is built and does not care where rows came from:

- `sdroxide_types::broadcast::parse_schedule` — the parser, and the wall
- the **SCHEDULE window** — filter by UTC time or now, metre band, language,
  target, free text over name/site/country/language/target
- **★ FAVS** favourites in `broadcast_favourites.json`, keyed by station name
- **broadcast spots on the waterfall** — the synthesis from schedule → spots
- the SWL reception log's **pre-fill** from the schedule (`prefill_station`)
- band scans, "what is on now"

**So any source feeding the parser lights up all of it at once.** That is why
this is a contained job: the work is entirely in *getting rows in*, not in
putting them on screen.

## 2. The wall

`parse_schedule` is welded to **two** things, and only one of them is obvious:

1. **EiBi's column layout** — semicolon-delimited, fixed order
2. **EiBi's code tables** — `broadcast_codes.json` resolves its language and
   target-area codes, and `resolve_site` its transmitter sites

Bending one parser to guess at three formats is the mistake to avoid. Each
source is a small reader; they share the vocabulary and the gazetteer.

## 3. What bebepip actually asked for (#69)

He runs **Aoki** ("I use an Aoki-based system for my radio room"). His answer,
2026-10-12, in full:

- **Which source:** Aoki, with HFCC named as the alternative.
- **Replace or alongside:** he proposed *"EiBi, Aoki, HFCC, or all of them."*
- **Where entries appear:** he did not answer that framing — he asked for
  something better. Station names **on the waterfall**, for signal ID, and said
  so twice: *"Shows the station´s name on the waterfall is very useful for easy
  signal´s id."* He also asked for a **checkbox** so only the tuned station is
  shown, rather than the display being cluttered.
- He gave the sources: Aoki at `starcat.ne.jp/ndxc/`, HFCC at `hfcc.org/data`.
- He listed the other things a listener might keep: **UDFX, fax, NAVTEX, GMDSS,
  Pactor** — all of which are *user* sources, not published schedules.

### His screenshots did not survive

The three images he attached are stored by GitHub as issue attachments, not
inline in the comment body, so the API returns `[image: aoki.jpg]` placeholders
and the bytes are not retrievable. **Do not rely on them** — he pasted the
HFCC rows as text instead, and the Aoki layout came from the operator's own
screenshot on disk.

## 4. The three formats, decoded

### EiBi — ours today
Semicolon-delimited, fixed column order. **Station name inline.** Language and
target are *codes* resolved through `broadcast_codes.json`.

### Aoki — `ncb17.txt`, and the one he uses

**One file, station names inline.** Space-aligned fixed columns, *not*
semicolons — this is the trap, and it is why a parser written for EiBi cannot be
nudged into reading it.

```
B17 Shortwave Frequency list  March 23  2018  0100UTC  Day 1 = Sunday
```

| # | Column | Example | Notes |
|---|---|---|---|
| 1 | FREQ kHz | `198` | |
| 2 | **STATION** | `BBC`, `R.Algerie Chaine 1` | **full name inline** |
| 3 | UTC window | `0100-0600` | crosses midnight as `2300-1600` |
| 4 | Su-W-Sa days | `1234567` | **Day 1 = Sunday** |
| 5 | Language | `French/Arabic/` | slash-separated, trailing slash |
| 6 | Pow kW | `1000` | |
| 7 | Azi | `ND` | azimuth; `ND` = nondirectional |
| 8 | Location | `Droitwich`, `Ouargla` | transmitter site |
| 9 | ADM | `G`, `ALG`, `MNG` | country/admin code — **needs a lookup** |
| 10 | L/L | `3722N14051E` | DMS, hemisphere in the letter |
| 11 | **Remarks** | `NICT`, `BBC R4`, `MEDI1` | free text: agency or service |
| 12 | Lat | `37 22` | redundant with 10 |
| 13 | Long | `-140 51` | |

Two traps, both worth their own test:

- **The day numbering is Sunday-based** (the header says so). A Monday-first
  parser shifts every schedule by a day — wrong, and wrong *quietly*.
- **Coordinates appear twice**, in two formats. A cross-check, and an ambiguity
  to decide once rather than twice.

### HFCC — and the gotcha he caught himself

His follow-up: *"Ooops i make a mistake — HFCC needs also a station´s name file
that comes in separate txt file."*

**Space-delimited**, unlike EiBi. And the row carries a **3-letter code**, not a
name:

```
3195 2100 0100 4,9,27,37-39                   WCR  100 46  0 902 1234567 010626 310826 D  Eng  USA WCR FCC  2347
```

`WCR`, `RNZ`, `VBT` — a **companion file** resolves them to names. Days carry
ranges (`37-39`) and target zones (`40E`, `41NW`); dates are `DDMMYY`.

**He pasted 377 real rows as text in the thread.** That is better evidence than
his screenshots and it is what the parser should be written against.

**The consequence for the design: a source owns more than one file.** EiBi and
Aoki are one file each; HFCC is a pair, and loading it is a two-step that can
half-fail. Whatever holds sources has to say so honestly rather than showing an
HFCC schedule full of `WCR`.

## 5. The design decision — one active source

Bebepip proposed *"all of them"*. **We are not doing that by default**, and the
reason is his own observation: *"Records are often duplicated (common to each
one)"*. All three at once is ~2,000 rows with heavy overlap, and the waterfall —
where he wants the name, for signal ID — would show one station three times.

**One active source at a time.** The waterfall and the labels can only ever come
from one, so duplicates are *impossible* rather than filtered out. Switching is
one setting.

If a comparison view is ever wanted, *"show all"* belongs in the **schedule
window** only — a browsable list where three entries for one station is
information rather than clutter — while the waterfall stays single-source.
That is a later addition and is not implied by anything above.

Also adopt from his message: the **checkbox for "only the tuned station"**, which
is about display density and is independent of which source is selected.

## 6. The plan, staged

Each stage is independently shippable; nothing below depends on a stage above it
being perfect.

1. **The source abstraction** — the keystone. `EiBi` becomes source #1 rather
   than *the* source. A `ScheduleSource` that turns bytes into `BroadcastStation`
   rows, a shared `Vocabulary` (code tables) and `Gazetteer` (transmitter sites),
   and a one-active-source selector. **Pure, in `sdroxide-types`, wasm-safe, no
   wire change, no `PROTO_VERSION` bump.** EiBi's behaviour must be byte-for-byte
   unchanged when this lands — that is the acceptance test.
2. **Aoki** — the format above, one file. He runs it, so it is the one that
   makes a listener's life better first. The Sunday-based day numbering gets its
   own test.
3. **User stations** — the listener's own list as a real source, in the same
   shape, so local services, unlisted stations, pirates and the things UDFX /
   fax / NAVTEX / GMDSS / Pacter lists carry appear *alongside* everything else
   instead of only being recordable in the log after hearing them.
4. **HFCC** — space-delimited *plus* the companion name file. Last, because it is
   the one with two files and a half-failure mode.
5. **The tuned-station checkbox** and any waterfall density work.

## 7. The globe — propagation paths (wanted by the maintainer)

The operator's words: *"I do want that globe projection too from shortwave
atlas… we have an awesome globe window."*

Agreed, and there is something in their `atlas-core/src/geo.rs` we do **not**
have. We already have distance, bearing, the subsolar point and the day/night
terminator (`sdroxide_types::geo`, `sdroxide_solar::ephem`) — and the IBP beacon
beams draw on them. What is missing is the path itself:

| Their function | What it is | Do we have it |
|---|---|---|
| `along(a, b, f)` | the point a fraction `f` along the great circle | **no** — needed to draw |
| `dark_share(a, b, unix)` | **the share of the path currently in darkness** | **no** — and this is the good one |
| `dist_bearing(a, b)` | haversine + initial bearing | yes, ours |
| `subsolar(unix)`, `sun_at` | solar position and day/twilight/night | yes, ours |

Their README describes it as *"traces the path from your station across the live
day/night line"* — and `dark_share` is what makes that more than decoration: it
answers **how much of this path is in daylight right now**, which is a
propagation question a listener actually has.

So the shape is: on our 3D globe, draw the great-circle path from the station's
QTH to the transmitter, and show its dark share. Our globe is `solar3d`
(three.js) and our flat maps are egui — **their React/globe code is not
portable and must not be attempted**; the Rust `geo` maths is the portable part.

Open question for the operator: on the **3D globe**, the flat maps, or both.

## 8. shortwave-atlas — what it is and what it is not

`github.com/juantoledo/shortwave-atlas` — MIT, © 2026 Juan Toledo Carrasco.
Rust core (`atlas-core` / `atlas-rig` / `atlas-server`) + Tauri/React shell.
Created 2026-10-08, pushed 2026-10-12, live at shortwaveatlas.org. Not a fork of
ours.

**Most of its features we already have**, and this is worth stating so nobody
builds a duplicate: the EiBi schedule with search and lenses, a 3D globe with a
day/night line, Hamlib/rigctld rig control, a headless server with a browser UI,
an audio waterfall, and listening stats. Its feature list is not news.

**What is genuinely new to us, and worth taking:**

- `source.rs` — the `SourceKind::Schedule` / `SourceKind::User` split, with a
  shared `Vocabulary` and `Gazetteer`. This is the architecture of stage 1.
- `user_stations.rs` — the listener's own stations as a real source, with
  import/export and honest limits (500 stations, 24 slots, 4 languages per slot).
- `geo.rs`'s `along` and `dark_share`, per §7.

## 9. Licence and attribution — not optional

The project is **MIT**; we are **GPL-3.0-or-later**. Compatible, so this is
ordinary work.

**Every file adapted from it carries that project's full notice in the file.**
This is the same rule as the OpenHamClock-derived work already in the tree, and
it is a house rule, not a courtesy. A stage that adapts `source.rs` gets Juan
Toledo Carrasco's copyright in that file.

**Nothing may be vendored.** No submodule, no copy of the crate. Adapt the ideas,
write our code, carry the notice.

## 10. Traps, recorded so they are not walked into again

- **Do not bend one parser into guessing at four formats.** That is the design
  this document exists to prevent.
- **A source is not one file.** HFCC is two, and it can half-load. Say so when it
  fails rather than showing a schedule full of `WCR`.
- **Aoki's days start at Sunday.** A Monday-first parser is wrong by a day and
  quiet about it.
- **A missing schedule must never overwrite a good one.** `cached_schedule` /
  `MIN_SCHEDULE_ROWS` already refuses a truncated download and falls back to the
  compiled-in copy — a second source must not become a way to *lose* the
  schedule.
- **Verify the list entry before acting on it.** On 2026-10-12 three list items
  in `TODO.md` / `PROJECT-BOARD.md` turned out to be already done — DAB, the
  store/UI bugs, and **#442 transverters**, where a type and a settings table
  exist with no obvious engine reference and `converter_plan` in `src/main.rs`
  wires it completely. A stale "not built, do not pick it up" note sends you away
  from finished work; a stale "broken" note sends you to fix working code.
- **Numbers in a report are evidence.** For #68 and #72 a reporter's own figures
  named the mechanism on the first read. Where we have real rows (the 377 HFCC
  lines), write the parser against those, not against a description.

## 11. Open questions

- **Globe:** 3D only, flat only, or both? (§7)
- **Where does the source selector live** — Settings → Radio, the SCHEDULE
  window's own row, or both? The SCHEDULE window is where a listener looks.
- **Aoki's update cadence.** Importing a file he supplies is one thing; fetching
  and keeping Aoki or HFCC current means someone owns keeping it fresh. The
  user-file route deliberately avoids that commitment.
- **User stations on the wire?** They are station-shared in the sense a memory
  is, and a listener with two machines probably wants them on both. Not decided;
  it would be a `PROTO_VERSION` bump, so it is worth deciding early rather than
  bolting on.