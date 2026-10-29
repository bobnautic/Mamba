#!/usr/bin/env node
// The app version is recorded in five files: package.json, package-lock.json,
// src-tauri/Cargo.toml, src-tauri/Cargo.lock and src-tauri/tauri.conf.json
// (Tauri stamps it into the installers, the updater compares it with the
// latest release tag). This script keeps them in step.
//
//   npm run version:set -- 1.2.0     sets 1.2.0 in every file
//   node scripts/set-version.mjs --check
//                                    prints the version, exits 1 when the
//                                    files disagree (CI and release workflow)

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

// X.Y.Z only: the updater (src-tauri/src/update.rs) compares numeric triples
// and the Windows installer needs a numeric product version.
const SEMVER = /^\d+\.\d+\.\d+$/;

// Each entry reads the version(s) a file records and writes a new one,
// touching nothing else in the file.
const FILES = [
  {
    path: "package.json",
    re: /^(\s*"version":\s*")([^"]*)(")/m,
  },
  {
    path: "src-tauri/tauri.conf.json",
    re: /^(\s*"version":\s*")([^"]*)(")/m,
  },
  {
    path: "src-tauri/Cargo.toml",
    // first `version =` at the start of a line: the one of [package]
    re: /^(version\s*=\s*")([^"]*)(")/m,
  },
  {
    path: "src-tauri/Cargo.lock",
    re: /^(name = "mamba"\r?\nversion = ")([^"]*)(")/m,
  },
  {
    path: "package-lock.json",
    json: true,
  },
];

function read(file) {
  const text = readFileSync(join(root, file.path), "utf8");
  if (file.json) {
    const lock = JSON.parse(text);
    return [
      { where: `${file.path} (version)`, version: lock.version },
      { where: `${file.path} (packages[""].version)`, version: lock.packages?.[""]?.version },
    ];
  }
  const m = text.match(file.re);
  if (!m) throw new Error(`${file.path}: version line not found`);
  return [{ where: file.path, version: m[2] }];
}

function write(file, version) {
  const full = join(root, file.path);
  const text = readFileSync(full, "utf8");
  let next;
  if (file.json) {
    // npm writes its lockfile as JSON.stringify(…, null, 2) plus a newline
    const lock = JSON.parse(text);
    lock.version = version;
    lock.packages[""].version = version;
    next = JSON.stringify(lock, null, 2) + "\n";
  } else {
    next = text.replace(file.re, `$1${version}$3`);
  }
  writeFileSync(full, next);
}

const arg = process.argv[2];

if (arg === "--check") {
  const found = FILES.flatMap(read);
  const versions = new Set(found.map((f) => f.version));
  if (versions.size !== 1) {
    console.error("The app version differs between files:");
    for (const f of found) console.error(`  ${f.version ?? "(missing)"}\t${f.where}`);
    console.error("Run `npm run version:set -- X.Y.Z` to align them.");
    process.exit(1);
  }
  const [version] = versions;
  if (!SEMVER.test(version)) {
    console.error(`App version "${version}" is not X.Y.Z.`);
    process.exit(1);
  }
  console.log(version);
} else if (arg) {
  const version = arg.replace(/^v/, "");
  if (!SEMVER.test(version)) {
    console.error(`"${arg}" is not a version of the form X.Y.Z.`);
    process.exit(1);
  }
  for (const file of FILES) write(file, version);
  console.log(`Version set to ${version} in ${FILES.map((f) => f.path).join(", ")}.`);
  console.log(`Next: commit, merge to main, then push the tag v${version} or run the Release workflow.`);
} else {
  console.error("Usage: npm run version:set -- X.Y.Z   |   node scripts/set-version.mjs --check");
  process.exit(1);
}
