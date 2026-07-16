#!/usr/bin/env node
"use strict";

const fs = require("node:fs");
const path = require("node:path");

const packageRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(packageRoot, "..", "..");
fs.copyFileSync(path.join(repoRoot, "dist.targets.json"), path.join(packageRoot, "dist.targets.json"));
