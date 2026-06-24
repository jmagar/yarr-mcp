---
name: tautulli
description: This skill should be used when the user asks about Plex watch statistics, current streams, who is watching Plex, active sessions, playback history, most-watched content, user activity, library stats, stream analytics, or anything related to Tautulli monitoring and analytics.
---

# Tautulli Analytics Skill

Monitor and analyze Plex Media Server usage through Tautulli's comprehensive analytics API. Track current streams, historical playback data, user activity, and library statistics.

## Purpose

This skill provides **read-only** access to Tautulli analytics:
- Monitor current activity and active streams
- View playback history with detailed filtering
- Track user statistics and viewing patterns
- Analyze library statistics and popular content
- View recently added media with metadata
- Monitor concurrent stream limits and bandwidth
- Analyze usage by time, platform, and stream type
- Track server and library performance metrics

All operations are **GET-only** and safe for monitoring and analytics.

**Note:** This skill complements the `plex` skill by adding analytics and historical data that Plex Media Server doesn't expose directly.

## Setup

Credentials are configured in the **plugin settings** (userConfig). A `SessionStart` hook writes them to `~/.config/lab-tautulli/config.env`, which the scripts load automatically — no manual file editing. Variables used:

```bash
# Tautulli Analytics
TAUTULLI_URL="http://localhost:8181"
TAUTULLI_API_KEY="<your_tautulli_api_key>"
```

- `TAUTULLI_URL`: Your Tautulli server URL with port (default: 8181)
- `TAUTULLI_API_KEY`: Your Tautulli API key

**Getting your API key:**
1. Open Tautulli web UI
2. Go to Settings → Web Interface → API
3. Enable "API enabled"
4. Copy the API key
5. Optionally set API HTTP Basic Authentication if desired

## Commands

All commands use the `tautulli-api.sh` wrapper script and return JSON output.

The helper script is located at: `scripts/tautulli-api.sh`

### Server Information

Get server identity and version:

```bash
./scripts/tautulli-api.sh server-info
```

### Current Activity

Monitor active streams and current playback:

```bash
# All active sessions
./scripts/tautulli-api.sh activity

# Activity with session details
./scripts/tautulli-api.sh activity --details
```

**Returns:** Current streams with user, media, player, bandwidth, transcode info

### Playback History

View historical playback data:

```bash
# Recent history (default: 25 items)
./scripts/tautulli-api.sh history

# History with filters
./scripts/tautulli-api.sh history --user "username" --limit 50
./scripts/tautulli-api.sh history --days 7 --media-type movie
./scripts/tautulli-api.sh history --section-id 1 --limit 100

# Search history
./scripts/tautulli-api.sh history --search "Inception"
```

**Parameters:**
- `--user <username>`: Filter by username
- `--section-id <id>`: Filter by library section
- `--media-type <type>`: Filter by movie, episode, track, etc.
- `--days <n>`: History from last N days
- `--limit <n>`: Maximum results (default: 25)
- `--search <query>`: Search in titles

### User Statistics

Track user activity and viewing patterns:

```bash
# All users watch stats
./scripts/tautulli-api.sh user-stats

# Specific user details
./scripts/tautulli-api.sh user-stats --user "username"

# Top users by play count
./scripts/tautulli-api.sh user-stats --sort-by plays --limit 10
```

**Parameters:**
- `--user <username>`: Specific user statistics
- `--sort-by <metric>`: passed straight through as Tautulli's `order_column`; the values it accepts depend on your Tautulli version (commonly `plays`, `duration`, `last_seen`)
- `--limit <n>`: Maximum results
- `--days <n>`: Stats from last N days

### Library Statistics

Analyze library usage and popular content:

```bash
# All library sections
./scripts/tautulli-api.sh libraries

# Specific library stats
./scripts/tautulli-api.sh library-stats --section-id 1

# Popular content in library
./scripts/tautulli-api.sh popular --section-id 1 --limit 10
./scripts/tautulli-api.sh popular --media-type movie --days 30
```

**Parameters:**
- `--section-id <id>`: Specific library section
- `--media-type <type>`: Filter by type (movie, show, artist)
- `--days <n>`: Timeframe for popularity
- `--limit <n>`: Maximum results

### Recently Added

View recently added media with rich metadata:

```bash
# Recently added (default: 25 items)
./scripts/tautulli-api.sh recent

# Recent with filters
./scripts/tautulli-api.sh recent --section-id 1 --limit 50
./scripts/tautulli-api.sh recent --media-type movie --days 7
```

