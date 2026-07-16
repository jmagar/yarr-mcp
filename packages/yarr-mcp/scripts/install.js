#!/usr/bin/env node
"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const { downloadBuffer, downloadToFile } = require("../lib/download");
const { binaryPath, installRoot, releaseMetadataUrl, releaseVersion, targetFor } = require("../lib/platform");

const MAX_ARCHIVE_BYTES = 200 * 1024 * 1024;
const MAX_METADATA_BYTES = 2 * 1024 * 1024;

function log(message) {
  process.stderr.write(`yarr: ${message}\n`);
}

function sha256(input) {
  const hash = crypto.createHash("sha256");
  hash.update(Buffer.isBuffer(input) ? input : fs.readFileSync(input));
  return hash.digest("hex");
}

function parseDigest(value, label) {
  const match = /^sha256:([a-f0-9]{64})$/i.exec(value || "");
  if (!match) throw new Error(`${label} is missing a valid GitHub release SHA-256 provenance digest`);
  return match[1].toLowerCase();
}

function checksumFromText(text, asset) {
  for (const line of text.split(/\r?\n/)) {
    const match = /^([a-f0-9]{64})\s+\*?(.+?)\s*$/i.exec(line);
    if (match && path.basename(match[2]) === asset) return match[1].toLowerCase();
  }
  throw new Error(`checksum manifest does not contain a SHA-256 entry for ${asset}`);
}

function verify(actual, expected, label) {
  if (actual !== expected) throw new Error(`${label} checksum mismatch: expected ${expected}, got ${actual}`);
}

function runTar(args) {
  const result = spawnSync("tar", args, { encoding: "utf8" });
  if (result.status !== 0) throw new Error((result.stderr || result.stdout || "tar failed").trim());
  return result.stdout;
}

function validateArchive(archive, expectedBinary) {
  const entries = runTar(["-tzf", archive]).split(/\r?\n/).filter(Boolean);
  if (entries.length !== 1) throw new Error(`archive must contain exactly one file; found ${entries.length}`);
  const raw = entries[0];
  if (raw.includes("\\") || raw.startsWith("/") || raw.split("/").includes("..")) {
    throw new Error(`unsafe archive member path: ${raw}`);
  }
  const normalized = raw.replace(/^\.\//, "");
  if (normalized !== expectedBinary || normalized.includes("/")) {
    throw new Error(`unexpected archive member: ${raw}`);
  }
  const listing = runTar(["-tvzf", archive]);
  if (!listing.startsWith("-")) throw new Error(`archive member ${raw} is not a regular file`);
}

function extractValidated(archive, root, expectedBinary) {
  validateArchive(archive, expectedBinary);
  const staging = fs.mkdtempSync(path.join(path.dirname(root), ".yarr-extract-"));
  try {
    runTar(["--no-same-owner", "--no-same-permissions", "-xzf", archive, "-C", staging]);
    const source = path.join(staging, expectedBinary);
    const stat = fs.lstatSync(source);
    if (!stat.isFile() || stat.isSymbolicLink()) throw new Error("extracted yarr binary is not a regular file");
    fs.mkdirSync(root, { recursive: true });
    const destination = path.join(root, expectedBinary);
    fs.rmSync(destination, { force: true });
    fs.renameSync(source, destination);
    fs.chmodSync(destination, 0o755);
  } finally {
    fs.rmSync(staging, { recursive: true, force: true });
  }
}

function verifyRuntimeVersion(destination) {
  const expected = `yarr ${require("../package.json").version}`;
  const result = spawnSync(destination, ["--version"], { encoding: "utf8", timeout: 10_000 });
  const actual = (result.stdout || "").trim();
  if (result.error || result.status !== 0 || actual !== expected) {
    fs.rmSync(destination, { force: true });
    throw new Error(`runtime version mismatch: expected ${expected}, got ${actual || result.error?.message || "execution failure"}`);
  }
}

function downloadPolicy(maxBytes) {
  const allowTestHttp = process.env.NODE_ENV === "test" && process.env.YARR_TEST_ALLOW_HTTP === "1";
  return {
    allowInsecureLoopback: allowTestHttp,
    allowedHosts: ["api.github.com", "github.com", "objects.githubusercontent.com", "release-assets.githubusercontent.com"],
    connectTimeoutMs: Number(process.env.YARR_DOWNLOAD_CONNECT_TIMEOUT_MS || 10_000),
    totalTimeoutMs: Number(process.env.YARR_DOWNLOAD_TIMEOUT_MS || 60_000),
    maxBytes,
    maxRedirects: 5,
  };
}

async function releaseMetadata() {
  const url = releaseMetadataUrl();
  const body = await downloadBuffer(url, downloadPolicy(MAX_METADATA_BYTES));
  let metadata;
  try {
    metadata = JSON.parse(body.toString("utf8"));
  } catch (error) {
    throw new Error(`invalid release metadata from ${url}: ${error.message}`);
  }
  if (metadata.tag_name !== releaseVersion()) {
    throw new Error(`release metadata tag ${metadata.tag_name || "<missing>"} does not match ${releaseVersion()}`);
  }
  return { metadata, url };
}

function assetByName(metadata, name) {
  const asset = (metadata.assets || []).find((candidate) => candidate.name === name);
  if (!asset || !asset.browser_download_url) throw new Error(`release metadata is missing asset ${name}`);
  return asset;
}

async function install() {
  if (process.env.YARR_SKIP_DOWNLOAD === "1") {
    log("skipping binary download because YARR_SKIP_DOWNLOAD=1");
    return;
  }

  const target = targetFor();
  const root = installRoot();
  const destination = binaryPath();
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "yarr-mcp-install-"));
  const archive = path.join(tempDir, target.asset);
  try {
    const { metadata, url: metadataUrl } = await releaseMetadata();
    const archiveAsset = assetByName(metadata, target.asset);
    const sumsAsset = assetByName(metadata, "SHA256SUMS");
    const provenanceDigest = parseDigest(archiveAsset.digest, target.asset);
    const sumsDigest = parseDigest(sumsAsset.digest, "SHA256SUMS");

    const sums = await downloadBuffer(sumsAsset.browser_download_url, downloadPolicy(MAX_METADATA_BYTES));
    verify(sha256(sums), sumsDigest, "SHA256SUMS provenance");
    const manifestDigest = checksumFromText(sums.toString("utf8"), target.asset);
    if (manifestDigest !== provenanceDigest) {
      throw new Error(`${target.asset} checksum disagrees with GitHub release provenance digest`);
    }

    log(`downloading verified ${archiveAsset.browser_download_url}`);
    await downloadToFile(archiveAsset.browser_download_url, archive, downloadPolicy(MAX_ARCHIVE_BYTES));
    const archiveDigest = sha256(archive);
    verify(archiveDigest, provenanceDigest, `${target.asset} provenance`);
    verify(archiveDigest, manifestDigest, target.asset);
    extractValidated(archive, root, target.binary);
    verifyRuntimeVersion(destination);
    fs.writeFileSync(path.join(root, ".yarr-install.json"), `${JSON.stringify({
      packageVersion: require("../package.json").version,
      runtimeVersion: releaseVersion().replace(/^v/, ""),
      target: `${process.platform}/${process.arch}`,
      asset: target.asset,
      sha256: archiveDigest,
      metadataUrl,
    }, null, 2)}\n`, { mode: 0o600 });
    log(`installed ${destination}`);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

if (require.main === module) {
  install().catch((error) => {
    log(error.message);
    process.exitCode = 1;
  });
}

module.exports = { checksumFromText, extractValidated, install, parseDigest, validateArchive, verifyRuntimeVersion };
