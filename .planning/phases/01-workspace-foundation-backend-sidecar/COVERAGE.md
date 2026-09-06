# Phase 1 — API/SDK Integration Coverage

No external API integration: Phase 1 builds a local-only Rust GPUI workspace that spawns the
repo's own Go sidecar (`tmux-gui-server`) as a child process on loopback only — the
`BACKEND_PORT:<n>` stdout handshake and `GET /api/health` readiness probe are a local
process/loopback contract against unchanged in-repo code (`be/cmd/server/main.go`,
`be/internal/server/router.go:27`), not an external SaaS/API/SDK integration. No network calls
beyond 127.0.0.1, no API keys, no third-party remote services.

*(Planner declaration per `/gsd-plan-phase` coverage gate, 2026-09-06.)*
