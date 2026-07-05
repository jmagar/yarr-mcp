# Bazarr API Endpoints

Base URL: `$BAZARR_URL`. Auth header: `X-API-KEY: $BAZARR_API_KEY`.
Bazarr exposes a REST API under `/api`.

| Purpose | Method | Path |
|---|---|---|
| System status | GET | `/api/system/status` |
| Badges (wanted counts) | GET | `/api/badges` |
| Providers | GET | `/api/providers` |
| Movies missing subtitles | GET | `/api/movies/wanted` |
| Episodes missing subtitles | GET | `/api/episodes/wanted` |
| Search movie subtitles | POST | `/api/providers/movies?radarrid=<id>` |
| Search episode subtitles | POST | `/api/providers/episodes?episodeid=<id>` |

Notes:
- The exact paths AND query parameter names can vary by Bazarr version; confirm with
  `GET /api/system/status` and the in-app API docs if a call 404s.
- Always pass the key via the `X-API-KEY` header, never in the query string.
