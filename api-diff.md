# Anthropic API Support Diff and Upgrade Roadmap

Date: 2026-02-23 (currency upgrade: 2026-06-22)
Repository: `threatflux`

## Summary
This document compares the crate's current API support to the latest Anthropic API docs and defines a concrete implementation roadmap.

## Currency Upgrade (2026-06-22)

Brought the request/response surface current with the flagship model generation:

- **Models**: added current catalog (`claude-fable-5`, `claude-opus-4-8/4-7/4-6`,
  `claude-sonnet-4-6`, `claude-haiku-4-5`); default model changed off the retired
  `claude-3-5-haiku`; retired ids marked `#[deprecated]`; `ModelFamily` extended.
- **Thinking**: added adaptive thinking (`type: "adaptive"` + `display`). `budget_tokens`
  (`enabled`) retained for legacy models only — it 400s on Opus 4.7+/Fable 5.
- **Effort / task budgets**: added `xhigh` effort level and `output_config.task_budget`.
- **Prompt caching**: wired `cache_control` onto text/tool/system blocks, added a
  cacheable `SystemPrompt`/`SystemBlock`, top-level `auto_cache`, and 1h TTL.
- **Server-side tools**: `Tool` now models `type` + server-tool constructors
  (web search/fetch, code execution, bash, text editor, memory), plus `strict`.
- **Refusal**: `stop_details` on the response, server-side `fallbacks` param +
  `fallback` content block, and beta-header helpers (server-side fallback, task
  budgets, compaction, mid-conversation system, MCP client).
- **Models API**: `Model` deserializes the real list/retrieve shapes
  (`max_input_tokens`, nested `capabilities`, optional `updated_at`).
- **Message response**: tolerates responses without `created_at`.

### Test suite resurrection (2026-06-23)
- The previously-orphaned `tests/unit/`, `tests/integration/`, and `tests/real_api/`
  directories are now wired into the build via `tests/unit_suite.rs`,
  `tests/integration_suite.rs`, and `tests/real_api_suite.rs`, reconciled with the
  current API, and **passing**. Full suite: **522 tests, 0 failures** (`unit_suite`
  282, `integration_suite` 95, `src` lib 97, doctests 20, plus the pre-existing
  top-level `tests/*.rs`). `cargo clippy --all-targets --all-features -- -D warnings`
  is clean and `cargo fmt --check` passes.

### Known remaining gap
- **Managed Agents** (`/v1/agents`, `/v1/sessions`, `/v1/environments`, vaults,
  memory stores, deployments) — the server-managed agent platform — is **not yet
  implemented**. It is a large, separable beta surface (≈20 endpoints + an SSE
  session event stream) and warrants its own module/PR. A full implementation plan
  is in [`MANAGED_AGENTS_PLAN.md`](MANAGED_AGENTS_PLAN.md).

## Current Coverage Snapshot

| Area | Latest Anthropic API Surface | Current SDK Status | Notes |
|---|---|---|---|
| Messages | `POST /v1/messages` | Partial | Core messaging works; advanced request fields were missing and are now added in P0. |
| Token counting | `POST /v1/messages/count_tokens` | Supported | Implemented. |
| Streaming | SSE message events | Supported | Core and newer delta/detail variants (thinking/signature/input_json/citations + partial usage fields) are implemented. |
| Models | List/get models | Supported | Implemented. |
| Message Batches | Create/retrieve/list/cancel/delete | Supported | Implemented. |
| Message batch results endpoint | `GET /v1/messages/batches/{message_batch_id}/results` | Added in P0 | New endpoint + JSONL parsing helpers added. |
| Files | Upload/list/get/download/delete | Supported | Implemented. |
| Skills | Skills API (`/v1/skills`) | Supported | List/get/create/delete + skill-version endpoints are implemented and now covered by dedicated integration tests. |
| Admin API | Organizations/workspaces/keys/usage/cost reports | Supported | Paths/models aligned to current docs, including Claude Code usage reporting and updated report query semantics. |
| Legacy text completions | `POST /v1/complete` | Supported | Implemented with typed request/response models and compatibility tests. |

