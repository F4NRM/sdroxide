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
| `along(a, b, f)` | the point a fraction `f` along the great circle | **had it** — `great_circle_points`, theirs is the single-point form |
| `dark_share(a, b, unix)` | **the share of the path currently in darkness** | **yes, ours** — `ephem::dark_share` |
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

### Fetching Aoki is built (2026-10-13) — and the zip is not a one-file zip

Aoki publishes **no plain-text URL** (`ncb24.txt` answers 404), so the schedule
comes as a zip, and that turned out to matter more than the decoding did.

**`ncb24.zip` holds three entries: `ncb24.xlsx`, `userlist1.txt` and `ncb24.txt`.
The schedule is not the first `.txt`.** `userlist1.txt` is **a listener's own
saved list**, 474 kB, in a *different* Aoki format — frequency first, no ADM
column, no position column. So "the first `.txt`" takes a real file, from the
right publisher, in a real Aoki format, and reports **zero** stations. With a
user's list in the older layout it would have reported plausible, **wrong**
stations.

That is the worst class of fault here: nothing about the download looks amiss.
**Every fixture in the zip tests was a one-entry archive**, so every one of them
passed; only reading the publisher's actual file found it. It is now guarded by
`the_schedule_is_chosen_by_name_not_by_position`, which builds the zip with the
user list **first** and fails on any reader that picks by position — verified
failing, with the reporter's own words (*"the entry chosen was the user's list,
not the schedule"*). The chooser is by name (`ncb26.txt` for season `b26`), then
by content (a schedule file says `Shortwave Frequency List`; no user list does),
then largest, naming what it guessed.

**Measured against the published file** (`a_published_aoki_zip_yields_its_schedule`,
`#[ignore]`d, needs the download):

| | |
|---|---|
| entries | `userlist1.txt` 473,709 B · `ncb24.txt` 802,712 B |
| parsed | **3,516 stations** (not the 3,612 rows quoted earlier — that was a row count) |
| placed on the map | **3,512 / 3,516 = 99.9 %** |
| countries resolved | `KCBS Pyongyang` → *Korea, North*; `WWCR NASHVILLE` → *United States of America* |

**Both new dependencies were already in the tree**, which is why this was cheap:
`flate2` is here via `ureq` and `encoding_rs` via `symphonia-metadata`, so
inflating the zip and decoding CP932 cost **no new compilation**. The `zip` crate
would have added a dependency tree to a 59-crate workspace to read one file from
one publisher, so there are ~40 lines of a deliberately minimal reader instead —
central directory only, deflate or stored, anything else skipped rather than
fatal.

Two smaller things the tests caught, both worth keeping:

- **The reader's EOCD search was wrong for small files.** It scanned
  `0..len - 65`, which excludes the end-of-central-directory record from any
  archive under ~87 bytes — so it refused every small zip, including every one a
  test builds. Found by a test, not by reading.
- **`encoding_rs`'s Shift-JIS encoder writes an HTML numeric reference** for a
  character the encoding cannot hold (`录` U+5F55 → `&#24405;`, seven ASCII bytes)
  and *also* sets `had_errors`. So a `&#nnnn;` in a station name is the
  encoder's doing and not a decoding fault — pinned, so a future one is
  recognised rather than chased into the reader. CP932 has the traditional
  `録` (U+9332); the first version of the test used the simplified one.

`SWL-HANDOVER.md` §6 stage 2 said the whole file "was checked against these
columns while writing this" — that was a **row count, not a parse**, and it is
what let the wrong-entry fault through. The parse now runs.

### The maths is built (2026-10-13) — `ephem::dark_share`

**`along` turned out to be ours already, and writing it a second time was the
find.** `great_circle_points` (`geo.rs`) has been in the tree since the
propagation work and does the same interpolation in the all-at-once form; their
`along(a, b, f)` is the single-point version of the same slerp. So the first
pass at stage 3 wrote `along` and `along_many`, and the honest move was to
**delete both and keep theirs** rather than leave two implementations of one
formula to drift. What was kept from the deleted pair was folded into the
existing function's tests, so the coverage is not lost — and one of those tests
found a real hole:

