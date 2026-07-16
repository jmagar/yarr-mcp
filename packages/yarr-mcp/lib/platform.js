"use strict";

const path = require("node:path");
const distribution = require("../dist.targets.json");

function packageVersion() {
  return require("../package.json").version;
}

function binaryVersion() {
  return packageVersion();
}

function targetFor(platform = process.platform, arch = process.arch) {
  const target = distribution.targets.find(
    (candidate) => candidate.nodePlatform === platform && candidate.nodeArch === arch,
  );
  if (target) return { asset: target.asset, binary: target.archiveBinary };
  const supported = distribution.targets.map((candidate) => `${candidate.nodePlatform}/${candidate.nodeArch}`).join(", ");
  throw new Error(`Unsupported platform ${platform}/${arch}. Supported targets: ${supported}.`);
}

function releaseVersion(env = process.env) {
  const raw = env.YARR_VERSION || binaryVersion();
  return raw.startsWith("v") ? raw : `v${raw}`;
}

function releaseBaseUrl(env = process.env) {
  const repo = env.YARR_REPO || distribution.identity.canonicalRepo;
  return env.YARR_RELEASE_BASE_URL || `https://github.com/${repo}/releases/download`;
}

function releaseMetadataUrl(env = process.env) {
  if (env.YARR_RELEASE_METADATA_URL) return env.YARR_RELEASE_METADATA_URL;
  const repo = env.YARR_REPO || distribution.identity.canonicalRepo;
  return `https://api.github.com/repos/${repo}/releases/tags/${releaseVersion(env)}`;
}

function downloadUrl(target, env = process.env) {
  return `${releaseBaseUrl(env)}/${releaseVersion(env)}/${target.asset}`;
}

function installRoot(env = process.env) {
  return env.YARR_INSTALL_ROOT
    ? path.resolve(env.YARR_INSTALL_ROOT)
    : path.resolve(__dirname, "..", "vendor");
}

function binaryPath(platform = process.platform, arch = process.arch, env = process.env) {
  if (env.YARR_BIN) return path.resolve(env.YARR_BIN);
  const target = targetFor(platform, arch);
  return path.join(installRoot(env), target.binary);
}

module.exports = {
  binaryVersion,
  binaryPath,
  downloadUrl,
  releaseBaseUrl,
  releaseMetadataUrl,
  installRoot,
  packageVersion,
  releaseVersion,
  targetFor,
};
