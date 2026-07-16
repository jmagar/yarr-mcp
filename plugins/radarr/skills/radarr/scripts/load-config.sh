#!/usr/bin/env bash
# Parse a plugin-generated JSON settings file without evaluating its contents.
load_plugin_config() {
  local config_file="${1:?config path required}"
  shift
  if [[ ! -f "${config_file}" ]]; then
    printf 'ERROR: plugin config not found: %s\n' "${config_file}" >&2
    return 1
  fi
  command -v node >/dev/null 2>&1 || {
    printf 'ERROR: node is required to parse plugin config safely\n' >&2
    return 1
  }
  local key value
  while IFS= read -r -d '' key && IFS= read -r -d '' value; do
    [[ "${key}" =~ ^[A-Z][A-Z0-9_]*$ ]] || {
      printf 'ERROR: invalid plugin config key: %s\n' "${key}" >&2
      return 1
    }
    declare -gx "${key}=${value}"
  done < <(node - "${config_file}" "$@" <<'NODE'
const fs = require("node:fs");
const [file, ...allowedNames] = process.argv.slice(2);
const allowed = new Set(allowedNames);
const parsed = JSON.parse(fs.readFileSync(file, "utf8"));
if (!parsed || Array.isArray(parsed) || typeof parsed !== "object") {
  throw new Error("plugin config must be a JSON object");
}
for (const [key, value] of Object.entries(parsed)) {
  if (!allowed.has(key) || typeof value !== "string") continue;
  process.stdout.write(key + "\0" + value + "\0");
}
NODE
  )
}
