# Anthropic API Support Diff and Upgrade Roadmap

Date: 2026-02-23
Repository: `threatflux`

## Summary
This document compares the crate's current API support to the latest Anthropic API docs and defines a concrete implementation roadmap.

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