## P0 / P1 / P2 Roadmap

## P0 (Immediate: unblock core API parity)
- [x] Add `GET /v1/messages/batches/{message_batch_id}/results` support.
- [x] Add parsed JSONL helpers for batch results.
- [x] Add core Skills API support: list/get/delete.
- [x] Add message request fields:
  - [x] `service_tier`
  - [x] `inference_geo`
  - [x] `output_config`
  - [x] `container`
  - [x] `context_management`
  - [x] `mcp_servers`
- [x] Expand stop reasons to include `pause_turn` and `refusal`.

## P1 (Near-term: feature depth and beta completeness)
- [x] Skills API write flows:
  - [x] create skill (multipart upload)
  - [x] update skill (implemented as version creation alias)
  - [x] skill version endpoints (list/get/create/delete)
- [x] Message content model expansion:
  - [x] document content blocks and citations structures
  - [x] richer image/document source variants (`url`, `file_id`)
  - [x] server tool result content schemas
- [x] Streaming parity for newer event payload variants.
- [x] Strongly typed `output_config` and structured-output helpers.

## P2 (Medium-term: admin and compatibility hardening)
- [x] Align Admin API paths and models with current organization/admin docs.
- [x] Add cost report endpoints and models.
- [x] Add legacy completions endpoint for backward compatibility (`/v1/complete`).
- [x] Add compatibility tests against current Anthropic reference payloads.

### P2 Progress (started 2026-02-23)
- Completed Admin docs-parity alignment for current organization/admin reference:
  - updated Organization/Workspace/API-key typed schemas to match current response shapes
  - switched message cost-report endpoint to `GET /v1/organizations/cost_report`
  - updated usage/cost report pagination/filter params to current `page` + array-query semantics
  - added Claude Code usage report support:
    - `GET /v1/organizations/usage_report/claude_code`
    - typed params and response models
  - added endpoint/query tests for updated report paths and Claude Code usage report
- Added Skills integration coverage for list/get/create/delete and version flows using mocked endpoint parity tests.
- Expanded compatibility tests against current reference payload shapes for:
  - Messages response + streaming delta variants
  - Skills objects/list/version/delete payloads
  - Admin users/invites/workspace-members list/delete payloads
  - Message usage/cost report nested `results` payload forms
  - Legacy completions response payload
- Added initial Admin API path migration to `/organizations/...` with admin-auth request routing.
- Added typed message usage/cost report params + models and endpoints:
  - `GET /v1/organizations/usage_report/messages`
  - `GET /v1/organizations/cost_report`
- Added legacy completions API support:
  - `POST /v1/complete`
  - request/response models and client surface
- Added initial compatibility tests for:
  - message usage report payloads
  - message cost report payloads
  - legacy completion request/response shape
- Added typed Admin parity schemas and endpoints for:
  - organization users (`User`, list/update/delete request and response shapes)
  - organization invites (typed create/list/delete schemas and enums)
  - workspace members (typed list/get/create/update/delete schemas)
- Hard-gated legacy `/organizations/usage*` helper methods and marked them deprecated in favor of:
  - `get_message_usage_report`
  - `get_message_cost_report`
- Added admin auth and pagination-parameter parity tests covering:
  - `x-api-key` admin authentication behavior
  - `after_id` / `before_id` query naming on admin list endpoints

## Execution Order
1. Finish P0 work and stabilize with tests.
2. Deliver P1 Skills write/version support.
3. Expand message content/tooling schemas and streaming parity.
4. Complete Admin/cost-report alignment and legacy compatibility in P2.

## Source References
- API overview: https://platform.claude.com/docs/api/overview
- Messages create: https://docs.claude.com/en/api/messages
- Message batches: https://platform.claude.com/docs/api/messages-batches
- Skills API: https://docs.claude.com/en/api/skills
- Admin API: https://platform.claude.com/docs/api/admin-api
