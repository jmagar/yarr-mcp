#!/usr/bin/env node
"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const { spawn } = require("node:child_process");

const root = path.resolve(__dirname, "..");
const clients = {
  sonarr: { script: "sonarr.sh", args: ["search", "fixture"], config: { SONARR_URL: "$URL", SONARR_API_KEY: "key" } },
  radarr: { script: "radarr.sh", args: ["search", "fixture"], config: { RADARR_URL: "$URL", RADARR_API_KEY: "key" } },
  prowlarr: { script: "prowlarr-api.sh", args: ["status"], config: { PROWLARR_URL: "$URL", PROWLARR_API_KEY: "key" } },
  sabnzbd: { script: "sab-api.sh", args: ["status"], config: { SABNZBD_URL: "$URL", SABNZBD_API_KEY: "key" } },
  qbittorrent: { script: "qbit-api.sh", args: ["version"], config: { QBITTORRENT_URL: "$URL", QBITTORRENT_USERNAME: "user", QBITTORRENT_PASSWORD: "pass" } },
  plex: { script: "plex-api.sh", args: ["info"], config: { PLEX_URL: "$URL", PLEX_TOKEN: "token" } },
  jellyfin: { script: "jellyfin-api.sh", args: ["info"], config: { JELLYFIN_URL: "$URL", JELLYFIN_API_KEY: "key" } },
  tautulli: { script: "tautulli-api.sh", args: ["server-info"], config: { TAUTULLI_URL: "$URL", TAUTULLI_API_KEY: "key" } },
  tracearr: { script: "tracearr-api.sh", args: ["health"], config: { TRACEARR_URL: "$URL" } },
  bazarr: { script: "bazarr-api.sh", args: ["status"], config: { BAZARR_URL: "$URL", BAZARR_API_KEY: "key" } },
};

function run(command, args, env) {
  return new Promise((resolve) => {
    const child = spawn(command, args, { env, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("close", (status, signal) => resolve({ status, signal, stdout, stderr }));
  });
}

async function main() {
  const server = http.createServer((_request, response) => {
    response.writeHead(503, { "content-type": "application/json" });
    response.end('{"error":"fixture-body"}');
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const url = `http://127.0.0.1:${server.address().port}`;
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "yarr-plugin-http-"));
  try {
    for (const [service, client] of Object.entries(clients)) {
      const configDir = path.join(temp, "config", `lab-${service}`);
      fs.mkdirSync(configDir, { recursive: true });
      const config = Object.fromEntries(Object.entries(client.config).map(([key, value]) => [key, value === "$URL" ? url : value]));
      fs.writeFileSync(path.join(configDir, "config.json"), JSON.stringify(config), { mode: 0o600 });
      const script = path.join(root, "plugins", service, "skills", service, "scripts", client.script);
      const source = fs.readFileSync(script, "utf8");
      assert.match(source, /curl\(\)[\s\S]*--fail-with-body[\s\S]*--connect-timeout[\s\S]*--max-time/, `${service}: hardened curl helper`);
      const result = await run("bash", [script, ...client.args], {
        ...process.env,
        HOME: temp,
        XDG_CONFIG_HOME: path.join(temp, "config"),
        YARR_CURL_CONNECT_TIMEOUT: "1",
        YARR_CURL_MAX_TIME: "2",
      });
      assert.notEqual(result.status, 0, `${service}: must not report success for HTTP 503: ${result.stdout}`);
      const output = `${result.stdout}${result.stderr}`;
      assert.match(output, /503|fixture-body|HTTP error|failed/i, `${service}: must expose the HTTP failure: ${JSON.stringify(output)}`);
    }
  } finally {
    await new Promise((resolve) => server.close(resolve));
    fs.rmSync(temp, { recursive: true, force: true });
  }
  console.log("plugin HTTP failure contract ok");
}

main().catch((error) => {
  console.error(error.stack || error.message);
  process.exitCode = 1;
});
