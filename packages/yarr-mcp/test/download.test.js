"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");

const { downloadBuffer, downloadToFile } = require("../lib/download");

async function listen(handler) {
  const server = http.createServer(handler);
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  return {
    base: `http://127.0.0.1:${address.port}`,
    close: () => new Promise((resolve) => server.close(resolve)),
  };
}

const testPolicy = {
  allowInsecureLoopback: true,
  connectTimeoutMs: 100,
  totalTimeoutMs: 300,
  maxBytes: 64,
  maxRedirects: 2,
};

test("follows relative redirects within a bound", async () => {
  const fixture = await listen((request, response) => {
    if (request.url === "/start") {
      response.writeHead(302, { location: "./payload" });
      response.end();
      return;
    }
    response.end("verified");
  });
  try {
    assert.equal((await downloadBuffer(`${fixture.base}/start`, testPolicy)).toString(), "verified");
  } finally {
    await fixture.close();
  }
});

test("rejects insecure production URLs", async () => {
  await assert.rejects(downloadBuffer("http://example.test/file"), /HTTPS/);
});

test("rejects redirect cycles", async () => {
  const fixture = await listen((request, response) => {
    response.writeHead(302, { location: request.url === "/a" ? "/b" : "/a" });
    response.end();
  });
  try {
    await assert.rejects(downloadBuffer(`${fixture.base}/a`, testPolicy), /redirect/i);
  } finally {
    await fixture.close();
  }
});

test("rejects oversized and stalled responses", async () => {
  const fixture = await listen((request, response) => {
    if (request.url === "/large") {
      response.end("x".repeat(65));
      return;
    }
    response.write("partial");
  });
  try {
    await assert.rejects(downloadBuffer(`${fixture.base}/large`, testPolicy), /size limit/i);
    await assert.rejects(downloadBuffer(`${fixture.base}/stall`, testPolicy), /timed out/i);
  } finally {
    await fixture.close();
  }
});

test("removes partial files after failed downloads", async () => {
  const fixture = await listen((_request, response) => {
    response.write("x".repeat(65));
    response.destroy();
  });
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "yarr-download-test-"));
  const destination = path.join(root, "asset");
  try {
    await assert.rejects(downloadToFile(`${fixture.base}/asset`, destination, testPolicy));
    assert.equal(fs.existsSync(destination), false);
    assert.equal(fs.existsSync(`${destination}.part`), false);
  } finally {
    await fixture.close();
    fs.rmSync(root, { recursive: true, force: true });
  }
});
