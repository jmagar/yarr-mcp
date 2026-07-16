# Bazarr Troubleshooting

## 401 / 403 Unauthorized
- Confirm `BAZARR_API_KEY` matches Settings -> General -> Security -> API Key.
- The key is sent as the `X-API-KEY` header. Do not append it to the URL.

## Connection refused / timeout
- Verify `BAZARR_URL` (default port 6767) and that Bazarr is reachable from this host.
- Check the JSON settings file exists: `ls -la ~/.config/lab-bazarr/config.json`

## A call returns 404
- Endpoint paths/params vary across Bazarr versions. Re-check against the
  in-app API documentation (Settings shows the API), then adjust the path.

## Re-generate credentials
- Update the bazarr plugin settings and restart the session so the hook
  rewrites `~/.config/lab-bazarr/config.json`.
