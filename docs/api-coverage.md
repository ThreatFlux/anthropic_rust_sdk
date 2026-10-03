# API coverage and maturity

> Source snapshot: 2026-10-03, including the unreleased 0.4 migration.

This page records the high-level clients and operations visible in this source
tree. It does not certify live-service parity, account eligibility, or support
for every optional request and response field. Anthropic can change endpoints,
schemas, model availability, and beta requirements independently of this crate.
Use the [official API documentation](https://platform.claude.com/docs/en/api/overview) as the
service authority.

## Status definitions

- **Supported** means the repository implements the core resource operations
  shown in the table and exercises them with unit or mock-server tests.
- **Partial** means useful operations or types exist, but the surface has a
  notable transport, operation, or maturity limitation.
- **Preview** means the source targets a beta or research-preview surface whose
  availability and schema can change independently of a stable crate release.
- **Legacy** means the client exists for compatibility with an older API and is
  not the recommended starting point for new integrations.
- **Types only** means models exist without a dedicated high-level endpoint
  client.

## Core resources

| Surface | Status | Implemented in this SDK | Important limits | Source |
| --- | --- | --- | --- | --- |
| Messages | **Supported** | Create messages, consume typed SSE streams, count tokens, and use typed content/tool models | Strict terminal collection, lossless unknown payloads and presence-aware usage; streaming has no automatic reconnect or retry | [client](../src/api/messages.rs) · [models](../src/models/message.rs) · [streaming](../src/streaming/message_stream.rs) |
| Models | **Supported** | List/retrieve models, bounded lazy pagination, full capability metadata, conservative capability queries, and existence checks | Local capability helpers are SDK metadata and can lag the service catalog | [client](../src/api/models.rs) · [models](../src/models/model.rs) |
| Message batches | **Supported** | Create/retrieve/list/cancel/delete, bounded incremental JSONL results, buffered raw/text results, and completion polling | Polling is client-side and the caller chooses both interval and total timeout | [client](../src/api/message_batches.rs) · [models](../src/models/batch.rs) |
| Files | **Supported** | Upload bytes/paths with expiration, token-paged lists and ID lookup, metadata, content download, and deletion | Large-file memory and disk behavior depends on the helper selected; preview headers can still be required by the service | [client](../src/api/files.rs) · [models](../src/models/file.rs) |
| Skills and versions | **Supported** | Separate current and legacy schema clients, skill/version lifecycle, bounded pagination, directory uploads | Legacy client keeps the dated beta header; current client rejects conflicting legacy header selection | [client](../src/api/skills.rs) · [models](../src/models/skill.rs) |
| Optional tool runner | **Supported** | Explicit callbacks/decoders, finite budgets, bounded concurrency, cancellation, complete-turn streaming adapter and partial transcripts | Callback side effects are caller-controlled; unknown replay and compaction need explicit policies | [runner](../src/tool_runner.rs) |
| Text completions | **Legacy** | Submit a text-completion request and deserialize the response | No streaming or broader lifecycle operations; new integrations should normally begin with Messages | [client](../src/api/completions.rs) · [models](../src/models/completion.rs) |

## Administration

Existing key-based administration requires `ANTHROPIC_ADMIN_KEY`.

| Surface | Status | Implemented in this SDK | Important limits | Source |
| --- | --- | --- | --- | --- |
| Organization | **Supported** | Retrieve the organization; list, retrieve, update, and delete users; manage invites; manage members | Admin credentials and organization permissions are enforced by the service | [client](../src/api/admin/organization.rs) |
| Workspaces | **Supported** | List, retrieve, create, update, delete, archive, and restore workspaces; manage workspace members | New administration fields can require a crate update | [client](../src/api/admin/workspace.rs) |
| API keys | **Partial** | List, retrieve, update, paginate all, and filter keys | Create, rotate, and delete helpers deliberately return `InvalidInput` because those operations are not implemented against a public endpoint | [client](../src/api/admin/api_keys.rs) |
| Usage | **Partial** | Message usage/cost reports, Claude Code usage reports, scoped usage queries, summaries, history, and top-key helpers | Convenience aggregations are SDK behavior; compare billing-sensitive results with the Anthropic Console | [client](../src/api/admin/usage.rs) |

The separate `OAuthAdminClient` uses bearer credentials for the following
OAuth-only resources; these endpoints reject Admin API keys.

| Surface | Status | Implemented in this SDK | Important limits | Source |
| --- | --- | --- | --- | --- |
| OAuth service accounts | **Supported** | Create/get/list/update/archive, account and workspace membership operations, bounded token pagination | Admin-role creation/promotion requires an interactive user credential | [client and DTOs](../src/oauth/resources.rs) |
| OAuth federation issuers and rules | **Supported** | Create/get/list/update/archive, rule workspace bindings, typed JWKS configurations | API callers can modify only workspace developer/inference rules; broader-scope bootstrap uses the Console | [client and DTOs](../src/oauth/resources.rs) |
| Federation token provider | **Supported** | Explicit JWT-bearer exchange, rotating token-file reread, expiry-aware shared OAuth client refresh | Requires an existing trusted rule and eligible service account/workspace; no interactive login or implicit metadata discovery | [provider](../src/oauth/wif.rs) · [transport](../src/oauth/mod.rs) |

## Beta and research-preview resources

These clients are intentionally separated from core coverage. Before use,
confirm that the endpoint is available to the target account and review the
request method for required `RequestOptions`.

| Surface | Status | Implemented in this SDK | Source |
| --- | --- | --- | --- |
| Dreams | **Preview** | Create, list, retrieve, archive, and cancel | [client](../src/api/dreams.rs) · [models](../src/models/dream.rs) |
| MCP Tunnels | **Preview** | Tunnel create/retrieve/list/archive, token reveal/rotation, and certificate lifecycle | [client](../src/api/tunnels.rs) · [models](../src/models/tunnel.rs) |
| User Profiles | **Preview** | Create, retrieve, update, list, and create enrollment URLs | [client](../src/api/user_profiles.rs) · [models](../src/models/user_profile.rs) |
| Managed Agents | **Preview** | Agents, environments, sessions, events/streams, resources, threads, vaults/credentials, memory stores/memories, and deployments/runs | [clients](../src/api/managed_agents) · [models](../src/models/managed_agents) |
| Webhook events | **Types only** | Forward-compatible event envelope and payload models | [models](../src/models/webhook.rs) |

## Operational behavior that affects coverage

- The generic `Client::request` and `Client::request_admin` methods are public
  escape hatches, but their existence does not make an unmodeled endpoint
  supported.
- Core message/content/usage and current resource DTOs retain unknown payloads
  and metadata. Unsupported delta accumulation fails explicitly while raw events
  remain available. Other older resource types can still ignore optional fields.
- Non-streaming retry behavior applies below each resource client. Read
  [configuration and operations](configuration.md#retries-and-idempotency)
  before relying on it for create or mutation calls.
- A module's presence does not prove that every operation is exercised against
  the live service. The default test suite uses unit and mock-server coverage;
  live tests are opt-in.

See [the 0.4 migration guide](migration-0.4.md) for source compatibility,
release staging, and the offline/live validation boundary.

## Reporting a mismatch

When the live API and this page disagree, please open an issue containing:

1. The official Anthropic reference URL and the date checked.
2. The endpoint, field, event, or behavior that differs.
3. The crate version or Git commit used.
4. A minimal reproduction with credentials and customer data removed.

Never include API keys, admin keys, authorization headers, uploaded customer
files, or unredacted production prompts and responses.
