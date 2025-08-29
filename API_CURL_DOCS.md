# Anthropic API cURL Examples

This document provides tested cURL examples for all Anthropic API endpoints. These examples demonstrate how to interact with the API directly and can help you understand the request/response format for the Threatflux SDK.

## Table of Contents

- [Authentication](#authentication)
- [Messages API](#messages-api)
- [Models API](#models-api)
- [Message Batches API](#message-batches-api)
- [Files API](#files-api)
- [Admin API](#admin-api)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)

## Authentication

All API requests require authentication via the `Authorization` header:

```bash
export ANTHROPIC_API_KEY="your_api_key_here"
export ANTHROPIC_ADMIN_KEY="your_admin_key_here"  # For admin endpoints
```

**Headers required for all requests:**
- `Authorization: Bearer $ANTHROPIC_API_KEY`
- `Content-Type: application/json`
- `anthropic-version: 2023-06-01`

## Available Models

### Claude 4 Models (Latest)
- **Claude Opus 4.1** (`claude-opus-4-1-20250805`) - World's best coding model, 74.5% on SWE-bench
- **Claude Opus 4** (`claude-opus-4-20250514`) - Previous Opus version, 72.5% on SWE-bench
- **Claude Sonnet 4** (`claude-sonnet-4-20250514`) - Balanced performance, 1M context available

### Claude 3 Models
- **Claude 3.7 Sonnet** (`claude-3-7-sonnet-20250219`) - Previous Sonnet version
- **Claude 3.5 Haiku** (`claude-3-5-haiku-20241022`) - Fastest, most cost-effective (default for testing)
- **Claude 3.5 Sonnet** (`claude-3-5-sonnet-20241022`) - Balanced performance
- **Claude 3 Opus** (`claude-3-opus-20240229`) - Maximum intelligence

## Messages API

The Messages API allows you to send messages to Claude and receive responses.

### Create a Message

**Basic text message:**

```bash
curl -X POST "https://api.anthropic.com/v1/messages" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-haiku-20241022",
    "max_tokens": 1000,
    "messages": [
      {
        "role": "user",
        "content": "Hello, Claude! Can you explain what you are?"
      }
    ]
  }'
```

**Response:**
```json
{
  "id": "msg_01ABC123...",
  "type": "message",
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "Hello! I'm Claude, an AI assistant created by Anthropic..."
    }
  ],
  "model": "claude-3-5-haiku-20241022",
  "stop_reason": "end_turn",
  "stop_sequence": null,
  "usage": {
    "input_tokens": 15,
    "output_tokens": 45
  }
}
```

### Message with System Prompt

```bash
curl -X POST "https://api.anthropic.com/v1/messages" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-haiku-20241022",
    "max_tokens": 500,
    "system": "You are a helpful coding assistant specializing in Rust programming.",
    "messages": [
      {
        "role": "user",
        "content": "Write a simple Hello World program in Rust"
      }
    ],
    "temperature": 0.3
  }'
```

### Message with Extended Thinking (Claude 4 Models)

```bash
curl -X POST "https://api.anthropic.com/v1/messages" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-opus-4-1-20250805",
    "max_tokens": 4096,
    "thinking": {
      "type": "enabled",
      "budget_tokens": 64000
    },
    "messages": [
      {
        "role": "user",
        "content": "Solve this complex algorithmic problem: [problem description]"
      }
    ]
  }'
```

### Sonnet 4 with 1M Context Window

```bash
curl -X POST "https://api.anthropic.com/v1/messages" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -H "anthropic-beta: context-1m-2025-08-07" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "max_tokens": 4096,
    "messages": [
      {
        "role": "user",
        "content": "[Very large document or codebase, 500K+ tokens]"
      }
    ]
  }'
```

### Conversation with Multiple Messages

```bash
curl -X POST "https://api.anthropic.com/v1/messages" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-haiku-20241022",
    "max_tokens": 300,
    "messages": [
      {
        "role": "user",
        "content": "What is 2+2?"
      },
      {
        "role": "assistant", 
        "content": "2 + 2 = 4"
      },
      {
        "role": "user",
        "content": "What about 3+3?"
      }
    ]
  }'
```

### Streaming Messages

```bash
curl -X POST "https://api.anthropic.com/v1/messages" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-haiku-20241022",
    "max_tokens": 500,
    "stream": true,
    "messages": [
      {
        "role": "user",
        "content": "Write a short story about a robot"
      }
    ]
  }'
```

**Streaming Response Format (Server-Sent Events):**
```
event: message_start
data: {"type": "message_start", "message": {...}}

event: content_block_start
data: {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}

event: content_block_delta
data: {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Once"}}

event: content_block_delta
data: {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " upon"}}

...

event: message_stop
data: {"type": "message_stop"}
```

### Message with Image (Vision)

```bash
curl -X POST "https://api.anthropic.com/v1/messages" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-haiku-20241022",
    "max_tokens": 300,
    "messages": [
      {
        "role": "user",
        "content": [
          {
            "type": "image",
            "source": {
              "type": "base64",
              "media_type": "image/png",
              "data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg=="
            }
          },
          {
            "type": "text",
            "text": "What do you see in this image?"
          }
        ]
      }
    ]
  }'
```

### Count Tokens

```bash
curl -X POST "https://api.anthropic.com/v1/messages/count_tokens" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-haiku-20241022",
    "messages": [
      {
        "role": "user",
        "content": "Hello, how many tokens is this message?"
      }
    ]
  }'
```

**Response:**
```json
{
  "input_tokens": 12
}
```

## Models API

The Models API provides information about available models.

### List Models

```bash
curl -X GET "https://api.anthropic.com/v1/models" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

**Response:**
```json
{
  "data": [
    {
      "id": "claude-3-5-haiku-20241022",
      "type": "model",
      "display_name": "Claude 3.5 Haiku",
      "max_tokens": 200000,
      "input_cost_per_token": 0.00025,
      "output_cost_per_token": 0.00125,
      "created_at": "2024-10-22T00:00:00Z"
    }
  ],
  "has_more": false,
  "first_id": "claude-3-5-haiku-20241022",
  "last_id": "claude-3-opus-20240229"
}
```

### List Models with Pagination

```bash
curl -X GET "https://api.anthropic.com/v1/models?limit=2&after=claude-3-haiku" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

### Get Specific Model

```bash
curl -X GET "https://api.anthropic.com/v1/models/claude-3-5-haiku-20241022" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

**Response:**
```json
{
  "id": "claude-3-5-haiku-20241022",
  "type": "model", 
  "display_name": "Claude 3.5 Haiku",
  "description": "Fast and lightweight model for everyday tasks",
  "max_tokens": 200000,
  "max_output_tokens": 8192,
  "input_cost_per_token": 0.00025,
  "output_cost_per_token": 0.00125,
  "capabilities": ["vision", "tool_use"],
  "created_at": "2024-10-22T00:00:00Z",
  "updated_at": "2024-10-22T00:00:00Z"
}
```

## Message Batches API

The Message Batches API allows you to process multiple messages in a single batch request.

### Create a Batch

```bash
curl -X POST "https://api.anthropic.com/v1/messages/batches" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "requests": [
      {
        "custom_id": "request-1",
        "params": {
          "model": "claude-3-5-haiku-20241022",
          "max_tokens": 100,
          "messages": [
            {
              "role": "user",
              "content": "What is 2+2?"
            }
          ]
        }
      },
      {
        "custom_id": "request-2", 
        "params": {
          "model": "claude-3-5-haiku-20241022",
          "max_tokens": 100,
          "messages": [
            {
              "role": "user",
              "content": "What is the capital of France?"
            }
          ]
        }
      }
    ]
  }'
```

**Response:**
```json
{
  "id": "batch_01ABC123...",
  "type": "message_batch",
  "processing_status": "in_progress",
  "request_counts": {
    "total": 2,
    "completed": 0,
    "failed": 0,
    "cancelled": 0
  },
  "created_at": "2024-01-15T10:00:00Z",
  "expires_at": "2024-01-16T10:00:00Z"
}
```

### Retrieve a Batch

```bash
curl -X GET "https://api.anthropic.com/v1/messages/batches/batch_01ABC123..." \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

### List Batches

```bash
curl -X GET "https://api.anthropic.com/v1/messages/batches" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

### Cancel a Batch

```bash
curl -X POST "https://api.anthropic.com/v1/messages/batches/batch_01ABC123.../cancel" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

### Delete a Batch

```bash
curl -X DELETE "https://api.anthropic.com/v1/messages/batches/batch_01ABC123..." \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

## Files API

The Files API allows you to upload and manage files.

### Upload a File

```bash
curl -X POST "https://api.anthropic.com/v1/files" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -F "file=@document.pdf" \
  -F "purpose=user_data"
```

**Response:**
```json
{
  "id": "file_01ABC123...",
  "type": "file",
  "filename": "document.pdf",
  "mime_type": "application/pdf",
  "size_bytes": 12345,
  "purpose": "user_data",
  "created_at": "2024-01-15T10:00:00Z",
  "status": "ready"
}
```

### Upload Text File

```bash
echo "This is test content for the API" > test.txt
curl -X POST "https://api.anthropic.com/v1/files" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -F "file=@test.txt" \
  -F "purpose=user_data"
```

### List Files

```bash
curl -X GET "https://api.anthropic.com/v1/files" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

### List Files with Pagination

```bash
curl -X GET "https://api.anthropic.com/v1/files?limit=10&after=file_01ABC123" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

### Get File Information

```bash
curl -X GET "https://api.anthropic.com/v1/files/file_01ABC123..." \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

### Download File

```bash
curl -X GET "https://api.anthropic.com/v1/files/file_01ABC123.../download" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -o downloaded_file.pdf
```

### Delete File

```bash
curl -X DELETE "https://api.anthropic.com/v1/files/file_01ABC123..." \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

## Admin API

The Admin API requires an admin key and provides access to organization management features.

### Get Organization Information

```bash
curl -X GET "https://api.anthropic.com/v1/organization" \
  -H "Authorization: Bearer $ANTHROPIC_ADMIN_KEY" \
  -H "anthropic-version: 2023-06-01"
```

**Response:**
```json
{
  "id": "org_01ABC123...",
  "name": "Example Organization",
  "display_name": "Example Org",
  "created_at": "2024-01-01T00:00:00Z",
  "settings": {
    "default_model": "claude-3-5-haiku-20241022",
    "features": ["batch_api", "admin_api"]
  }
}
```

### List Organization Members

```bash
curl -X GET "https://api.anthropic.com/v1/organization/members" \
  -H "Authorization: Bearer $ANTHROPIC_ADMIN_KEY" \
  -H "anthropic-version: 2023-06-01"
```

### Add Organization Member

```bash
curl -X POST "https://api.anthropic.com/v1/organization/members" \
  -H "Authorization: Bearer $ANTHROPIC_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "email": "newuser@example.com",
    "role": "member",
    "name": "New User"
  }'
```

### List Workspaces

```bash
curl -X GET "https://api.anthropic.com/v1/organization/workspaces" \
  -H "Authorization: Bearer $ANTHROPIC_ADMIN_KEY" \
  -H "anthropic-version: 2023-06-01"
```

### Create Workspace

```bash
curl -X POST "https://api.anthropic.com/v1/organization/workspaces" \
  -H "Authorization: Bearer $ANTHROPIC_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "name": "development",
    "display_name": "Development Team",
    "description": "Workspace for development team"
  }'
```

### List API Keys

```bash
curl -X GET "https://api.anthropic.com/v1/organization/api_keys" \
  -H "Authorization: Bearer $ANTHROPIC_ADMIN_KEY" \
  -H "anthropic-version: 2023-06-01"
```

### Create API Key

```bash
curl -X POST "https://api.anthropic.com/v1/organization/api_keys" \
  -H "Authorization: Bearer $ANTHROPIC_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "name": "Production API Key",
    "description": "Key for production environment",
    "permissions": ["messages:read", "messages:write"]
  }'
```

### Get Usage Report

```bash
curl -X GET "https://api.anthropic.com/v1/organization/usage?start_date=2024-01-01&end_date=2024-01-31" \
  -H "Authorization: Bearer $ANTHROPIC_ADMIN_KEY" \
  -H "anthropic-version: 2023-06-01"
```

**Response:**
```json
{
  "input_tokens": 125000,
  "output_tokens": 75000,
  "request_count": 1500,
  "cost": {
    "total_cost_cents": 12500,
    "input_cost_cents": 3125,
    "output_cost_cents": 9375,
    "currency": "USD"
  }
}
```

## Error Handling

All API endpoints return structured error responses for failures.

### Common Error Responses

**400 Bad Request:**
```json
{
  "type": "invalid_request_error",
  "message": "Invalid model specified"
}
```

**401 Unauthorized:**
```json
{
  "type": "authentication_error", 
  "message": "Invalid API key"
}
```

**429 Too Many Requests:**
```json
{
  "type": "rate_limit_error",
  "message": "Rate limit exceeded"
}
```

**500 Internal Server Error:**
```json
{
  "type": "api_error",
  "message": "Internal server error"
}
```

### Testing Error Scenarios

**Invalid API key:**
```bash
curl -X POST "https://api.anthropic.com/v1/messages" \
  -H "Authorization: Bearer invalid-key" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-haiku-20241022",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

**Invalid model:**
```bash
curl -X POST "https://api.anthropic.com/v1/messages" \
  -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
  -H "Content-Type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "invalid-model-name",
    "max_tokens": 100,
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

## Rate Limiting

The API implements rate limiting with the following headers in responses:

- `x-ratelimit-remaining`: Requests remaining in current window
- `x-ratelimit-limit`: Total requests allowed in window
- `x-ratelimit-reset`: Unix timestamp when window resets
- `retry-after`: Seconds to wait before retrying (on 429 errors)

### Rate Limit Testing

```bash
# Make multiple rapid requests to test rate limiting
for i in {1..10}; do
  echo "Request $i:"
  curl -s -w "Status: %{http_code}, Rate Limit Remaining: %{header_x-ratelimit-remaining}\n" \
    -X POST "https://api.anthropic.com/v1/messages" \
    -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
    -H "Content-Type: application/json" \
    -H "anthropic-version: 2023-06-01" \
    -d '{
      "model": "claude-3-5-haiku-20241022",
      "max_tokens": 50,
      "messages": [{"role": "user", "content": "Hello"}]
    }' > /dev/null
  sleep 0.1
done
```

## Testing the Examples

To test these examples with your own API key:

1. **Set up environment:**
   ```bash
   export ANTHROPIC_API_KEY="your_api_key_here"
   export ANTHROPIC_ADMIN_KEY="your_admin_key_here"
   ```

2. **Test basic message:**
   ```bash
   curl -X POST "https://api.anthropic.com/v1/messages" \
     -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
     -H "Content-Type: application/json" \
     -H "anthropic-version: 2023-06-01" \
     -d '{
       "model": "claude-3-5-haiku-20241022",
       "max_tokens": 100,
       "messages": [{"role": "user", "content": "Hello!"}]
     }'
   ```

3. **Test models endpoint:**
   ```bash
   curl -X GET "https://api.anthropic.com/v1/models" \
     -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
     -H "anthropic-version: 2023-06-01"
   ```

4. **Test token counting:**
   ```bash
   curl -X POST "https://api.anthropic.com/v1/messages/count_tokens" \
     -H "Authorization: Bearer $ANTHROPIC_API_KEY" \
     -H "Content-Type: application/json" \
     -H "anthropic-version: 2023-06-01" \
     -d '{
       "model": "claude-3-5-haiku-20241022",
       "messages": [{"role": "user", "content": "Test message"}]
     }'
   ```

## Notes

- **API Version**: All examples use `anthropic-version: 2023-06-01`
- **Model**: Examples use `claude-3-5-haiku-20241022` as the default test model
- **Authentication**: Replace placeholder API keys with your actual keys
- **Rate Limits**: Be mindful of rate limits when testing multiple requests
- **Costs**: API usage incurs costs - monitor your usage in the Anthropic Console

---

*These examples were tested against the Anthropic API and are kept up-to-date with the latest API specifications.*