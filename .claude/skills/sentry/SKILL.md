---
name: sentry
description: Investigate Sentry issues, analyze error traces, and help debug production incidents. Use when users ask about Sentry errors, production outages, error investigation, or need to understand why something is failing in production.
---

# Sentry Issue Investigation

You have access to the Sentry API via the `SENTRY_ACCESS_TOKEN` environment variable. Use curl to interact with the Sentry API.

## Prerequisites

The environment has `SENTRY_ACCESS_TOKEN` set. The organization slug is typically available from context or can be discovered.

## Common Operations

### List Recent Issues

```bash
curl -s -H "Authorization: Bearer $SENTRY_ACCESS_TOKEN" \
  "https://sentry.io/api/0/projects/{organization_slug}/{project_slug}/issues/?query=is:unresolved&statsPeriod=24h" | jq
```

### Get Issue Details

```bash
curl -s -H "Authorization: Bearer $SENTRY_ACCESS_TOKEN" \
  "https://sentry.io/api/0/issues/{issue_id}/" | jq
```

### Get Latest Events for an Issue

```bash
curl -s -H "Authorization: Bearer $SENTRY_ACCESS_TOKEN" \
  "https://sentry.io/api/0/issues/{issue_id}/events/latest/" | jq
```

### Get Issue Heuristics (Tags, User Impact)

```bash
curl -s -H "Authorization: Bearer $SENTRY_ACCESS_TOKEN" \
  "https://sentry.io/api/0/issues/{issue_id}/tags/" | jq
```

### List Projects in Organization

```bash
curl -s -H "Authorization: Bearer $SENTRY_ACCESS_TOKEN" \
  "https://sentry.io/api/0/organizations/{organization_slug}/projects/" | jq '.[] | {slug, name}'
```

### Search Events with Query

```bash
curl -s -H "Authorization: Bearer $SENTRY_ACCESS_TOKEN" \
  "https://sentry.io/api/0/organizations/{organization_slug}/events/?query=error.type:TypeError&statsPeriod=24h" | jq
```

## Investigation Workflow

When investigating a production issue:

1. **Identify the issue** - Get issue ID from URL or search by error message
2. **Get latest event** - Fetch the most recent occurrence with full stack trace
3. **Analyze context** - Look at tags, user info, breadcrumbs, and request data
4. **Find the code** - Use the stack trace to locate the problematic code in the codebase
5. **Correlate** - Check if there are related issues or recent deployments

## Response Format

When reporting findings, structure your response as:

1. **Summary** - One sentence describing the root cause
2. **Impact** - How many users affected, frequency, severity
3. **Stack Trace Analysis** - Key frames and what they reveal
4. **Root Cause** - Technical explanation of why the error occurs
5. **Suggested Fix** - Code changes or configuration needed

## Tips

- The `statsPeriod` parameter accepts: `24h`, `7d`, `14d`, `30d`
- Use `jq` to parse and filter JSON responses
- Stack traces in events are in `exception.values[].stacktrace.frames`
- Breadcrumbs show user actions leading to the error
- Tags like `environment`, `release`, `browser` help narrow down issues
