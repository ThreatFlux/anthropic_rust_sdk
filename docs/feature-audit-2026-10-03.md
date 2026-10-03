# SDK feature audit: 2026-10-03

Checked on **2026-10-03** against the official Claude API reference and Anthropic's
Python SDK. Repository API code reviewed at upstream commit
`5207544e352b06d67251600c809e00b82ca11d56`.

This is a bounded compatibility audit, not a claim of complete service parity.
The Rust and dependency modernization accompanying this report does not fix the
existing API gaps below. No authenticated service calls were made.

## What already exists

The SDK implements Messages creation/streaming, token counting, model retrieval,
batches, files, skills/version operations, and several administration clients.
Adaptive thinking, effort, JSON Schema outputs, strict tool definitions, prompt
caching, and server-tool definitions already have request support. Containers,
context management, MCP server configuration, and extra tool fields can pass
through JSON values; those features should not be described as wholly absent.
See the existing [coverage inventory](api-coverage.md) for the resource list.

## Prioritized compatibility findings

### 1. P1: Explicit tool choice uses the wrong JSON shape

[ToolChoice](../src/models/common.rs), lines 705–716, is an untagged enum.
The actual source serializes `Auto` and `Any` as `null`, and
`Tool { name: "get_weather" }` as `{"name":"get_weather"}`. The API expects
an object with `type`; consequently the builder's forced-tool options cannot
reliably request the intended behavior. `none` and `disable_parallel_tool_use`
also have no representation. Verified by serialization in the isolated harness.
Authority: [official tool-use example](https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview)
and [official tool-choice union](https://github.com/anthropics/anthropic-sdk-python/blob/main/src/anthropic/types/tool_choice_param.py).

### 2. P1: Valid Files responses fail deserialization

[File](../src/models/file.rs), lines 9–30, requires `purpose: String`.
The current FileMetadata response has `downloadable` and optional `expires_at`,
and does not require or show `purpose`. Deserializing the documented payload
fails with `missing field purpose`; upload, get, and list share this model.
The upload request also lacks `expires_in_seconds`, and the response loses
downloadability/expiration metadata. Verified with an official-shaped fixture.
Authority: [Upload File](https://platform.claude.com/docs/en/api/files/upload).

### 3. P1: Missing content variants discard payloads and break conversation replay

[ContentBlock](../src/models/common.rs), lines 253–374, has a unit `Unknown`
fallback. `search_result`, `container_upload`, `mcp_tool_use`, and `compaction`
all deserialize as `Unknown`, then serialize as `{"type":"unknown"}`; their
original fields disappear. This blocks typed search-result/file input and
replay of MCP or compacted conversations. A generic JSON request can bypass
the input limitation, but does not make typed responses lossless. The same
union is used for streaming content. Verified with all four fixture variants.
Authority: [stable content parameters](https://github.com/anthropics/anthropic-sdk-python/blob/main/src/anthropic/types/content_block_param.py)
and [beta response content union](https://github.com/anthropics/anthropic-sdk-python/blob/main/src/anthropic/types/beta/beta_content_block.py).

### 4. P1: A documented stop reason rejects the entire response

[StopReason](../src/models/common.rs), lines 747–763, lacks
`model_context_window_exceeded` and has no unknown-value fallback.
The fixture fails with `unknown variant model_context_window_exceeded`.
This affects ordinary responses, streamed message deltas, and parsed batch
results containing such a message, instead of preserving the partial output.
Authority: [official StopReason definition](https://github.com/anthropics/anthropic-sdk-python/blob/main/src/anthropic/types/stop_reason.py).

### 5. P2: Skills targets the older beta schema rather than current stable schemas

[Skill](../src/models/skill.rs), lines 29–55, expects a string `source`,
`display_title`, and `latest_version`. The current stable response uses an
object `source`, `display_name`, and `latest_version_id`; the documented
fixture fails with `invalid type: map, expected a string`. The
[upload form](../src/api/skills.rs), lines 102–123, likewise sends
`display_title`. Older Python beta response types retain the legacy fields,
so this finding is a missing current stable-schema path, not proof that all
legacy beta calls fail. The SDK always adds its Skills beta header.
Authority: [current Create Skill](https://platform.claude.com/docs/en/api/skills/create),
[current beta Create Skill](https://platform.claude.com/docs/en/api/beta/skills/create),
and [legacy beta response type](https://github.com/anthropics/anthropic-sdk-python/blob/main/src/anthropic/types/beta/skill_create_response.py).

### 6. P2: Streaming collection drops metadata and accepts an unfinished stream

[collect_message](../src/streaming/message_stream.rs), lines 154–249, merges
selected fields only. A streamed refusal fixture preserves its stop reason
but loses delta `stop_details`/fallback credit and `container`; usage `speed`,
`output_tokens_details`, `iterations`, and `fallback_credit` are also dropped.
The raw delta exposes extra fields, so callers can implement their own merger.
A fixture ending after `message_start`, without `message_stop`, returns `Ok`
with an unfinished message. Both behaviors were reproduced with in-memory SSE.
[MessageResponse](../src/models/message.rs), lines 780–808, additionally lacks
response diagnostics and beta context-management metadata.
Authority: [official message schema](https://github.com/anthropics/anthropic-sdk-python/blob/main/src/anthropic/types/message.py)
and [official beta message delta](https://github.com/anthropics/anthropic-sdk-python/blob/main/src/anthropic/types/beta/beta_raw_message_delta_event.py),
and [streaming lifecycle](https://platform.claude.com/docs/en/build-with-claude/streaming).

### 7. P2: Token counting cannot mirror several supported Messages options

[TokenCountRequest](../src/models/message.rs), lines 828–844, exposes model,
messages, system, tools, and profile attribution. It lacks `thinking`,
`tool_choice`, `output_config`, and top-level `cache_control`, which the
current endpoint accepts. Deserializing a fixture with those options and
reserializing it drops all four. Callers must use the generic request method
to count an equivalently configured request.
Authority: [Count tokens in a Message](https://platform.claude.com/docs/en/api/messages/count_tokens).

### 8. P2: Local model catalog and capability predicates are stale

[Model constants and predicates](../src/config.rs), lines 19–145, omit
`claude-opus-5-5`, `claude-sonnet-5-5`, `claude-fable-5-1`, and
`claude-mythos-5-1` from the current official model union. Exact-match helpers
therefore reject these IDs or report missing capabilities. The request's
string model setter and Models API still permit using newly released models;
this is a helper/catalog limitation rather than a transport restriction.
Authority: [official current model union](https://github.com/anthropics/anthropic-sdk-python/blob/main/src/anthropic/types/model.py).

## Optional SDK improvements

These are convenience or additional-resource gaps, separate from the failures
above. They are candidates for follow-up work, not mandatory modernization.

- **Tool runner:** [MessagesApi](../src/api/messages.rs) provides requests and
  streams, but no automatic tool execution/conversation loop. The
  [official tool runner](https://platform.claude.com/docs/en/agents-and-tools/tool-use/tool-runner)
  provides that optional beta convenience in other languages.
- **Incremental batch results and consistent pagination:**
  [batch results](../src/api/message_batches.rs), lines 155–225, buffer the full
  JSONL response before parsing. Files and batches lack general `list_all`
  helpers, while Models/Skills have them. Existing single-page operations work;
  callers can paginate manually. Compare the
  [official Python SDK](https://github.com/anthropics/anthropic-sdk-python#pagination)
  and its [batch-result iterator](https://github.com/anthropics/anthropic-sdk-python/blob/main/src/anthropic/resources/messages/batches.py).
- **Additional administration resources:** [AdminApi](../src/api/admin/mod.rs)
  exposes organization/users, workspaces, API keys, and usage, but no dedicated
  service-account/federation clients. The
  [service-account API](https://platform.claude.com/docs/en/api/organization/service_accounts/create)
  and [WIF administration](https://platform.claude.com/docs/en/manage-claude/wif-admin-api)
  require OAuth credentials; simply adding methods to the admin-key client
  would not satisfy their authentication contract.

## Verification notes

An isolated Rust harness outside the checkout imported the actual common,
message, file, skill, SSE parser, and stream collector source files. It ran
offline with cached dependencies, synthetic fixtures, and in-memory HTTP
responses; no repository API implementation or existing tests were changed.
The harness proved findings 1–7, except the Skills multipart mismatch and
unmodeled response fields, which were checked directly in source. Finding 8
and the optional gaps are source/reference comparisons. Live account behavior,
beta rollout eligibility, and complete Managed Agents parity remain untested.
