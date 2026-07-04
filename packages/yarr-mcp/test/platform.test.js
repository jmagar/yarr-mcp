"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const {
  downloadUrl,
  releaseVersion,
  targetFor,
} = require("../lib/platform");

test("maps supported platforms to release assets", () => {
  assert.deepEqual(targetFor("linux", "x64"), {
    asset: "rustarr-x86_64.tar.gz",
    binary: "rustarr",
  });
  assert.deepEqual(targetFor("win32", "x64"), {
    asset: "rustarr-windows-x86_64.tar.gz",
    binary: "rustarr.exe",
  });
});

test("rejects unsupported platforms", () => {
  assert.throws(() => targetFor("darwin", "arm64"), /Unsupported platform/);
});

test("uses npm package version as the binary tag by default", () => {
  assert.equal(releaseVersion({}), "v0.4.0");
});

test("allows release tag override", () => {
  const env = { YARR_BINARY_VERSION: "v9.9.9", YARR_RELEASE_BASE_URL: "https://example.test/releases" };
  assert.equal(downloadUrl(targetFor("linux", "x64"), env), "https://example.test/releases/v9.9.9/rustarr-x86_64.tar.gz");
});
