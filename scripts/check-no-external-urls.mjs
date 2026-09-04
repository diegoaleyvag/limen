#!/usr/bin/env node
// CI's final "offline claim" gate: greps the built `web/dist` for any URL literal that would
// actually cause the browser to contact a non-relative, non-same-origin host, and fails loudly if
// it finds one. This is the concrete, automated proof behind "no runtime network dependency" for
// the *shipped app* (distinct from CI's own dependency-fetching, which happens earlier and is
// explicitly allowed network access -- see `.github/workflows/ci.yml`'s comments).
//
// Usage: node scripts/check-no-external-urls.mjs [dist-dir]  (defaults to web/dist)
//
// What counts as a real violation: an absolute `http://`/`https://` URL, or a protocol-relative
// `//host/...` URL, whose host is not in the small documented allowlist below.
//
// What does NOT count (deliberately excluded, so this stays a precise check rather than a noisy
// one):
// - Relative paths (`/assets/...`, `./foo`, `../foo`) -- same-origin by construction.
// - `data:`/`blob:` URIs -- never a network request.
// - XML/SVG namespace URIs (e.g. `http://www.w3.org/2000/svg`, `http://www.w3.org/1999/xlink`) --
//   these are inert identifier strings the XML/SVG spec requires verbatim, never dereferenced or
//   fetched by a browser.
// - A relative `//# sourceMappingURL=...` comment referencing a same-directory `.map` file --
//   the leading `//` here is a JS line-comment marker, not a protocol-relative URL; this script
//   only flags `//` immediately followed by a plausible hostname (letters/digits/dots ending in a
//   multi-letter TLD before the next path separator), which a bare `//# ...` comment never is.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { extname, join } from "node:path";

const XML_NAMESPACE_ALLOWLIST = new Set([
  "http://www.w3.org/2000/svg",
  "http://www.w3.org/1999/xlink",
  "http://www.w3.org/XML/1998/namespace",
  "http://www.w3.org/2000/xmlns/",
]);

// User-initiated footer link to the public repository. Not fetched at load time; excluded from
// the offline-runtime claim the same way XML namespace URIs are (inert until clicked).
const PUBLIC_SOURCE_LINK_ALLOWLIST = new Set(["https://github.com/diegoaleyvag/limen"]);

const SCANNED_EXTENSIONS = new Set([".html", ".js", ".css", ".mjs", ".json", ".map"]);

const ABSOLUTE_URL_RE = /\bhttps?:\/\/[^\s"'<>)\\]+/g;
// Protocol-relative: `//` directly followed by a hostname-shaped run (letters/digits/dots/hyphens)
// that contains at least one dot and ends in a 2+ letter TLD before a path separator, quote, or
// whitespace. This deliberately does not match a bare `//` comment marker (e.g. `//#
// sourceMappingURL=...`), which is never followed by a dotted hostname shape, nor the `//` inside
// an already-matched `http://`/`https://` URL (the `(?<!:)` lookbehind excludes that case, since
// this pattern would otherwise separately re-match that URL's own `//host` portion).
const PROTOCOL_RELATIVE_RE = /(?<!:)\/\/([a-z0-9-]+\.)+[a-z]{2,}(?=[/"'\s<>)\\]|$)/gi;

function collectFiles(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    const stats = statSync(path);
    if (stats.isDirectory()) {
      out.push(...collectFiles(path));
    } else if (SCANNED_EXTENSIONS.has(extname(entry))) {
      out.push(path);
    }
  }
  return out;
}

function normalizeMatchedUrl(raw) {
  // Minified bundles can glue template-literal delimiters or attribute separators onto URLs.
  return raw.replace(/[),.;`]+$/, "").split(/[`'"<>\s,]/)[0];
}

function isAllowlistedExternalUrl(url) {
  const normalized = normalizeMatchedUrl(url);
  if (PUBLIC_SOURCE_LINK_ALLOWLIST.has(normalized)) return true;
  for (const allowed of PUBLIC_SOURCE_LINK_ALLOWLIST) {
    if (normalized.startsWith(`${allowed}/`)) return true;
  }
  return false;
}

function findViolations(filePath) {
  const text = readFileSync(filePath, "utf-8");
  const violations = [];

  for (const match of text.matchAll(ABSOLUTE_URL_RE)) {
    const url = normalizeMatchedUrl(match[0]);
    let host;
    try {
      host = new URL(url).origin + "/";
    } catch {
      host = url;
    }
    if (
      XML_NAMESPACE_ALLOWLIST.has(url) ||
      XML_NAMESPACE_ALLOWLIST.has(host) ||
      isAllowlistedExternalUrl(url)
    ) {
      continue;
    }
    violations.push({ url, index: match.index });
  }

  for (const match of text.matchAll(PROTOCOL_RELATIVE_RE)) {
    violations.push({ url: match[0], index: match.index });
  }

  return violations.map(({ url, index }) => {
    const line = text.slice(0, index).split("\n").length;
    return { url, line };
  });
}

function main() {
  const distDir = process.argv[2] ?? "web/dist";
  let files;
  try {
    files = collectFiles(distDir);
  } catch (err) {
    console.error(`Could not read dist directory '${distDir}': ${err.message}`);
    process.exitCode = 1;
    return;
  }

  if (files.length === 0) {
    console.error(`No files found under '${distDir}' -- did the build run first?`);
    process.exitCode = 1;
    return;
  }

  let totalViolations = 0;
  for (const file of files) {
    const violations = findViolations(file);
    for (const { url, line } of violations) {
      console.error(`${file}:${line}: external URL reference: ${url}`);
      totalViolations += 1;
    }
  }

  if (totalViolations > 0) {
    console.error(
      `\nFound ${totalViolations} external URL reference(s) in '${distDir}'. The shipped app must be fully offline -- remove or relativize these, or add a documented, justified allowlist entry if this is a genuine false positive (e.g. an XML namespace URI).`,
    );
    process.exitCode = 1;
    return;
  }

  console.log(`OK: scanned ${files.length} file(s) under '${distDir}', found zero external URL references.`);
}

main();
