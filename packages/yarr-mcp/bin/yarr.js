#!/usr/bin/env node
"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { spawn } = require("node:child_process");
const { binaryPath } = require("../lib/platform");

function fail(message) {
  process.stderr.write(`yarr: ${message}\n`);
  process.exitCode = 1;
}

async function ensureBinary() {
  const binary = binaryPath();
  if (fs.existsSync(binary)) return binary;
  const installer = path.resolve(__dirname, "..", "scripts", "install.js");
  const child = spawn(process.execPath, [installer], { stdio: "inherit" });
  const result = await new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", (code, signal) => resolve({ code, signal }));
  });
  if (result.signal || result.code !== 0 || !fs.existsSync(binary)) {
    throw new Error("binary is not installed; postinstall may have failed");
  }
  return binary;
}

async function main() {
  const binary = await ensureBinary();
  const child = spawn(binary, process.argv.slice(2), { stdio: "inherit", windowsHide: false });
  let forwarded = false;
  const forward = (signal) => {
    if (forwarded || child.killed) return;
    forwarded = true;
    child.kill(signal);
  };
  const onSigint = () => forward("SIGINT");
  const onSigterm = () => forward("SIGTERM");
  process.once("SIGINT", onSigint);
  process.once("SIGTERM", onSigterm);

  const result = await new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", (code, signal) => resolve({ code, signal }));
  });
  process.removeListener("SIGINT", onSigint);
  process.removeListener("SIGTERM", onSigterm);
  if (result.signal) {
    process.kill(process.pid, result.signal);
    return;
  }
  process.exitCode = result.code ?? 1;
}

main().catch((error) => fail(error.message));
