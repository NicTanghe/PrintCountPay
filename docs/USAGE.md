Build and run

1. cargo check --workspace
2. cargo run -p printcountpay-app

Tabs

- Printers: left list of discovered (currently demo) printers, right details with Polling and SNMP OIDs sub-tabs.
- Counters segment shows B/W, color, and total clicks (N/A until counter OIDs are mapped).
- SNMP OIDs sub-tab lets you load/save counter OIDs in RON, manually edit dotted OIDs (comma/space separated), or crawl prtMarkerLifeCount.
- Debug: log console, filters, and diagnostics panel.

Logging controls

- Use the Log level picker to change verbosity at runtime.
- Toggle targets to filter the console view.
- Use "Add mock SNMP entry" to inject a mock SNMP log line for diagnostics.
- Use "Copy diagnostics" to copy recent logs and state to the clipboard.
