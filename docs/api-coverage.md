# API coverage and maturity

> Source snapshot: 2026-08-02, based on the repository's `main` branch.

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
| Messages | **Supported** | Create messages, consume typed SSE streams, count tokens, and use typed content/tool models | Streaming has no automatic reconnect, resume, or retry; new event variants and optional fields can require a crate update | [client](../src/api/messages.rs) · [models](../src/models/message.rs) · [streaming](../src/streaming/message_stream.rs) |
| Models | **Supported** | List and retrieve models, paginate all results, filter by locally modeled capability, and check existence | Local capability helpers are SDK metadata and can lag the service catalog | [client](../src/api/models.rs) · [models](../src/models/model.rs) |
| Message batches | **Supported** | Create, retrieve, list, cancel, delete, fetch raw or parsed results, filter locally, and poll for completion | Polling is client-side and the caller chooses both interval and total timeout | [client](../src/api/message_batches.rs) · [models](../src/models/batch.rs) |
| Files | **Supported** | Upload bytes or paths, list, retrieve metadata, download bytes or paths, delete, and filter by purpose | Large-file memory and disk behavior depends on the helper selected; preview headers can still be required by the service | [client](../src/api/files.rs) · [models](../src/models/file.rs) |
| Skills and versions | **Preview** | List, retrieve, create, update, and delete skills; list, retrieve, create, and delete versions; build uploads from a directory | This surface uses beta headers and service availability can be account-specific | [client](../src/api/skills.rs) · [models](../src/models/skill.rs) |
| Text completions | **Legacy** | Submit a text-completion request and deserialize the response | No streaming or broader lifecycle operations; new integrations should normally begin with Messages | [client](../src/api/completions.rs) · [models](../src/models/completion.rs) |

## Administration

Administration requires a separate `ANTHROPIC_ADMIN_KEY`.

| Surface | Status | Implemented in this SDK | Important limits | Source |
| --- | --- | --- | --- | --- |
| Organization | **Supported** | Retrieve the organization; list, retrieve, update, and delete users; manage invites; manage members | Admin credentials and organization permissions are enforced by the service | [client](../src/api/admin/organization.rs) |
| Workspaces | **Supported** | List, retrieve, create, update, delete, archive, and restore workspaces; manage workspace members | New administration fields can require a crate update | [client](../src/api/admin/workspace.rs) |
| API keys | **Partial** | List, retrieve, update, paginate all, and filter keys | Create, rotate, and delete helpers deliberately return `InvalidInput` because those operations are not implemented against a public endpoint | [client](../src/api/admin/api_keys.rs) |
| Usage | **Partial** | Message usage/cost reports, Claude Code usage reports, scoped usage queries, summaries, history, and top-key helpers | Convenience aggregations are SDK behavior; compare billing-sensitive results with the Anthropic Console | [client](../src/api/admin/usage.rs) |

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
- Unknown optional response fields are generally ignored by Serde. A new enum
  variant or changed field type can still cause deserialization to fail.
- Non-streaming retry behavior applies below each resource client. Read
  [configuration and operations](configuration.md#retries-and-idempotency)
  before relying on it for create or mutation calls.
- A module's presence does not prove that every operation is exercised against
  the live service. The default test suite uses unit and mock-server coverage;
  live tests are opt-in.

## Reporting a mismatch

When the live API and this page disagree, please open an issue containing:

1. The official Anthropic reference URL and the date checked.
2. The endpoint, field, event, or behavior that differs.
3. The crate version or Git commit used.
4. A minimal reproduction with credentials and customer data removed.

Never include API keys, admin keys, authorization headers, uploaded customer
files, or unredacted production prompts and responses.
