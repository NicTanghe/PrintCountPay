Workspace layout

- core: domain model, SNMP, discovery, persistence, errors
- ui: Iced UI state, views, logging console/debug panel
- app: binary entrypoint that wires core and ui together

Dependency flow

- ui depends on core
- app depends on core and ui
- core stays free of UI crates
