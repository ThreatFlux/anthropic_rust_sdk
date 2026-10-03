# Migrating to the 0.4 API

This source implements the [reviewed gap plan](sdk-gap-implementation-plan.md) and
[design issue #60](https://github.com/ThreatFlux/anthropic_rust_sdk/issues/60).
The changes require a 0.4 release because Rust enum matches, public struct
literals, and several field types change. Cargo and release metadata remain at
the last published version until the Release Please version PR is merged.

## Content, stop reasons, and replay

Use content constructors such as `ContentBlock::text`, `tool_use`, `thinking`,
and `tool_result`. Known variants now retain flattened `extra` fields. Add `..`
to patterns that extract selected fields and a wildcard arm to matches of
evolving enums. Existing `ToolChoice::Auto`, `Any`, and `Tool { name }`
construction remains valid; choices now serialize to the correct tagged JSON.
Use `None` and the options variants for explicit parallel-tool controls.

`StopReason::Unknown(String)` preserves new service reasons verbatim.
`ContentBlock::Unknown(RawContentBlock)` preserves the full JSON object instead
of discarding it. `ContentBlock::raw` rejects malformed objects and recognized
types with invalid schemas. Do not replace a future block with the literal
`{"type":"unknown"}` or reconstruct history from response text.

Replay a complete assistant turn with
`response.to_conversation_message(ReplayUnknownPolicy::Preserve)` or choose
`Reject` when an application cannot handle unfamiliar content. Known replay
shapes and roles are validated. Retain signed thinking, including empty
thinking blocks, and append tool results as a separate user turn. Existing
tools that return JSON should encode it as text for the API rather than use
the legacy arbitrary JSON tool-result shape.

Use `MessageResponse::new(id, model, usage)` instead of an exhaustive literal,
then populate the needed fields. Responses retain diagnostics, container,
context-management, input transformations, and unknown metadata.

## Streaming

`MessageDelta` nullable fields use `FieldUpdate::{Missing, Null, Value}`.
`UsageDelta` uses optional counters: a present zero replaces the previous
total, while an omitted count retains it. Counts are cumulative; never sum
successive usage deltas. Match `StreamEvent::MessageDelta { delta, usage, .. }`
to allow the added event-level metadata.

`collect_message` and `collect_text` require a completed `message_stop`
lifecycle. Truncated streams and invalid completed tool JSON return errors.
Raw event consumers can retain partial output and unknown events themselves.
Use `create_stream_with_limits` to configure frame, tool-input, block and queue
bounds. Dropping the stream cancels its producer. The collector rejects invalid
lifecycles and buffered events following termination, then completes at the
terminal event without requiring the HTTP body to close.

## Files and Skills

`File.purpose` is now `Option<String>` for legacy payloads. Current Files
responses need no purpose. `downloadable: Option<bool>` distinguishes an
omitted value from false, and expiration metadata is retained.

Remove `.purpose(...)` from `FileUploadRequest` and remove the purpose argument
from `upload_from_path(path, callback, options)`. The current upload accepts
the file and optional `expires_in_seconds` (3,600–7,776,000).
Current file lists use `page`, `ids`, `scope_id` and `limit`; purpose filtering
and ID cursors are unsupported. Follow `next_page`, independently of legacy
`has_more` fields. Download requests use the file's `/content` endpoint.

Existing `client.skills()` and `skills_legacy()` retain the dated beta schema.
Use `skills_current()` with `CurrentSkillCreateRequest` and current response
DTOs for `display_name`, object-valued `source`, and `latest_version_id`.
The current client rejects a conflicting legacy Skills beta header while
retaining unrelated caller-selected betas. Version paths use current version
IDs. See [the current resources example](../examples/current_resources.rs).

## Counting and models

Prefer `TokenCountRequest::from_message(&request)?` to manually copying fields.
It projects supported prompt options, including thinking, tool choice, output
configuration, cache controls and profile attribution. Unsupported prompt
features that would prevent an equivalent count return a validation error.
Generation limits and streaming controls are omitted. Beta and workspace
request options stay explicit.

`Model.capabilities` is now `Option<ModelCapabilities>`. Convert a legacy
`Vec<String>` with `.into()`. `as_json()` retains nested capability metadata;
`support(feature)` distinguishes supported, unsupported and unknown values.
The boolean helpers return true only for explicit support.

The local catalog adds Opus/Sonnet 5.5 and Fable/Mythos 5.1. Arbitrary IDs still
work through the HTTP APIs. Exact known-model validation checks forced tools,
thinking modes, sampling and assistant prefill. Sonnet 5.5's
`between_tools_thinking()` works at low/medium/high effort and takes no extra
thinking fields. Model/account access and beta activation are caller choices.
Validated builders require sampling fields to be omitted on the four new IDs,
because the current guides do not specify their numeric defaults. Unvalidated
low-level calls retain explicitly supplied parameters.

## Bounded convenience APIs

`PageStream` fetches on demand. `PaginationLimits` defaults to 100 pages and
10,000 items. Existing `list_all` helpers use finite defaults; choose
`list_all_with_limits` to override them. Limits and invalid cursor progress
return errors instead of a silently truncated collection.

`message_batches().results_stream` yields complete JSONL entries before the
download finishes. `BatchResultsStreamOptions` bounds each row (8 MiB by
default). Malformed rows produce one error with a row number and terminate;
error messages omit the raw row. Buffered text/raw methods remain available.

The optional `tool_runner` module executes only explicitly registered client
tools. It has finite turn/call/time/output budgets, sequential default
execution, explicit bounded concurrency, and cancellation. It preserves the
assistant history and matching tool-result IDs. Streaming mode collects a
complete turn before invoking a callback. Partial results identify started
and completed calls and retain completed output on cancellation or failure;
callbacks are never automatically retried. Tool input decoders and execution
hooks are caller-supplied. Unknown content, compaction, and sanitized callback
errors each have explicit policies.

## OAuth administration and federation

Use `OAuthAdminClient` for service-account, federation-issuer, federation-rule,
and workspace-membership APIs. It sends only bearer authentication and rejects
API/admin-key overrides. `from_env` reads `ANTHROPIC_AUTH_TOKEN` and rejects
conflicting credential selections. Static tokens never pretend to refresh;
`TokenProvider` credentials are cached and refreshed under a shared lock.
Read-only requests can refresh once after a 401; mutations are sent once.

`FederationTokenProvider` exchanges an explicitly supplied JWT or rotating
token file at `/v1/oauth/token`. The file is reread on each exchange. Specify
organization, service-account and rule IDs, plus a workspace when required.
The provider does not run a login command, query cloud metadata, or create
trust rules. Administration needs an existing `org:admin` rule created in the
Console. Workload tokens cannot create or promote an admin-role service
account. Broader-scope rule changes and issuer restrictions are enforced by
the service; API callers can modify only developer/inference workspace rules.

## Release and validation

The compatibility fixes and additions are delivered together as an atomic
0.4 migration; no 0.3.1 backport or release was published. The implementation
commit specifies `Release-As: 0.4.0`, and Release Please's pre-1.0 settings
treat breaking changes as minor bumps. The competing auto-release helper is
manual and creates a release PR, so it cannot independently tag a partial
migration after CI completes. Preserve the release footer when squash merging.
See [Release Please's version override](https://github.com/googleapis/release-please#how-do-i-change-the-version-number).

Wire contracts and model rules were checked against the official Python SDK
at `18f25547f20cf5f01da69ac611e700e3bc9ebf21` and current documentation on
2026-10-03. Offline fixtures, mock HTTP, controlled chunk delivery, and callback
tests establish the implemented behavior. Live eligibility, billing, OAuth
permissions and beta availability require separate opt-in smoke tests with
dedicated credentials; offline validation does not establish live-service parity.

Primary references: [streaming](https://platform.claude.com/docs/en/build-with-claude/streaming),
[Opus migration](https://platform.claude.com/docs/en/models/opus-5-5/migration-guide),
[Sonnet migration](https://platform.claude.com/docs/en/models/sonnet-5-5/migration-guide),
[Fable/Mythos migration](https://platform.claude.com/docs/en/models/fable-5-1/migration-guide),
[WIF administration](https://platform.claude.com/docs/en/manage-claude/wif-admin-api),
and [WIF exchange](https://platform.claude.com/docs/en/manage-claude/wif-reference).