- **`great_circle_points` divided by zero on antipodal points.** `sin(d)` is
  zero when `d = π`, so every sample came back identical — a "path" through the
  Earth. It is now guarded, answering the start point, because two points a
  whole hemisphere apart have **no single great circle** and asking for the one
  between them is asking for something that does not exist.
- Three new tests pin what the function is *for*: the midpoint **halves the
  distance** (a straight-line lerp of lat/lon does not, which is the whole
  point), crossing the antimeridian takes the **short** way round, and the
  endpoints land where they started.

**`dark_share` is ours and genuinely new**, in `sdroxide-solar::ephem` because
that is where our Sun already lives — it reuses `is_daylight_at` and the
existing `great_circle_points` rather than carrying a second copy of either.

What it answers: **how much of the route between this station and that
transmitter is currently in night**, as a fraction. Twilight counts as light —
"dark" means the Sun is actually below the horizon, not "below −6°".

It is **sampled** (nine points, an eighth of the path apart), not solved, so a
narrow dark region falling between two samples is under-reported. That is the
deliberate trade for something cheap enough to call while a map is being dragged,
and it is right to within about a tenth of the path. The doc says so.

**Verified failing against a constant.** The tests that bite are the two
solstice ones — a 60°N path in December must be darker than the same path in
June, and a midday equatorial path must be mostly light. Replacing the body
with `filter(|_| true)` fails **both** and passes the two structural ones, so
the pair is what is doing the work.

**Not drawn anywhere yet.** This is the maths only; §7's open question (3D globe
/ flat maps / both) is still open and nothing renders it.

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
- `geo.rs`'s `dark_share`, per §7 — **built, and ours**. `along` was a
  duplicate of our own `great_circle_points`, which is the half of §7 worth
  recording: check whether we already have it before porting it.

## 9. Licence and attribution — not optional

The project is **MIT**; we are **GPL-3.0-or-later**. Compatible, so this is
ordinary work.

**Every file adapted from it carries that project's full notice in the file.**
This is the same rule as the OpenHamClock-derived work already in the tree, and
it is a house rule, not a courtesy. A stage that adapts `source.rs` gets Juan
Toledo Carrasco's copyright in that file.

`ephem.rs` carries the credit in `dark_share`'s own doc comment rather than at
the top of the file, because it credits an **idea** and uses **our own** solar
geometry and our own existing great-circle code — nothing was copied. If a later
stage adapts actual code rather than an idea, the notice goes at the top of the
file as the rule above says.

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

## 12. The LISTEN window — map, station, comment, LOG (operator's idea, 2026-10-13)

**What was asked for, in the operator's words:** *"make a listen panel with a map
on in the right part and station info in the left part where they can immediately
add a comment and hit log."*

**And the correction worth making first, because the two are easy to confuse:**
`dark_share` is **not** the grey line. The grey line is a *picture* — where night
is, everywhere, at this instant, one texture over the whole map. `dark_share` is a
*number* about **one path**: the fraction of the route from home to that station
that is in darkness now. The grey line is its input.

### What already exists, so nothing is duplicated

Checked before designing, per the house rule:

- **SWL LOG** (`swl_log.rs`, app-id `swl-log`, 720×520) — the reception log: list,
  filters, QSL loop, reports, exports, and an entry form that **already has the
  comment** (`SwlEntry::notes`) and pre-fills station/language/site from the
  schedule at the dial.
- **SCHEDULE** (`schedule.rs`) — the broadcast schedule as a table.
- **Neither has a map.** Nothing anywhere in the tree is map-driven.

So this is a genuine gap, not a re-skin. It is **map-first capture feeding the
same `swl_log` file**, so the QSL/report loop is untouched.

### Decisions the operator settled

