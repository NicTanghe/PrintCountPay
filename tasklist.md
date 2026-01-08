Ricoh PrintCount

SNMP-based Ricoh printer counter monitor (Iced GUI, debug-first)

Epic A — Repo, observability & foundations (NON-NEGOTIABLE)
A1. Workspace + CI

Depends: —
Status: todo

Cargo workspace:

core/ - SNMP, discovery, counters, persistence

ui/ - Iced UI components/state

app/ - App entrypoint (wires core + ui)

docs/ - Usage and architecture notes

CI:

cargo fmt

cargo clippy

cargo test

rust-toolchain.toml (stable)

Acceptance

CI green

No formatting or clippy noise

A2. Logging & tracing infrastructure (EARLY)

Depends: A1
Status: todo

tracing + tracing-subscriber

Log targets:

discovery

snmp

polling

ui

storage

Runtime log-level control (UI)

Acceptance

Logs visible at startup

Log level adjustable live

No println!

A3. Error model + diagnostics

Depends: A2
Status: todo

core::Error with:

SNMP auth/timeout

Unsupported Ricoh model

Missing counters

Counter reset

Discovery failure

RON load/save errors

User summary + technical detail per error

Acceptance

Errors never crash app

Storage errors clearly reported

A4. In-app Console & Debug panel

Depends: A2, A3
Status: todo

Console:

live logs

severity coloring

target filters

Debug panel:

per-printer errors

SNMP OIDs used

persistence diagnostics

copy-to-clipboard

Acceptance

SNMP + persistence issues visible in UI

Epic B — Core data model
B1. Printer & counter structures

Depends: A3
Status: todo

PrinterId (stable)

PrinterRecord:

IP / hostname

model

sysObjectID

SNMP address (IP:port)

community string (optional override)

status + last_seen

CounterSnapshot:

BW / Color / Total

timestamp

source OIDs

Acceptance

Model serializable with RON

Partial data supported safely

Epic C — SNMP layer (instrumented)
C1. SNMP client abstraction

Depends: A2
Status: done

SNMP v2c

Configurable:

community

timeout

retries

Debug/Trace logging for every SNMP call

Acceptance

SNMP behavior fully observable

Timeouts never block UI

C2. Ricoh identification & capability probing

Depends: C1, B1
Status: done

Identify Ricoh via sysObjectID / sysDescr

Detect counter availability

Record counter strategy per printer

Acceptance

Unmapped models clearly marked

C3. Counter resolution logic

Depends: C2
Status: done

Prefer BW + Color

Fallback to Total

Preserve raw OID data

Acceptance

No silent fallbacks

Limitations visible in UI

Epic D — Network discovery
D1. Network scan engine

Depends: C1
Status: done

Auto subnet detection

Manual CIDR override

Async + cancelable

Acceptance

Incremental results

Cancel stops activity immediately

D2. Discovery UI

Depends: D1, A4
Status: done

Subnet list

Community string

Start/Stop

Progress indicators

Acceptance

Discovery errors visible live

Epic E — Polling engine
E1. Poll scheduler

Depends: C3, B1
Status: todo

Global interval

Concurrency limit

Per-printer enable/disable

Manual poll

Acceptance

No UI stalls

Limits respected

E2. Delta & reset handling

Depends: E1
Status: todo

Delta computation

Reset detection

Acceptance

No negative deltas

Reset warnings visible

Epic F — Iced application shell
F1. Main layout

Depends: B1
Status: todo

Top bar:

Discover

Poll toggle

Load / Save

Export

Settings

Main table:

Status

IP

Model

BW / Color

Side panel:

Details

Errors

Raw SNMP data

Acceptance

App usable without debug panel

F2. Error visibility in UI

Depends: A4, F1
Status: todo

Error badges

Tooltips / panels

Diagnostics copy

Acceptance

User always knows why data is missing

Epic G — Persistence & configuration (RON-based)
G1. RON schema definition

Depends: B1
Status: todo

Define PrintersConfig.ron:

list of printers

PrinterId

SNMP address

community (optional)

user label (optional)

polling enabled flag

global settings:

default poll interval

log level

last used discovery ranges

Acceptance

Schema is versioned

Backward-compatible changes possible

G2. Save printers to RON

Depends: G1, F1
Status: todo

“Save configuration” action

Writes current printers + settings to RON

Pretty-printed + stable ordering

Acceptance

Saved file reloads identically

Errors shown clearly on write failure

G3. Load printers from RON

Depends: G1, F1
Status: todo

“Load configuration” action

Merge or replace mode (explicit choice)

Validate printers on load

Acceptance

Invalid entries reported, not fatal

Valid printers appear immediately

G4. Default config path + auto-load

Depends: G2, G3
Status: todo

Configurable default path:

stored in app settings

On startup:

if default path exists → auto-load

else → start empty

Clear UI indicator when auto-loaded

Acceptance

App restores printers automatically

Missing file handled gracefully with warning

Epic H - Export (removed)
H1. CSV export

Depends: E2
Status: dropped (RON only)

Constraints (explicit)

❌ No CLI

❌ No headless mode

✅ RON is the persistence format

✅ Debugging fully in-app

✅ SNMP behavior inspectable

✅ Errors explain themselves

Suggested implementation order (fastest to “feels real”)
A1 + A2 + A3 + A4
B1
C1
G1 + G3 + G4      (load printers early)
F1
D1 + D2          (printers appear from network or RON)
C2 + C3
E1 + E2          (live counters)
F2               (error visibility polish)
G2               (saving configs)
Polish: Ricoh model mappings, UX, performance tuning

