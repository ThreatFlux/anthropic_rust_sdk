# SDK gap implementation plan

Reviewed **2026-10-03** against [the feature audit](feature-audit-2026-10-03.md), repository API code at
`5207544e352b06d67251600c809e00b82ca11d56`, and Anthropic's Python SDK at `18f25547f20cf5f01da69ac611e700e3bc9ebf21`. This is a
follow-up plan for `main` after [modernization PR #59](https://github.com/ThreatFlux/anthropic_rust_sdk/pull/59)
lands. The implementation is tracked in [design issue #60](https://github.com/ThreatFlux/anthropic_rust_sdk/issues/60)
and [the migration guide](migration-0.4.md). This document records the reviewed design; authenticated validation remains opt-in.

The implementation delivers all slices together as an atomic 0.4 migration, including the bridge fixes and optional additions.
The staged release sequence below records the original proposal; no separate 0.3.1 release was published. The migration guide
describes the implemented behavior and release guard.

## Review decisions

The audit's eight findings are actionable, with the Skills and model-catalog qualifications retained. JSON escape hatches already
support several newer request options; the work is reliable typed support and complete responses.

| Audit item | Reviewed status | Implementation decision | Slice |
| --- | --- | --- | --- |
| 1. Tool choice | Confirmed wrong wire encoding | Correct existing choices first; add `none` and parallel controls at the breaking release | 0, 1 |
| 2. Files | Confirmed missing `purpose` rejects current responses | Bridge parsing; model current metadata and expiration separately from legacy purpose | 0, 1 |
| 3. Content blocks | Confirmed payload loss | Extend the existing shared union with typed blocks and recursively lossless unknown data | 2 |
| 4. Stop reasons | Confirmed whole-response rejection | Add context-window reason and exact-string unknown fallback | 1 |
| 5. Skills | Current-schema path absent; legacy beta is not proven broken | Add a separate current client and DTOs; retain the dated legacy path | 5 |
| 6. Streaming | Confirmed metadata loss and false success on EOF | Require completed lifecycle; add presence-aware, field-specific accumulation | 0, 3 |
| 7. Token counting | Confirmed option loss | Share an allowlisted prompt projection with Messages | 4 |
| 8. Model helpers | Confirmed catalog drift, not inability to send new IDs | Update constants; distinguish unknown capabilities from unsupported ones | 4 |
| Tool runner | Optional convenience | Explicit callback registry, bounded turns/execution, conservative model checks | 7 |
| JSONL and pagination | Optional convenience and memory improvement | Incremental results; consistent bounded cursor traversal | 6 |
| Service accounts / WIF | Additional resources with authentication prerequisites | Separate OAuth client first, resource clients second, federation exchange third | 8 |

Anthropic's [Skills migration](https://github.com/anthropics/anthropic-sdk-python/commit/e541b4d61a640cb39af78539def933de9b23cc61)
states that requests retaining the dated beta header receive legacy shapes. The current Skills fixture therefore demonstrates
missing current support, without proving existing beta calls fail. Live legacy compatibility remains unverified.

## Release and compatibility contract

Ship a small **0.3.1 bridge**, then a deliberate **0.4.0 compatibility release**. Use normal focused PRs to `main` after
PR #59; follow this repository's [Release Please process](../CONTRIBUTING.md), rather than introducing another repository's
integration-branch or ticket conventions. Open the design issue required by CONTRIBUTING before implementing the breaking redesign;
this plan does not create issues or authorize implementation.

Slice 0 can release immediately. Merge slices 1–4 as bounded PRs, recording breaking changes explicitly, and publish them together
as 0.4.0. Prevent the automated release PR from publishing an intermediate, partly migrated API. Use a release-version override
through Release Please and a `BREAKING CHANGE` description; do not assume its pre-1.0 defaults choose the desired version. Slices
5–8 can follow as additive 0.4.x releases, with their new types designed for extension. A later change to existing public types
needs another minor version boundary, regardless of whether the added fields are optional.

These are actual Rust source breaks: adding exhaustive enum variants, adding fields to public literal-constructible structs,
changing `File.purpose`'s type, changing `ContentBlock::Unknown` from a unit variant, and changing streaming delta field types.
`#[serde(default)]` only preserves wire compatibility. Do not describe these changes as source compatible.

Retain `ToolChoice::Auto`, `ToolChoice::Any`, and `ToolChoice::Tool { name }` construction unchanged. New options variants require
new match cases at 0.4. `StopReason` currently implements `Clone`, **not `Copy`**; `Unknown(String)` does not remove a current
trait, and must not acquire a `Copy` promise. Mark evolving enums and new response DTOs `#[non_exhaustive]` at the 0.4 boundary,
provide constructors/builders, and explain wildcard matches and struct-literal migrations in release notes and compile-checked
examples.

## 0. Bridge: wire fixes and strict termination — 0.3.1

Targets: [common models](../src/models/common.rs), [Files models](../src/models/file.rs), [message
collector](../src/streaming/message_stream.rs), and existing reference, Files, and streaming tests. This slice has no dependency on
the redesign.

- Serialize existing choices as `{"type":"auto"}`, `{"type":"any"}`, and `{"type":"tool","name":"…"}`. Custom deserialization may
  retain old `null` as `Auto` and an untagged `name` object as `Tool`; always write canonical JSON. Old `null` cannot recover
  whether the original caller meant `Any`.
- Default missing `File.purpose: String` to an empty legacy placeholder. State clearly that empty means unavailable, not an
  API-assigned purpose. Keep public fields unchanged and omit the empty placeholder on serialization; complete metadata arrives in
  slice 1.
- Both `collect_message` and `collect_text` require `message_stop`; return the existing `AnthropicError::Stream` on EOF beforehand,
  even after useful output. Consumers deliberately reading raw events may still retain partial output.

Tests must assert exact JSON for every existing choice, parse upload/get/list fixtures without purpose, and reject truncation after
start, a content delta, or message delta. A normal terminated empty-text message must succeed. Do not promise exact zero/null-aware
usage merging in this bridge: current `Usage` parsing has already replaced omitted numeric fields with zero. Acceptance: existing
construction examples compile and the P1 wire/parsing reproductions improve without a public shape change.

## 1. Protocol primitives: tool choices, stop reasons, Files — 0.4.0

Depends on slice 0. Targets: [common models](../src/models/common.rs), [Files API](../src/api/files.rs), [Files
models](../src/models/file.rs), [message builder](../src/builders/message_builder.rs), and public re-exports.

Add `ToolChoice::None` plus `AutoWithOptions`, `AnyWithOptions`, and `ToolWithOptions { name, disable_parallel_tool_use }`. The
options variants store a present `bool`, so explicit `false` differs from omission and has an unambiguous round trip. Add `none()`
and a validated parallel-control method, plus `kind()`/`forced_tool_name()` helpers so callers need not enumerate the options
variants to determine intent. Dispatch by `type` and flag presence with custom serde; reject a parallel flag on `none`,
non-booleans, missing forced-tool names, and reserved-field collisions. Preserve existing builders. Authority: [official choice
parameters](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/tool_choice_param.py).

Add `StopReason::ModelContextWindowExceeded` and `Unknown(String)` with custom string serialization. Preserve an unfamiliar reason
exactly, including in Messages, SSE, and batch entries; never coerce it to `end_turn` or fail a well-formed response. Tests include
all current reasons and one future string. Authority: [pinned stop
reasons](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/stop_reason.py).

Change response `File.purpose` to `Option<String>`; add optional `downloadable` (omission is unknown), nullable `expires_at`, and
flattened extra fields. Keep existing processing fields optional for legacy/session payloads. Add `expires_in_seconds` to
`FileUploadRequest`, validated within the current documented range (3,600–7,776,000 seconds). Standard upload sends only the file
and optional expiration, removing the unsupported upload-purpose field/method at this explicit boundary. Replace unsupported
purpose/ID list filters with the current `ids`, `page`, `scope_id` and `limit` contract. Authority: [Upload
File](https://platform.claude.com/docs/en/api/files/upload).

HTTP mocks cover methods, multipart names/expiration boundaries, default downloadability, null/missing expiration, and
upload/get/list using identical metadata fixtures. Acceptance: canonical choices work through Messages and counting, unknown stop
strings survive, and current Files metadata is retained.

## 2. Content preservation and replay — 0.4.0

Depends on slice 1. Targets: [common models](../src/models/common.rs), [message models](../src/models/message.rs), [stream event
parsing](../src/streaming/event_parser.rs), [session events](../src/models/managed_agents/session_event.rs), and replay tests.
Extend the existing `ContentBlock` shared request/response union; avoid creating overlapping legacy, response, and request enums
with divergent serializers.

Add schema-checked `SearchResult`, `ContainerUpload`, `McpToolUse`, and `Compaction` cases using the exact stable/beta parameter and
response contracts: search source/title/text content, uploaded file ID, MCP ID/name/server/input, and compaction content/encrypted
content/signature/tool changes. Shared representation does not imply every block is valid for every role or API surface. Validate
supported known block/role combinations at request preparation; keep beta activation explicit in `RequestOptions`. Compaction
history handling must follow its documented replay rules, not append an arbitrary summary as ordinary text. Sources: [stable
parameters](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/content_block_param.py),
[beta
parameters](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/beta/beta_content_block_param.py),
and [compaction](https://platform.claude.com/docs/en/build-with-claude/compaction).

Replace unit `Unknown` with `Unknown(RawContentBlock)`. Its validated newtype owns the entire JSON object, including the original
string `type`. Responses automatically preserve unfamiliar discriminators. Requests can explicitly construct a raw block, or
deliberately replay one; send the validated object unchanged and document that service support is unknown. Reject non-objects,
missing/non-string discriminators, and malformed recognized blocks rather than treating any deserialization failure as a future
type.

Known variants preserve extra fields too, recursively through citations, image/document sources, cache controls, and nested
tool-result content. Use one typed representation with flattened extras and reserved-key checks; do not maintain independently
mutable raw and typed copies. Update constructors to initialize extras and all receive/send sites that share this union. Offer a
checked response-to-conversation helper that validates known replay shapes and takes an explicit preserve-unknown policy. Never
route replay through text extraction or replace an unfamiliar block with `type: unknown`.

Define losslessness as preservation of all payload values and unknown fields; known optional null/missing fields may normalize under
documented serde rules. Unknown blocks must round-trip as equal JSON values. Fixtures cover four new types, unknown nested
discriminators, nested extras in known types, malformed known blocks, and block replay through Messages, batches, SSE, and sessions.
Extend [the existing replay workflow](../tests/integration/e2e_test.rs) to prove history retention with HTTP mocks. Acceptance: no
payload becomes unit `Unknown`, and assistant replay keeps signed thinking and tool identifiers intact.

## 3. Streaming accumulator and response metadata — 0.4.0

Depends on slices 1–2. Targets: [message models](../src/models/message.rs), [usage models](../src/models/common.rs), [SSE
parser](../src/streaming/event_parser.rs), and [collector](../src/streaming/message_stream.rs). Add optional diagnostics,
context-management/input-transformation metadata, and response extras. These additions break public literals and belong in this
release. Preserve beta event-level metadata in an explicit message-delta event DTO rather than assuming it is nested inside `delta`.

Introduce a dedicated `UsageDelta` for `StreamEvent::MessageDelta`, preserving omission instead of defaulting counters to zero. Use
an internal missing/null/value representation for nullable message-delta fields; present zero and empty arrays must remain
distinguishable. Initialize the snapshot from all of `message_start`, then apply this explicit merger:

| Field | Rule |
| --- | --- |
| `stop_reason`, `stop_sequence`, `stop_details` | Omitted retains; present value replaces; present null clears |
| `container` and beta event-level context/input transformations | Replace a non-null value; omitted/null retains; an empty array is a replacement |
| Usage `output_tokens` and other present token counts | Replace cumulative totals, including zero; never sum or take a blanket numeric maximum |
| Optional usage objects, `iterations`, `fallback_credit` | Replace the supplied non-null complete object/array; omitted/null retains |
| Start-only `speed`, `cache_creation`, `inference_geo`, `service_tier`, diagnostics | Retain initial values; do not invent delta aggregation semantics |
| Text/thinking, tool JSON, citations, signature | Append text/thinking and JSON fragments; append citation entries; replace signature on its documented delta |
| Compaction delta | Replace content/encrypted-content snapshots verbatim, without concatenation or decrypting opaque values |
| Unknown metadata | Preserve start fields and event payloads; opaque delta keys use documented replace semantics when known, otherwise remain raw event data |

The [streaming lifecycle](https://platform.claude.com/docs/en/build-with-claude/streaming) defines cumulative delta token counts.
Pin field rules to the [stable
accumulator](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/lib/streaming/_messages.py#L476),
[beta
accumulator](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/lib/streaming/_beta_messages.py#L449),
and [delta usage
schema](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/message_delta_usage.py).
Preserve iteration objects and unknown iteration types; do not add compaction iteration tokens or thinking-token detail to already
inclusive top-level totals. Update the serving model from a documented fallback boundary block.

Track one start, indexed block start/delta/stop lifecycles, and one terminal stop. Reject impossible indices, duplicate
starts/stops, open blocks or pending JSON at terminal stop, invalid completed tool-input JSON, and EOF before stop. Never turn
invalid tool JSON into a string. Unknown SSE events remain available as raw events without aborting known content. Unknown content
deltas that cannot be faithfully accumulated make collection fail explicitly while raw streaming remains usable. Preserve complete
unknown blocks that require no delta merge.

Fix line framing across chunk splits, UTF-8 boundaries, CRLF, multi-line data, and final incomplete frames. Consolidate
`parse_event` and `finish_event` so both preserve unknown events and field presence consistently. Bound frame/input buffers and
cancel the producer on consumer drop. Tests use controlled chunk delivery, all truncation points, zero/null/omitted values, two
cumulative updates, nested-object replacements, empty arrays, start-only metadata, fallback model changes, and all known iteration
kinds. Acceptance: collected current fixtures match equivalent non-streaming payload values and incomplete streams never report
success.

## 4. Counting parity and conservative model capabilities — 0.4.0

Depends on slices 1–2; can develop beside slice 3. Targets: [Messages API](../src/api/messages.rs), [message
models](../src/models/message.rs), [builder](../src/builders/message_builder.rs), [catalog](../src/config.rs), and [Models
API](../src/api/models.rs).

Add `thinking`, `tool_choice`, `output_config`, and `cache_control` to `TokenCountRequest`, with matching fluent setters. Provide
shared typed `PromptOptions`/projection logic for these countable options and `TokenCountRequest::from_message(&MessageRequest)`.
Keep wire fields flat; do not require a new nested field on `MessageRequest`. Project only fields accepted by counting, excluding
generation, streaming, diagnostics, container, and unsupported beta-only options. Return a validation error for options that prevent
an equivalent count instead of silently claiming parity. Both paths share profile-header preparation; caller beta/workspace options
stay explicit. Authority: [counting
parameters](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/message_count_tokens_params.py).

Add `OPUS_5_5`, `SONNET_5_5`, `FABLE_5_1`, and `MYTHOS_5_1`; retain prior constants and the default model. Use an exact-ID
capability table backed by dated sources. Add a supported/unsupported/unknown query; existing boolean helpers return true only for
documented support. Keep arbitrary string IDs valid for transport and clarify `is_valid_model` as catalog membership. Use Models API
metadata for capabilities it actually exposes; it does not provide forced-tool-choice support. Unknown IDs never inherit guessed
family limits or trigger local rejection solely for being absent from the catalog.

The four new IDs reject forced `any`/`tool` selection; known model checks must also honor their thinking/prefill/sampling
restrictions. Do not generalize that all adaptive-thinking models reject forcing. Sources: [Opus
migration](https://platform.claude.com/docs/en/models/opus-5-5/migration-guide), [Sonnet
migration](https://platform.claude.com/docs/en/models/sonnet-5-5/migration-guide), [Fable/Mythos
migration](https://platform.claude.com/docs/en/models/fable-5-1/migration-guide). Sonnet's `between_tools` mode requires its own
typed configuration constraints; support it only with the documented effort levels, without injecting unsupported budget/display
fields. Mythos access and beta eligibility remain account-specific. Mocks compare the countable JSON subset and attribution headers
with Messages; tests cover new IDs, aliases, retired IDs, unknown strings, and incompatible known options. Acceptance: counting
preserves supported prompt configuration and catalog updates do not restrict use of future models.

## 5. Current Skills alongside legacy beta — additive 0.4.x

Depends on the 0.4 release, not on convenience work. Targets: [Skills API](../src/api/skills.rs), [Skills
models](../src/models/skill.rs), [client factories](../src/client.rs), and Skills mock/reference tests. Add `CurrentSkillsApi`
through `Client::skills_current()` and distinct `CurrentSkill`, `CurrentSkillVersion`, create/list DTOs and source-object types.
Keep existing `skills()`, `Skill`, and legacy version DTOs unchanged so this slice remains source compatible. An explicit
`skills_legacy()` alias can help documentation; do not silently switch existing calls to the current schema.

Current requests omit the SDK-injected dated Skills beta header and send `display_name`; model object `source`, `latest_version_id`,
and version IDs used in paths. Reject conflicting legacy Skills header selection on the current client; preserve unrelated
explicitly requested betas. Reuse safe directory upload logic and shared transport without changing legacy forms. Sources: [current
Skill](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/skill.py),
[create
parameters](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/skill_create_params.py),
and [current
version](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/skills/skill_version.py).
Mocks independently assert legacy/current forms, header selection, list/get/create/delete and version lifecycle. Acceptance:
existing legacy examples compile unchanged and current fixtures parse with no lossy field aliases.

## 6. Bounded pagination and incremental batch JSONL — additive 0.4.x

Depends on lossless message results; split into a pagination PR and JSONL PR. Targets: [pagination types](../src/types.rs), [API
utilities](../src/api/utils.rs), [Models](../src/api/models.rs), [Files](../src/api/files.rs), [Skills](../src/api/skills.rs),
[batches](../src/api/message_batches.rs), and streaming. Add shared `PageStream`/`PaginationLimits` with bounded defaults,
configurable page/item ceilings, and `list_all_with_limits`; retain single-page methods. Existing `list_all` delegates to documented
finite defaults and errors on a limit rather than returning a silently truncated vector.

Adapters must preserve endpoint-specific filters and options: ID cursors use `last_id`/`after`, token cursors use
`next_page`/`page`. Token-based adapters determine continuation from `next_page` without requiring `has_more`.
Reject simultaneous forward/reverse cursors, missing next cursor while `has_more`, empty continuing pages,
repeated/cyclic cursors, and unchanged page/item progress. Encode query values, honor cancellation, and preserve requests'
retry/timeout settings. Mocks cover three normal pages, empty terminal pages, all cursor failures, filters, options, configured
ceilings and endpoint-specific cursor names.

Add `results_stream` yielding `Result<MessageBatchResultEntry>` directly from HTTP bytes, with configurable maximum row bytes and
backpressure. Decode a whole row after newline, supporting split UTF-8/JSON, LF/CRLF, blank rows and a valid final row without
newline. Fail once on malformed/oversized rows or transport failure with row number and no raw sensitive payload; then terminate.
Dropping the stream releases HTTP work; cancellation never buffers remaining results. Keep buffered raw/text methods and implement
typed `results` by collecting the iterator, preserving its existing return type. Sources: [official result
iterator](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/resources/messages/batches.py)
and
[pagination](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/README.md#pagination).
Acceptance: first entry is yielded before download completes, retained memory is bounded by one row plus transport buffering, and
all convenience traversal terminates under adversarial mock responses.

## 7. Optional explicit tool runner — additive 0.4.x

Depends on slices 1–4 and completed streaming; separate non-streaming runner from a later streaming adapter. Add a `tool_runner`
module, `ToolRegistry`, `ToolRunnerOptions`, and compile-checked examples without changing low-level Messages behavior. Match [the
optional official workflow](https://platform.claude.com/docs/en/agents-and-tools/tool-use/tool-runner), without promising every
SDK-specific convenience in the first slice.

Users explicitly register named async callbacks and input decoders/validators. Reject duplicate names and unregistered tools before
executing callbacks. Only client `tool_use` invokes callbacks; server/MCP tools and unknown blocks do not create hidden execution.
The runner makes side effects only through registered callbacks, with an optional user execution hook before invocation. Default to
sequential execution; opt-in bounded concurrency respects the parallel-choice flag and preserves result order and tool-use IDs.

Require finite max turns, total calls, per-call/overall timeouts and result-byte limits. Cancellation stops future work; do not
retry callbacks automatically, re-execute duplicate call IDs, change model, inject thinking settings or force tools. Validate
documented known-model incompatibilities; unknown-model callers choose conservative `auto` defaults or explicitly supply their
configuration. Keep complete assistant history, including signed thinking; append the matching user tool-result blocks. Apply
explicit compaction policy rather than discarding history. Stop safely on end-turn, refusal, max tokens, pause-turn, context-window
limit, unknown stop reasons, budget exhaustion or unsupported replay content. Policy-controlled callback failures may emit sanitized
`is_error` results; default errors return partial transcript/usage through a runner-specific result.

Mocks and fake callbacks cover multiple turns, parallel order, schemas, unknown tools, duplicate IDs, callback failure, termination
reasons and every budget. Acceptance: no callback runs during partial SSE, no registered callback is invoked twice by transport
retry, and the transcript replays without content loss.

## 8. OAuth administration and WIF prerequisites — additive 0.4.x

This is independent of the tool runner. Split into three PRs: OAuth transport, resource clients, then an optional federation token
provider. Targets: [client transport](../src/client.rs), [Admin API](../src/api/admin/mod.rs), [configuration](../src/config.rs),
and new dedicated OAuth modules/DTOs. Existing `request_admin` strips bearer authorization and installs the admin key; it cannot
implement these resources. Generic bearer heuristics also do not provide a credential contract or refresh lifecycle.

Introduce `OAuthAdminClient` with a separate config and explicit bearer-token or async token-provider constructors. Redact
credential debugging; send only Bearer authorization, never an API/admin key. Its `from_env` deliberately reads
`ANTHROPIC_AUTH_TOKEN` and rejects conflicting credential selections. Reuse non-authentication transport behavior, timeouts, error
parsing and paging. A static token never pretends to refresh. Providers report expiration, refresh under a shared single-flight
lock, and cannot initiate interactive login. Bound 401 recovery; do not blindly replay mutations after uncertain delivery.

Add service-account and federation issuer/rule list/get/create/update/archive clients under `/v1/organizations`, including
documented workspace subresources, typed discriminators with extras, and page-token traversal from slice 6. Resources require
`org:admin` OAuth credentials; admin API keys are rejected. User and service-account tokens have different operation permissions.
Enforce documented local constraints while preserving actionable 401/403 responses. OAuth callers cannot create broader-scope
federation rules: the bootstrap `org:admin` rule must be created in the Console. Source: [WIF
administration](https://platform.claude.com/docs/en/manage-claude/wif-admin-api), [service
accounts](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/resources/organization/service_accounts/service_accounts.py),
and [federation
rules](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/resources/organization/federation/rules/rules.py).

Federation exchange is prerequisite-dependent, not something CRUD clients should improvise. Pin its official exchange schema and
caller permission matrix before coding a provider; implement explicit organization/service-account/rule selection, subject-token
source, expiry/skew, token-file reread and refresh. Do not fetch cloud metadata, execute a CLI, or create trust rules implicitly.
Mocks cover header isolation, expiry/concurrent refresh, bad scopes, 401/403, request methods/bodies, archive dependencies and
cursor limits. Acceptance: token-only resource clients work independently of exchange support; WIF is advertised only after exchange
and refresh have their own proven fixtures.

## Validation and completion evidence

For every implementation PR, extend existing unit/reference fixtures and `wiremock` integration tests; use a controlled chunked
local server where a buffered mock cannot demonstrate incremental delivery or cancellation. Update rustdoc, [API
coverage](api-coverage.md), examples and changelog context. Record the official schema revision, header version and verification
date. Only new changes or failed checks justify repeating broader validation.

Credential-free gates are the pinned toolchain's formatting, strict clippy, unit/integration/reference suites, doctests,
warnings-as-errors documentation, example builds, and `python3 scripts/check_docs.py`. Before an implementation commit/push run
repository `make ci`; use the repository's available hook setup rather than inventing `make hooks-install` if that target is absent.
Release or dependency slices additionally run audit/deny and `cargo package --locked`. Run both native-TLS and rustls test
configurations when transport changes. Check a downstream compile fixture for the documented 0.3 → 0.4 migration.

No live call is required to establish wire serialization, schema retention, stream termination, bounded iteration or callback
behavior. Live smoke tests are separate, opt-in work with dedicated credentials and documented quota, resource cleanup, model access
and beta eligibility. Cover one forced-tool call on a compatible model, Files upload/get/list/delete, counting and SSE, both Skills
schema paths, a small batch, and a deterministic registered tool. OAuth administration needs its own scoped credential and isolated
disposable resources; archive rules before referenced issuers/accounts. Do not bootstrap organization-admin trust during validation
or claim gated features were tested when account access was unavailable.

Completion means all eight findings have fixture/HTTP evidence, the 0.4 migration is explicit, convenience APIs are bounded, and
OAuth prerequisites are documented. Missing live eligibility is reported separately from offline acceptance; it must not silently
become a claim of complete service parity.

## Implementation adjustments

The source implements the fixes and optional additions together for an atomic 0.4 migration;
a separate 0.3.1 backport/release was not published. The implementation uses a one-time
`Release-As: 0.4.0` commit footer and pre-1.0 Release Please settings. The former competing
automatic tagger is now a manual PR-only helper, preventing independent release of a partial
migration. Cargo and the release manifest retain the last published version until the version PR.

Pinned schema verification corrected Files assumptions: `downloadable` is optional, current lists
use `next_page`/`page`, and the current contract has no upload purpose or purpose-list filter.
Models now retain the complete `ModelCapabilities` payload instead of flattening away capability
metadata; this is another explicitly documented 0.4 field-type migration.

See [the migration guide](migration-0.4.md) and [coverage table](api-coverage.md) for concrete API
changes, tests, bounds, authentication prerequisites, and the limits of offline validation.