### Home Statistics

Get homepage dashboard statistics:

```bash
# Overview stats (most popular, most active, etc.)
./scripts/tautulli-api.sh home-stats

# Stats for specific timeframe
./scripts/tautulli-api.sh home-stats --days 30
```

### Stream Analytics

Analyze stream types and platform usage:

```bash
# Plays by stream type (direct/transcode)
./scripts/tautulli-api.sh plays-by-stream --days 30

# Plays by platform
./scripts/tautulli-api.sh plays-by-platform --days 30

# Plays by date/time
./scripts/tautulli-api.sh plays-by-date --days 30
./scripts/tautulli-api.sh plays-by-hour --days 7
./scripts/tautulli-api.sh plays-by-day --days 30
```

### Concurrent Streams

Monitor concurrent stream patterns:

```bash
# Concurrent stream history
./scripts/tautulli-api.sh concurrent-streams --days 30

# Peak concurrent streams (approximated via Tautulli's plays-per-month concurrent axis)
./scripts/tautulli-api.sh concurrent-streams --days 7 --peak
```

### Media Metadata

Get detailed metadata for specific media:

```bash
# By rating key
./scripts/tautulli-api.sh metadata --rating-key 12345

# By GUID
./scripts/tautulli-api.sh metadata --guid "plex://movie/5d776..."
```

## Workflow

When the user asks about Plex analytics:

1. **"Who's watching right now?"** → Run `activity`
2. **"What are the most watched movies?"** → Run `popular --media-type movie --days 30`
3. **"Show me recent watch history"** → Run `history --limit 25`
4. **"How much has [user] watched this week?"** → Run `user-stats --user "username" --days 7`
5. **"What's new in my library?"** → Run `recent --limit 10`
6. **"When do people watch most?"** → Run `plays-by-hour --days 30`
7. **"Are we hitting stream limits?"** → Run `concurrent-streams --days 7 --peak`

### Activity Monitoring Flow

1. Check current activity for active streams
2. If issues detected (buffering, transcoding), investigate specific session
3. Review user's watch history to understand patterns
4. Check library statistics to identify popular content
5. Analyze stream types to optimize server settings

### Analytics Flow

1. Get home statistics for overview
2. Drill into specific libraries with library-stats
3. Identify popular content with popular command
4. Analyze user behavior with user-stats
5. Review temporal patterns with plays-by-hour/date/day
6. Monitor platform distribution with plays-by-platform

## Output Format

All commands return JSON with standard Tautulli response structure:

```json
{
  "response": {
    "result": "success",
    "message": null,
    "data": { ... }
  }
}
```

Use `jq` to extract and format data:

```bash
# Get just the data
./scripts/tautulli-api.sh activity | jq '.response.data'

# Extract specific fields
./scripts/tautulli-api.sh history | jq '.response.data.data[] | {user: .friendly_name, title: .full_title, date: .date}'
```

## Notes

- Requires network access to your Tautulli server
- All operations are **read-only GET requests**
- Tautulli must be connected to your Plex Media Server
- Library section IDs match Plex library section keys
- Historical data depends on Tautulli's configured retention period
- Some statistics require sufficient historical data to be meaningful
- Response times may vary based on database size and query complexity
- Rating keys are Plex's unique identifiers for media items
- User-friendly names are shown by default (can show usernames with flags)

## Integration with Plex Skill

This skill complements the `plex` skill:

- **Plex skill**: Real-time server state (libraries, search, sessions)
- **Tautulli skill**: Historical analytics (trends, statistics, watch history)

Use both together:
1. Find content with `plex` skill search
2. Check popularity with `tautulli` skill analytics
3. Monitor current playback with either skill
4. Analyze viewing patterns with `tautulli` skill

## Multiple Servers

The plugin supports one Tautulli instance configured via plugin settings. To monitor a different Tautulli server, update the URL and API key in the plugin settings.

## Reference

- [Tautulli API Documentation](https://github.com/Tautulli/Tautulli/wiki/Tautulli-API-Reference)
- [Tautulli GitHub](https://github.com/Tautulli/Tautulli)
- [Tautulli Homepage](https://tautulli.com)

For detailed API reference, see:
- **[API Endpoints](./references/api-endpoints.md)** - Complete endpoint reference with parameters
- **[Quick Reference](./references/quick-reference.md)** - Common operations with copy-paste examples
- **[Troubleshooting](./references/troubleshooting.md)** - Authentication, connection, and error solutions

---
