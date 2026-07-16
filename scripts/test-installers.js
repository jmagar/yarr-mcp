#!/usr/bin/env node
"use strict";

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const { spawn, spawnSync } = require("node:child_process");

const root = path.resolve(__dirname, "..");
const installers = [path.join(root, "install.sh"), path.join(root, "scripts", "install.sh")];

function sha256(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

function archiveFixture(rootDir, member = "yarr") {
  const payload = path.join(rootDir, "payload");
  fs.mkdirSync(payload);
  fs.writeFileSync(path.join(payload, "yarr"), "#!/bin/sh\necho 'yarr 1.1.1'\n", { mode: 0o755 });
  const archive = path.join(rootDir, "asset.tar.gz");
  const args = member === "yarr"
    ? ["-C", payload, "-czf", archive, "yarr"]
    : ["-C", payload, "--transform", `s,^yarr$,${member},`, "-czf", archive, "yarr"];
  const result = spawnSync("tar", args, { encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  return fs.readFileSync(archive);
}

async function fixtureServer(archive, { digest = sha256(archive), checksum = sha256(archive) } = {}) {
  const assetName = "yarr-x86_64.tar.gz";
  const sums = Buffer.from(`${checksum}  ${assetName}\n`);
  const server = http.createServer((request, response) => {
    const origin = `http://127.0.0.1:${server.address().port}`;
    if (request.url === "/release") {
      response.setHeader("content-type", "application/json");
      response.end(JSON.stringify({
        tag_name: "v1.1.1",
        assets: [
          { name: assetName, browser_download_url: `${origin}/${assetName}`, digest: digest && `sha256:${digest}` },
          { name: "SHA256SUMS", browser_download_url: `${origin}/SHA256SUMS`, digest: `sha256:${sha256(sums)}` },
        ],
      }));
      return;
    }
    if (request.url === `/${assetName}`) return response.end(archive);
    if (request.url === "/SHA256SUMS") return response.end(sums);
    response.writeHead(404);
    response.end("missing");
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return {
    metadataUrl: `http://127.0.0.1:${server.address().port}/release`,
    close: () => new Promise((resolve) => server.close(resolve)),
  };
}

function runInstaller(script, temp, metadataUrl) {
  return new Promise((resolve) => {
    const child = spawn("bash", [script], {
      env: {
        ...process.env,
        HOME: temp,
        INSTALL_DIR: path.join(temp, "bin"),
        YARR_MCP_INSTALL_DIR: path.join(temp, "bin"),
        YARR_VERSION: "v1.1.1",
        YARR_MCP_VERSION: "v1.1.1",
        YARR_RELEASE_API_URL: metadataUrl,
        YARR_TEST_ALLOW_HTTP: "1",
      },
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("close", (status, signal) => resolve({ status, signal, stdout, stderr }));
  });
}

async function exercise(script, archive, options, expectedSuccess, expectedError) {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), "yarr-shell-install-"));
  const fixture = await fixtureServer(archive, options);
  try {
    const result = await runInstaller(script, temp, fixture.metadataUrl);
    if (expectedSuccess) {
      assert.equal(result.status, 0, `${script}: ${result.stderr}`);
      const binary = path.join(temp, "bin", "yarr");
      assert.equal(fs.existsSync(binary), true);
      assert.equal(spawnSync(binary, ["--version"], { encoding: "utf8" }).stdout.trim(), "yarr 1.1.1");
    } else {
      assert.notEqual(result.status, 0, `${script}: tampered install succeeded`);
      assert.match(`${result.stdout}${result.stderr}`, expectedError);
      assert.equal(fs.existsSync(path.join(temp, "bin", "yarr")), false);
    }
  } finally {
    await fixture.close();
    fs.rmSync(temp, { recursive: true, force: true });
  }
}

async function main() {
  for (const script of installers) {
    let temp = fs.mkdtempSync(path.join(os.tmpdir(), "yarr-shell-fixture-"));
    await exercise(script, archiveFixture(temp), {}, true);
    fs.rmSync(temp, { recursive: true, force: true });

    temp = fs.mkdtempSync(path.join(os.tmpdir(), "yarr-shell-fixture-"));
    await exercise(script, archiveFixture(temp), { digest: "" }, false, /digest|provenance/i);
    fs.rmSync(temp, { recursive: true, force: true });

    temp = fs.mkdtempSync(path.join(os.tmpdir(), "yarr-shell-fixture-"));
    await exercise(script, archiveFixture(temp), { checksum: "0".repeat(64) }, false, /checksum|digest/i);
    fs.rmSync(temp, { recursive: true, force: true });

    temp = fs.mkdtempSync(path.join(os.tmpdir(), "yarr-shell-fixture-"));
    await exercise(script, archiveFixture(temp, "../yarr"), {}, false, /archive|member|path/i);
    fs.rmSync(temp, { recursive: true, force: true });
  }
  console.log("shell installer contract ok");
}

main().catch((error) => {
  console.error(error.stack || error.message);
  process.exitCode = 1;
});
