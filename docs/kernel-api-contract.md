# Kernel API contract

The external-controller surface of the RClash kernel, frozen at upstream
`v0.29.0` (`de17693`) minus the embedded panel.

**The rule:** the kernel owns the data plane and this HTTP/WS control API. It
serves no UI, no static files and no management logic. Management UX — views,
config workflows, process supervision — belongs to a separate application
(RClash's NSH manager), and anything a remote client cannot do itself is the
only argument for growing this surface.

`crates/meow-api/tests/api_surface_test.rs` enforces this document: it parses
every `.route("…")` literal out of `src/routes.rs` and fails when the set
differs from the table below, or when a UI module, a static-file service or
`external_ui` state reappears. Change the routing table and this document in
the same commit.

## Auth

- `Authorization: Bearer <secret>` on every route.
- WebSocket upgrades additionally accept `?token=<secret>` (browsers cannot set
  headers on a WebSocket).
- An empty or absent `secret` disables the check, matching mihomo.
- CORS is permissive (`Access-Control-Allow-Origin: *`). A deployment that
  serves the manager from the same origin as the API (reverse proxy) does not
  depend on it.
- `meow` publishes the bound address, secret and config path to a file only its
  own user can read (`meow_common::controller_file`), so a local client needs
  no configuration. That file is a public interface — it is not part of the
  removed CLI.

## Routes

57 `METHOD /path` pairs over 42 route entries. `GET` on `/logs`, `/traffic`,
`/memory` and `/connections` upgrades to a WebSocket when the request carries
`Upgrade: websocket`; without the header it streams newline-delimited JSON.

| Method | Path | Handler | Notes |
|---|---|---|---|
| `GET` | `/` | `hello` | `{"hello":"meow"}` |
| `GET` | `/version` | `version` | `{version, meta}` |
| `GET` | `/proxies` | `get_proxies` | all proxies and groups |
| `GET` | `/proxies/{name}` | `get_proxy` | one proxy/group, or a provider node |
| `PUT` | `/proxies/{name}` | `update_proxy` | select a group member |
| `DELETE` | `/proxies/{name}` | `unfix_proxy` | drop a manual selection |
| `GET` | `/proxies/{name}/delay` | `get_proxy_delay` | single-node probe |
| `GET` | `/proxies/{name}/chain-health` | `get_chain_health` | `dialer-proxy` chain probe |
| `GET` | `/group` | `get_groups` | groups only |
| `GET` | `/group/{name}` | `get_group` | one group |
| `GET` | `/group/{name}/delay` | `get_group_delay` | probe every member |
| `GET` | `/rules` | `get_rules` | ordered rule list |
| `POST` | `/rules` | `replace_rules` | whole-table replace |
| `PUT` | `/rules` | `update_rule_at_index` | replace one entry |
| `DELETE` | `/rules/{index}` | `delete_rule` | by position |
| `POST` | `/rules/reorder` | `reorder_rules` | move entries |
| `GET` | `/connections` | `get_connections` | live connections (WS push) |
| `GET` | `/connections/{id}` | `get_connection` | one connection |
| `DELETE` | `/connections/{id}` | `close_connection` | close one |
| `DELETE` | `/connections` | `close_all_connections` | close all |
| `GET` | `/configs` | `get_configs` | runtime config view |
| `PATCH` | `/configs` | `update_configs` | validate + apply fields (mode, log-level, tun…) |
| `PUT` | `/configs` | `put_configs` | replace from a file path or base64 YAML, then reload |
| `GET` | `/metrics` | `get_metrics` | Prometheus text |
| `GET` | `/traffic` | `get_traffic` | up/down samples (WS push) |
| `GET` | `/traffic/ledger` | `get_ledger` | ADR-0015 ledger report |
| `DELETE` | `/traffic/ledger` | `clear_ledger` | clear the ledger |
| `GET` | `/statistics` | `get_statistics` | engine statistics |
| `GET` | `/logs` | `get_logs` | log stream (`?level=`) |
| `GET` | `/logbook` | `get_logbook` | persistent events (ADR-0014) |
| `DELETE` | `/logbook` | `clear_logbook` | clear the logbook |
| `GET` | `/memory` | `get_memory` | heap samples (WS push) |
| `GET` | `/dns/results` | `get_dns_results` | recent resolutions |
| `GET` | `/dns/query` | `dns_query_get` | query via query string |
| `POST` | `/dns/query` | `dns_query` | query via body |
| `POST` | `/cache/dns/flush` | `flush_dns_cache` | drop the DNS cache |
| `POST` | `/cache/fakeip/flush` | `flush_fakeip_cache` | drop the fake-IP cache |
| `POST` | `/api/config/save` | `save_config` | **no body**: persist the in-memory config to disk |
| `GET` | `/api/subscriptions` | `get_subscriptions` | subscription list |
| `POST` | `/api/subscriptions` | `add_subscription` | add one |
| `DELETE` | `/api/subscriptions/{name}` | `delete_subscription` | remove one |
| `POST` | `/api/subscriptions/{name}/refresh` | `refresh_subscription` | fetch now |
| `GET` | `/api/proxy-groups` | `get_proxy_groups` | declared groups |
| `POST` | `/api/proxy-groups` | `create_proxy_group` | create |
| `PUT` | `/api/proxy-groups/{name}` | `update_proxy_group` | update |
| `DELETE` | `/api/proxy-groups/{name}` | `delete_proxy_group` | delete |
| `PUT` | `/api/proxy-groups/{name}/select` | `select_proxy_in_group` | choose a member |
| `GET` | `/providers/proxies` | `get_providers` | proxy providers |
| `GET` | `/providers/proxies/{name}` | `get_provider` | one proxy provider |
| `PUT` | `/providers/proxies/{name}` | `refresh_provider` | re-fetch |
| `GET` | `/providers/proxies/{name}/healthcheck` | `provider_healthcheck` | health-check all nodes |
| `GET` | `/providers/proxies/{provider_name}/{proxy_name}` | `get_provider_proxy` | one provider node |
| `GET` | `/providers/proxies/{provider_name}/{proxy_name}/healthcheck` | `provider_proxy_healthcheck` | probe one provider node |
| `GET` | `/providers/rules` | `get_rule_providers` | rule providers |
| `GET` | `/providers/rules/{name}` | `get_rule_provider` | one rule provider |
| `PUT` | `/providers/rules/{name}` | `refresh_rule_provider` | re-fetch |
| `GET` | `/listeners` | `get_listeners` | named listeners (read-only) |

## Must never come back

Removed on purpose; the contract test fails if any of these reappears:

| Removed | Why |
|---|---|
| `GET /ui`, `GET /ui/{*rest}` | The kernel hosts no UI. |
| `crates/meow-api/static/` | Panel assets: no static files are served or embedded. |
| `external_ui` state, `ServeDir`, `nest_service` | No third-party dashboard hosting. |
| `--ext-ui` flag, `external-ui-download` feature | No UI plumbing in the binary. |
| `meow-cli` (crate, binary, agent skill) | Management clients are separate programs. |

A config that still sets `external-ui`, `external-ui-name` or `external-ui-url`
keeps loading: `RawConfig` has no `deny_unknown_fields`, so the keys are
accepted and ignored (`crates/meow-config/tests/external_ui_tolerated_test.rs`).

## Client notes

- Config edits are two steps: change the in-memory config (`PATCH /configs`,
  `PUT /rules`, `/api/subscriptions`, `/api/proxy-groups`), then
  `POST /api/config/save` to persist. A client should show "applied but not
  saved" as a distinct state.
- `PUT /configs` accepts `{"path": "…"}` or `{"payload": "<base64 YAML>"}` and
  reloads; it is the whole-config import path.
- There is no endpoint that returns the raw YAML; reading the file is the
  operator's job (the discovery file carries its path).
