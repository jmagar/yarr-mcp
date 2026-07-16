"use strict";

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const { spawn, spawnSync } = require("node:child_process");
const test = require("node:test");

const packageRoot = path.resolve(__dirname, "..");
const installer = path.join(packageRoot, "scripts", "install.js");
const packageVersion = require("../package.json").version;

function sha256(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

function archiveFixture(root, member = "yarr", versionOutput = `yarr ${packageVersion}`) {
  const payload = path.join(root, "payload");
  fs.mkdirSync(payload);
  fs.writeFileSync(path.join(payload, "yarr"), `#!/bin/sh\necho '${versionOutput}'\n`, { mode: 0o755 });
  const archive = path.join(root, "yarr-x86_64.tar.gz");
  const args = member === "yarr"
    ? ["-C", payload, "-czf", archive, "yarr"]
    : ["-C", payload, "--transform", `s,^yarr$,${member},`, "-czf", archive, "yarr"];
  const result = spawnSync("tar", args, { encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  return fs.readFileSync(archive);
}

async function releaseServer(archive, { digest = sha256(archive), checksum = sha256(archive) } = {}) {
  const asset = "yarr-x86_64.tar.gz";
  const checksumBody = Buffer.from(`${checksum}  ${asset}\n`);
  const server = http.createServer((request, response) => {
    const origin = `http://127.0.0.1:${server.address().port}`;
    if (request.url === "/metadata") {
      response.setHeader("content-type", "application/json");
      response.end(JSON.stringify({
        tag_name: `v${packageVersion}`,
        assets: [
          { name: asset, browser_download_url: `${origin}/assets/${asset}`, digest: digest && `sha256:${digest}` },
          { name: "SHA256SUMS", browser_download_url: `${origin}/assets/SHA256SUMS`, digest: `sha256:${sha256(checksumBody)}` },
        ],
      }));
      return;
    }
    if (request.url === `/assets/${asset}`) {
      response.end(archive);
      return;
    }
    if (request.url === "/assets/SHA256SUMS") {
      response.end(checksumBody);
      return;
    }
    response.writeHead(404);
    response.end("missing");
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return {
    metadata: `http://127.0.0.1:${server.address().port}/metadata`,
    close: () => new Promise((resolve) => server.close(resolve)),
  };
}

function runInstaller(root, metadata) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [installer], {
      cwd: packageRoot,
      env: {
        ...process.env,
        NODE_ENV: "test",
        YARR_TEST_ALLOW_HTTP: "1",
        YARR_RELEASE_METADATA_URL: metadata,
        YARR_INSTALL_ROOT: path.join(root, "vendor"),
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

test("postinstall downloads, independently verifies, validates, and extracts a release", async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "yarr-install-test-"));
  const fixture = await releaseServer(archiveFixture(root));
  try {
    const result = await runInstaller(root, fixture.metadata);
    assert.equal(result.status, 0, result.stderr);
    const installed = path.join(root, "vendor", "yarr");
    assert.equal(fs.existsSync(installed), true);
    assert.equal(fs.statSync(installed).mode & 0o777, 0o755);
    const provenance = JSON.parse(fs.readFileSync(path.join(root, "vendor", ".yarr-install.json"), "utf8"));
    assert.equal(provenance.packageVersion, packageVersion);
    assert.equal(provenance.runtimeVersion, packageVersion);
    assert.match(provenance.sha256, /^[a-f0-9]{64}$/);
  } finally {
    await fixture.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("postinstall fails closed on missing provenance, tampering, and unsafe members", async (t) => {
  const cases = [
    ["missing digest", { digest: "" }, "yarr", /digest|provenance/i],
    ["checksum mismatch", { checksum: "0".repeat(64) }, "yarr", /checksum/i],
    ["path traversal", {}, "../yarr", /archive|member|path/i],
  ];
  for (const [name, options, member, expected] of cases) {
    await t.test(name, async () => {
      const root = fs.mkdtempSync(path.join(os.tmpdir(), "yarr-install-fail-"));
      const fixture = await releaseServer(archiveFixture(root, member), options);
      try {
        const result = await runInstaller(root, fixture.metadata);
        assert.notEqual(result.status, 0);
        assert.match(`${result.stderr}${result.stdout}`, expected);
        assert.equal(fs.existsSync(path.join(root, "vendor", "yarr")), false);
      } finally {
        await fixture.close();
        fs.rmSync(root, { recursive: true, force: true });
      }
    });
  }
});

test("postinstall rejects a runtime whose executable version disagrees with the package", async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "yarr-install-version-"));
  const fixture = await releaseServer(archiveFixture(root, "yarr", "yarr 0.0.0"));
  try {
    const result = await runInstaller(root, fixture.metadata);
    assert.notEqual(result.status, 0);
    assert.match(`${result.stderr}${result.stdout}`, /runtime version/i);
    assert.equal(fs.existsSync(path.join(root, "vendor", "yarr")), false);
  } finally {
    await fixture.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