1. **A window, not a docked panel.** The docked column caps at ~280 pt and the
   operating panels are built for ~680 pt; a map beside station info wants ~700,
   so docked it squeezes what is below it (the #643 fault). As a window it gets
   the `⇱` chip, remembered geometry, and joins `workspaces` like every other
   tool window.
2. **History is a toggle, off by default.** The operator's reason, and it is the
   right one: *an experienced listening station does not want its history
   cluttering the band*. Off, the map is only what is on now; on, it is also
   everything ever logged. It also makes a far better screenshot.
3. Map default = **schedule stations in the span around the dial**, logged ones
   drawn differently. Signal spots and decodes are deliberately **not** mixed in —
   two populations on one picture.

### How it fits the existing map, with nothing changed in it

`worldmap::show` is shared by three panels, so it is **not touched**. Both station
kinds already have a slot in its signature:

| what | passed as | drawn as |
|---|---|---|
| schedule stations in span | `stations: &[DigiStation]` — `{lat, lon, fade: 1.0, call: Some(name), band: None}` | plain dots, name on hover |
| logged stations (history toggle) | `spots: &[(lat, lon, tint)]` | **tinted diamonds** (`worldmap.rs:785`) |

So "logged looks different" is free — the map already draws spots as diamonds and
stations as dots, and it already returns `Option<(f64, f64)>`, **the clicked
lat/lon**, which is the whole of click-to-select.

**A `MapView` of its own is required.** `SdroxideApp::map_view` is one per radio
and `show` eases toward a target fitted to the stations it was *given*, so a
second map sharing it would pull the first toward its own fit — the two would
fight. So `listen_map_view: MapView` is a separate field.

### The one type change, and why it is cheap

`SwlEntry` has **no coordinates** — only a `site` *string*. `BroadcastStation`
has `lat`/`lon`, so a schedule station arrives pre-placed, but **logging** one
needs the pair added:

- `lat: Option<f64>`, `lon: Option<f64>`, both `#[serde(default)]`.
- `SwlEntry` is local `swl.json` and appears in **no wire message** — checked
  against `crates/sdroxide-proto/src/lib.rs` — so this is **no `PROTO_VERSION`
  bump**. An old log file loads unchanged.

### The chip — and why there isn't one

**LISTEN does not get a chip on the top bar, and that was measured rather than
assumed.** It was built with one, and the width guard the project already has for
exactly this (issue **#211**) refused it:

- The SWL row is **already at the two-row limit** on a 1400 pt window for **AM** —
  the main SWL mode. Every other mode (USB, NFM, …) **already** wants three rows in
  SWL mode, before any of this work.
- A third chip pushed **AM** onto a **third** row.

So the chip was taken out again and **LISTEN opens from the SCHEDULE window**,
which is already in that row and holds the same data. It costs no width and
loses nothing: a **MAP** chip beside the clock opens LISTEN with **whatever is on
the dial already selected** (`broadcast::at_dial`), because that is the station
the operator is listening to and the one they most often want to write down.

`the_swl_row_has_no_room_for_a_third_chip` pins the AM line. It deliberately
asserts **AM alone** and not "two rows for every mode", because widening it to
accept the pre-existing three-row states would hide the thing it measures.

**Which chip the operator would give up for a strip chip is their call**, and it
is worth asking: `SAT` and `ISM` in that row are ham extras a shortwave listener
rarely opens, and freeing one would put LISTEN on the bar where it is obvious.

### The real estate, and why LISTEN still is not on the bar (2026-10-13)

The operator pointed at **chip padding being excessive on every chip**, which was
right and a far bigger lever than any label: a chip added 2 pt a side on top of
`button_padding`, so a desktop chip carried **9 pt a side — 18 pt of air around a
five-letter label**. Removed (`3085a941`), worth 28 pt across the SWL row.
`? HELP` became `?` beside the gear-alone settings chip.

**Then the measurement that explains everything:**

```
SWL top row     446.9 pt     <- this one sizes the box
SWL bottom row  258.6 pt     <- ~190 pt sitting idle
```

`system_rows_w` is `max(top, bottom)`, so **the bottom row's spare room was never
reachable**. That is why the gear and the `?` changed nothing: both landed on the
non-binding row. Each is a real saving on its own chips; the box never saw either.

**Moving PUBLIC SDR down rebalances the rows almost exactly** (356.0 against
349.5 — the closest they have ever been) **and does fit LISTEN on the top row.** It
was built, measured, and then **withdrawn**, for a reason that matters more than
the chip:

> Moving it changes the System box by **2.9 pt** in the ordinary ham screen, and
> that is enough to make **`the_operating_panel_stays_inside_its_column` fail** —
> the #643 guard: *`Sstv at 1920 pt: ink x 8..1923.5938 runs past the window`*.

**A 3 pt change anywhere flips that guard**, which says the SSTV panel's budget is
still width-sensitive: it fits at the widths tested and overflows by 3.6 pt a
couple of points either side. That is the #643 class, and the house rule is
explicit that it is **not** fixed with a clamp at the panel boundary — that hides
the cause and only catches the window case. It wants its own job, in the panel's
own arithmetic.

**And then it was fixed, and the fix was the reason this all worked.**

`module_bare_h` budgeted its content for the frame's **margins** and not its
**stroke**, and a stroke paints *outside* the rect it is given — so **every module
in the program drew 2 pt wider than its allocation**. Six on the strip row, that
is 12 pt, and the plate went past the window edge. `6f9d1295`; the operator
described it as *"the design flaw in this whole program is the fixed widths
instead of adaptive"*, which was closer to the truth than three of my own
attempts at naming it.

**LISTEN is now on the top row**, and it fits, because **PUBLIC SDR moved down**:
the two rows go from 446.9 and 258.6 to **416.6 and 349.5**. The box is `max(top,
bottom)`, so the row that was sitting there with ~190 pt of idle space was the
one worth freeing — and the two earlier trims were both on it, which is why they
freed nothing.

The sequence is worth keeping: three sessions of the same fault, each diagnosed
by looking rather than by measuring, and the thing that fixed it was the
operator noticing **padding** and then **which row binds**.

**And the #643 guard was lying about which panel it was.** Bisecting the failure
to the 2.9 pt move found that `the_operating_panel_stays_inside_its_column`
collects **every painted rect in the frame**, so with the box 3 pt wider it caught
a **top bar** overflow while claiming to be about SSTV. The SSTV panel is fine.

The real fault, on clean `HEAD` with nothing else changed:

```
window 1920   fits
window 1930   ink to 1933.50   3.50 pt past
window 2000   ink to 2003.50   3.50 pt past
window 2560   ink to 2562.09   2.09 pt past
```

**The strip's plate paints past the right edge above 1920 pt** — and *both* width
guards stop at exactly 1920, so it has been invisible. `chrome`'s own recorder
names it: `1996x92 panel, 3.5 pt past the edge (offered 1984.0, pinned 1964.0)` —
pinned 1964 plus the frame's own 10+10 is 1984, and the frame came out 1996, so
something **inside** the pin demands more than `set_min_width`/`set_max_width` can
both give. Left as `the_strip_plate_paints_past_the_window_above_1920`,
**`#[ignore]`d** rather than deleted: it is a real shipped fault and a guard that
stops just short of one is worse than one that reaches it and says so.

**And then the operator found it on screen, in a way the harness never did.**

Their screenshot is **split view** — two radios side by side — and the right
pane's strip is **clipped at the screen edge**: `DEC off | AG…`, `REC` cut to
`RE`. So the fault is not "a wide window", which is what the harness reproduced
and what the test is named after. `desktop_strip` takes its width **pane-local**:

> Pane-local, not viewport: in split view each radio packs its own strip against
> its own pane.

So the same box that overflows a 1930 pt single window also overflows **any pane
narrow enough to need it**, and a split screen makes that reachable with no large
monitor at all. **Two triggers, one cause** — a strip module that draws wider than
the plan gave it — and the harness only ever found the wider-of-the-two doors.

That is worth more than the fix: a sweep that varies the **viewport** while the
code varies the **pane** tests one of two doors.

**The naming is worth knowing about:** LISTEN is now also the band menu's
*Explore the dial* **tab**, which is a different thing in a different place. The
operator's word won, and both stay.

### Sequenced after this, not before

`dark_share` goes **inside** LISTEN, on the selected station's path, as a
readout — not as a thing of its own. The number earns its place beside a station
being decided on; on its own it is a statistic nobody asks for.
