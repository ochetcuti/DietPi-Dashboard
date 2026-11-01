# Dashboard redevelopment plan

## Existing architecture

The Rust frontend renders Maud HTML and serves embedded, Brotli-compressed assets. Browser interactions use Nomini and xterm.js. A separate Rust backend connects to the frontend over an encrypted bitcode protocol; a shared key and protocol version gate the connection. Getters collect metrics using sysinfo and DietPi commands. The current frontend caches several requests for 1.5 seconds.

## Inventory and findings

- Overview: CPU, temperature, RAM, swap, configured mounts and network graphs refresh through HTML fragments. Graph history is carried in client query strings; network readings are byte deltas, not rates, and the axis is logarithmic. Readings lack timestamps and missing-data handling.
- Processes: PID, name, state, CPU and memory are available, with sort and four signal actions. Rows refresh wholesale and signals have no acknowledgement; no search, user, runtime or command details are shown.
- Software: DietPi-Software machine-readable output is split into installed and available. The parser collapses every nonzero state into installed and command failures can appear as empty lists. Actions run a DietPi command but discard exit status and stderr.
- Services: DietPi-Services output is parsed for status and logs, with no actions or reliable systemd inventory.
- Management: host metadata and raw frontend/backend configuration are displayed, potentially exposing secrets.
- Terminal: xterm.js connects to one shared, long-lived backend PTY via a streamed GET and POST input/resize. All clients on a node share the session. There is no isolated lifecycle or meaningful connection state. Multiple clients can see and send the same session output/input.
- File browser: backend file operations exist, but errors are frequently discarded. The browser uses paths and raw data through a command channel.
- Security: login is optional and token-based. Administrative operations depend on that global switch and backend permissions; action messages commonly lack confirmation or an operation result. The current design needs explicit CSRF protection, session isolation, safe authorization boundaries, and no credentials rendered in pages.
- Deployment: frontend and backend are separate executables. Wire-format changes require matched versions and a coordinated rollout and rollback.

## Direction

Build a dense operations workspace with restrained neutral surfaces, clear status language, keyboard-accessible controls, light/dark themes and narrow-screen reflow. Continue using server-rendered Rust and small native browser components unless a dependency has a specific measured benefit. Do not display a metric when its source or units cannot be established.

## Acceptance checklist

- [ ] Overview uses timestamped real measurements, correct units, missing-data gaps and bounded history.
- [ ] Processes have usable filtering, stable updates, accurate fields, and confirmed, acknowledged actions.
- [ ] DietPi-managed software is distinguished from packages, services and actual update availability; failures are visible.
- [ ] Management exposes safe, correctly authorized services, storage, networking, updates and logs without disclosing credentials.
- [ ] Terminal uses isolated authenticated PTY sessions with lifecycle, resize, connection feedback and interactive tests.
- [ ] All screens have responsive, accessible states and have been checked with real backend data.
- [ ] Protocol/security changes are tested and rolled out to matching backend and frontend binaries.
- [ ] Run asset build, fmt, clippy, unit/integration tests, ARM64 CI and browser checks before deployment.

Items remain unchecked until implemented and verified. This list records the full redevelopment scope; a successful build alone is not completion.

## Implemented in this iteration

- Added process name/PID filtering, refresh pause, labelled controls and confirmation for termination. Process signal results remain unacknowledged and process rows are still replaced during refresh.
- Added DietPi-Software filtering, explicit inventory scope, destructive-operation confirmation, a warning for an empty/unavailable inventory, and meaningful command failure output. The backend accepts only DietPi install/uninstall commands with numeric IDs and no longer treats queued state as installed.
- Replaced raw configuration contents with non-secret security settings. Login now works without a connected backend. Browser file paths are no longer interpolated into executable JavaScript.
- Added same-site session cookies, origin checks on browser POSTs, and required configured authentication for terminal, file browser, management and modifying operations. Existing read-only monitoring remains available when login is disabled.
- Added terminal connection/error feedback, ordered client writes and browser resource cleanup. The PTY remains shared; it is not isolated and should not be treated as a completed terminal redesign.
- Labelled client-side chart history and untimed network byte deltas honestly. Zero network readings no longer generate invalid SVG coordinates. These are not timestamped throughput charts.

## Remaining before complete redevelopment

The acceptance checklist above is deliberately not marked complete. Needed work includes isolated authenticated PTY lifecycles and integration tests, server-side timestamped bounded telemetry and throughput, process identity/runtime/details and acknowledged actions, package and update inventories, safe systemd management, networking/storage/logs/system actions, cross-device authenticated browser validation, and a complete UX redesign. Current software and process data still have backend error-reporting limitations. Software state parsing was checked against the local DietPi machine-readable command; only states 0 and 2 are classified.
