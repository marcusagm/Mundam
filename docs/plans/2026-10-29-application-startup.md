# Application startup and splash screen

The native host presents `splashscreen.html` while the main window remains hidden.
The splash has an independent Vite entry and imports only Solid, startup IPC,
the existing SVG brand assets, and design tokens. No artificial minimum duration
or cross-window transition is used.

## Responsibilities

- `feature/startup.rs` retains monotonic milestones, readiness and terminal errors.
- `bootstrap/startup.rs` composes backend initialization after Tauri setup returns.
- `delivery/tauri/startup.rs` adapts queries and main-window acknowledgements.
- `src/core/startup/` reconciles state, waits for backend readiness and supervises frontend boot.
- `src/mountApplication.tsx` initializes appearance, formats and system data before rendering.
- `src/components/features/startup/` presents progress, delays and failures.

Startup is application lifecycle state, independent of library domain events.
Backend dependencies are registered in order: filesystem/configuration/events/formats,
database, media/workers, and library/watchers. Frontend library IPC starts only after
backend readiness. Indexing, thumbnails and statistics remain background work.
The initial asset query is awaited when configured locations exist. An empty
library and no configured locations are valid initial states.

## State and IPC

`get_startup_state` returns a retained snapshot, for example:

```json
{
  "revision": 6,
  "stage": "content",
  "backendReady": true,
  "failed": false,
  "errorCode": null
}
```

`startup:changed` carries the same contract. Clients subscribe before querying,
ignore older revisions and query again every second to recover missed events.
Equal revisions still confirm IPC contact. A missing response is not inferred
from a frozen progress percentage. The indicator is deliberately indeterminate.

The main window reports `appearance`, `content` and `interface`, and calls
`complete_startup` after mounting and loading fonts. The desktop adapter validates
readiness, shows/focuses main, records `ready`, and closes the splash. Startup
commands do not expose library queries to the splash; mutating acknowledgements
also validate the calling window. The design-system window bypasses the main
presentation handshake.

## Failure and shutdown

Filesystem/database failures and supervised backend task panics retain a safe
`BACKEND_STARTUP_FAILED` code. Frontend import, initialization and mount failures
retain `FRONTEND_STARTUP_FAILED`. Detailed backend errors stay in tracing.
Startup metadata loaders opt into error propagation instead of quietly rendering
an empty library. Frontend readiness is never set in a failure `finally` block.

After eight seconds the splash adds elapsed time. After five seconds without
successful IPC contact it reports that it is waiting for a response and pauses
the animated indicator. Neither threshold changes readiness or claims failure.
Users can quit at any time. Closing splash before readiness or closing main runs
graceful shutdown; closing splash after readiness only closes that window.
Other auxiliary windows close independently.

Automatic retry is intentionally absent: initialization currently registers
long-lived services, so restarting a partial boot in place is not an idempotent
operation. Quit and reopen is the supported recovery path.

The splash follows the operating system's light/dark preference and reduced-motion
setting; the main window applies the user's persisted appearance before opening.
Window composition and cold/warm boot behavior should be verified on each target
desktop platform before release.

## Validation

```sh
npm run typecheck
npm run lint
npm run build
npm run test -- src/core/startup/client.test.ts
cargo test --manifest-path src-tauri/Cargo.toml feature::startup::tests
```

Automated checks cover event/query ordering, missed readiness events, delayed
listener cleanup, failure propagation, and rejecting premature presentation.
Desktop checks should include an empty library, an existing library, database
failure, slow startup, quitting during startup, and closing auxiliary windows.
