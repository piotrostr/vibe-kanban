---
name: linear
description: Interact with Linear issues - update status, add comments, create issues, and link work. Use when users want to update tickets, mark issues done, create new issues, or need information from Linear.
---

# Linear Issue Management

You have access to the Linear GraphQL API via the `LINEAR_API_KEY` environment variable. Use curl to interact with Linear.

## API Basics

Linear uses GraphQL. All requests go to `https://api.linear.app/graphql`.

```bash
curl -s -X POST \
  -H "Authorization: $LINEAR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "YOUR_GRAPHQL_QUERY"}' \
  https://api.linear.app/graphql | jq
```

## Common Operations

### Get Current User and Teams

```bash
curl -s -X POST \
  -H "Authorization: $LINEAR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ viewer { id name email } teams { nodes { id name key } } }"}' \
  https://api.linear.app/graphql | jq
```

### Search Issues

```bash
curl -s -X POST \
  -H "Authorization: $LINEAR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ issues(filter: { state: { type: { in: [\"started\", \"unstarted\"] } } }, first: 20) { nodes { id identifier title state { name } priority assignee { name } } } }"}' \
  https://api.linear.app/graphql | jq
```

### Get Issue by Identifier (e.g., ENG-123)

```bash
curl -s -X POST \
  -H "Authorization: $LINEAR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ issue(id: \"ISSUE_UUID\") { id identifier title description state { name } priority labels { nodes { name } } comments { nodes { body user { name } createdAt } } } }"}' \
  https://api.linear.app/graphql | jq
```

### Search by Identifier

```bash
curl -s -X POST \
  -H "Authorization: $LINEAR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ issueSearch(query: \"ENG-123\", first: 1) { nodes { id identifier title description state { name } } } }"}' \
  https://api.linear.app/graphql | jq
```

### Update Issue State

First, get workflow states for the team:
```bash
curl -s -X POST \
  -H "Authorization: $LINEAR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ workflowStates(filter: { team: { key: { eq: \"TEAM_KEY\" } } }) { nodes { id name type } } }"}' \
  https://api.linear.app/graphql | jq
```

Then update the issue:
```bash
curl -s -X POST \
  -H "Authorization: $LINEAR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "mutation { issueUpdate(id: \"ISSUE_UUID\", input: { stateId: \"STATE_UUID\" }) { success issue { identifier state { name } } } }"}' \
  https://api.linear.app/graphql | jq
```

### Add Comment to Issue

```bash
curl -s -X POST \
  -H "Authorization: $LINEAR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "mutation { commentCreate(input: { issueId: \"ISSUE_UUID\", body: \"Your comment here\" }) { success comment { id body } } }"}' \
  https://api.linear.app/graphql | jq
```

### Create New Issue

```bash
curl -s -X POST \
  -H "Authorization: $LINEAR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"query": "mutation { issueCreate(input: { teamId: \"TEAM_UUID\", title: \"Issue title\", description: \"Description here\", priority: 2 }) { success issue { id identifier title url } } }"}' \
  https://api.linear.app/graphql | jq
```

## Workflow

1. **Find the issue** - Search by identifier (ENG-123) or keywords
2. **Get context** - Read description, comments, and current state
3. **Take action** - Update state, add comments, or link related issues
4. **Confirm** - Verify the update was successful

## Priority Levels

- 0: No priority
- 1: Urgent
- 2: High
- 3: Medium
- 4: Low

## State Types

- `backlog` - Not yet started
- `unstarted` - Ready to start
- `started` - In progress
- `completed` - Done
- `canceled` - Won't do

## Tips

- Issue identifiers like `ENG-123` need to be searched first to get the UUID
- Use `issueSearch` with the identifier string to find issues by their human-readable ID
- GraphQL responses are nested - use jq paths like `.data.issue.title`
- When updating, check for `success: true` in the mutation response
