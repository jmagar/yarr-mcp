"use strict";

const fs = require("node:fs");
const http = require("node:http");
const https = require("node:https");

const REDIRECT_CODES = new Set([301, 302, 303, 307, 308]);
const DEFAULTS = Object.freeze({
  connectTimeoutMs: 10_000,
  totalTimeoutMs: 60_000,
  maxBytes: 2 * 1024 * 1024,
  maxRedirects: 5,
  allowInsecureLoopback: false,
  allowedHosts: null,
});

function isLoopback(hostname) {
  return hostname === "127.0.0.1" || hostname === "::1" || hostname === "localhost";
}

function validatedUrl(raw, policy, previous) {
  const url = new URL(raw, previous);
  const insecureTestUrl = policy.allowInsecureLoopback && url.protocol === "http:" && isLoopback(url.hostname);
  if (url.protocol !== "https:" && !insecureTestUrl) {
    throw new Error(`download URL must use HTTPS: ${url}`);
  }
  if (policy.allowedHosts && !policy.allowedHosts.includes(url.hostname) && !insecureTestUrl) {
    throw new Error(`download redirect host is not allowlisted: ${url.hostname}`);
  }
  return url;
}

function requestBuffer(url, policy, state) {
  return new Promise((resolve, reject) => {
    let settled = false;
    const fail = (error) => {
      if (settled) return;
      settled = true;
      reject(error);
    };
    const remaining = state.deadline - Date.now();
    if (remaining <= 0) {
      fail(new Error(`download timed out after ${policy.totalTimeoutMs}ms`));
      return;
    }

    const client = url.protocol === "http:" ? http : https;
    const request = client.get(url, {
      headers: { "user-agent": "yarr-mcp-installer", accept: "application/octet-stream, application/json" },
    }, (response) => {
      if (REDIRECT_CODES.has(response.statusCode)) {
        response.resume();
        const location = response.headers.location;
        if (!location) {
          fail(new Error(`redirect from ${url} did not include Location`));
          return;
        }
        if (state.redirects >= policy.maxRedirects) {
          fail(new Error(`redirect limit exceeded for ${url}`));
          return;
        }
        let next;
        try {
          next = validatedUrl(location, policy, url);
        } catch (error) {
          fail(error);
          return;
        }
        const key = next.toString();
        if (state.visited.has(key)) {
          fail(new Error(`redirect cycle detected at ${next}`));
          return;
        }
        state.visited.add(key);
        requestBuffer(next, policy, { ...state, redirects: state.redirects + 1 }).then(resolve, fail);
        return;
      }

      if (response.statusCode < 200 || response.statusCode >= 300) {
        response.resume();
        fail(new Error(`download failed (${response.statusCode}) from ${url}`));
        return;
      }

      const declared = Number(response.headers["content-length"] || 0);
      if (declared > policy.maxBytes) {
        response.destroy();
        fail(new Error(`download size limit exceeded (${declared} > ${policy.maxBytes})`));
        return;
      }

      const chunks = [];
      let bytes = 0;
      response.on("data", (chunk) => {
        bytes += chunk.length;
        if (bytes > policy.maxBytes) {
          response.destroy(new Error(`download size limit exceeded (${bytes} > ${policy.maxBytes})`));
          return;
        }
        chunks.push(chunk);
      });
      response.on("error", fail);
      response.on("end", () => {
        if (settled) return;
        settled = true;
        resolve(Buffer.concat(chunks, bytes));
      });
    });

    request.setTimeout(Math.min(policy.connectTimeoutMs, remaining), () => {
      request.destroy(new Error(`download timed out connecting to ${url}`));
    });
    request.on("error", fail);
  });
}

async function downloadBuffer(rawUrl, options = {}) {
  const policy = { ...DEFAULTS, ...options };
  const url = validatedUrl(rawUrl, policy);
  const state = {
    deadline: Date.now() + policy.totalTimeoutMs,
    redirects: 0,
    visited: new Set([url.toString()]),
  };
  const timeout = new Promise((_, reject) => {
    const timer = setTimeout(() => reject(new Error(`download timed out after ${policy.totalTimeoutMs}ms`)), policy.totalTimeoutMs);
    timer.unref();
  });
  return Promise.race([requestBuffer(url, policy, state), timeout]);
}

async function downloadToFile(url, destination, options = {}) {
  const partial = `${destination}.part`;
  fs.rmSync(partial, { force: true });
  try {
    const body = await downloadBuffer(url, options);
    fs.writeFileSync(partial, body, { mode: 0o600 });
    fs.renameSync(partial, destination);
  } catch (error) {
    fs.rmSync(partial, { force: true });
    fs.rmSync(destination, { force: true });
    throw error;
  }
}

module.exports = { downloadBuffer, downloadToFile, validatedUrl };
