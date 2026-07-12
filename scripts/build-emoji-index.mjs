#!/usr/bin/env node
/**
 * Builds public/assets/emoji-index.json and copies Google Noto + Fluent UI
 * emoji assets into public/assets/emoji/{google,fluent}/.
 *
 * Requires: git, network on first run (clones noto-emoji + fluentui-emoji).
 */
import fs from "fs";
import path from "path";
import { execSync } from "child_process";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.join(__dirname, "..");
const CACHE = path.join(__dirname, ".cache");
const FLUENT_REPO = path.join(CACHE, "fluentui-emoji");
const NOTO_REPO = path.join(CACHE, "noto-emoji");
const OUT_DIR = path.join(ROOT, "public", "assets", "emoji");
const GOOGLE_OUT = path.join(OUT_DIR, "google");
const FLUENT_OUT = path.join(OUT_DIR, "fluent");
const INDEX_PATH = path.join(ROOT, "public", "assets", "emoji-index.json");

const EMOJI_TEST_URL =
  "https://unicode.org/Public/emoji/16.0/emoji-test.txt";
const CLDR_URL =
  "https://raw.githubusercontent.com/unicode-org/cldr-json/main/cldr-json/cldr-annotations-full/annotations/en/annotations.json";

/** No practical cap — ship the full fully-qualified Unicode set with assets. */
const MAX_EMOJI = Number.MAX_SAFE_INTEGER;

const GROUP_LABELS = {
  "Smileys & Emotion": "Smileys",
  "People & Body": "People",
  "Animals & Nature": "Nature",
  "Food & Drink": "Food",
  "Travel & Places": "Travel",
  Activities: "Activities",
  Objects: "Objects",
  Symbols: "Symbols",
  Flags: "Flags",
  Component: "Component",
};

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function download(url, dest) {
  if (fs.existsSync(dest)) return;
  ensureDir(path.dirname(dest));
  execSync(`curl -fsSL "${url}" -o "${dest}"`, { stdio: "inherit" });
}

function ensureRepo(dir, url, label) {
  if (fs.existsSync(dir) && fs.readdirSync(dir).length > 0) return;
  ensureDir(CACHE);
  if (fs.existsSync(dir)) fs.rmSync(dir, { recursive: true, force: true });
  console.log(`Cloning ${label} (one-time)…`);
  execSync(`git clone --depth 1 ${url} "${dir}"`, { stdio: "inherit" });
}

function ensureFluentRepo() {
  ensureRepo(
    FLUENT_REPO,
    "https://github.com/microsoft/fluentui-emoji.git",
    "fluentui-emoji",
  );
}

function ensureNotoRepo() {
  ensureRepo(
    NOTO_REPO,
    "https://github.com/googlefonts/noto-emoji.git",
    "noto-emoji",
  );
}

function normalizeName(name) {
  return name
    .toLowerCase()
    .replace(/:/g, "")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

function codepointsToChar(cps) {
  return cps
    .split(/\s+/)
    .map((cp) => String.fromCodePoint(parseInt(cp, 16)))
    .join("");
}

/** Stable id for filenames: lowercase hex codepoints joined by '-' */
function charToHexId(char) {
  return [...char]
    .map((c) => c.codePointAt(0).toString(16))
    .join("-");
}

function cpsFromChar(char) {
  return [...char].map((c) => c.codePointAt(0).toString(16));
}

function parseEmojiTest(text) {
  const entries = [];
  let currentGroup = "Other";
  for (const line of text.split("\n")) {
    if (line.startsWith("# group:")) {
      currentGroup = line.replace("# group:", "").trim();
      continue;
    }
    const match = line.match(/^([0-9A-F ]+);\s*fully-qualified\s+#\s+(.+)$/);
    if (!match) continue;
    const [, cps, rest] = match;
    const parts = rest.split(/\s+/);
    const name = parts.slice(2).join(" ").trim();
    if (!name) continue;
    entries.push({
      char: codepointsToChar(cps.trim()),
      cps: cps
        .trim()
        .split(/\s+/)
        .map((c) => c.toLowerCase()),
      group: currentGroup,
      name,
    });
  }
  return entries;
}

function asStrings(value) {
  if (!value) return [];
  if (Array.isArray(value)) return value.filter((item) => typeof item === "string");
  if (typeof value === "string") return [value];
  return [];
}

function loadCldrKeywords(filePath) {
  const raw = JSON.parse(fs.readFileSync(filePath, "utf8"));
  const annotations = raw.annotations?.annotations ?? {};
  const keywords = new Map();
  for (const [char, data] of Object.entries(annotations)) {
    const terms = [...asStrings(data.default), ...asStrings(data.tts)].map((t) =>
      t.toLowerCase(),
    );
    keywords.set(char, [...new Set(terms)]);
  }
  return keywords;
}

function pickFluentAsset(dir) {
  const flatDir = path.join(dir, "Flat");
  if (fs.existsSync(flatDir)) {
    const flatSvg = fs
      .readdirSync(flatDir)
      .find((f) => f.endsWith("_flat.svg"));
    if (flatSvg) return path.join(flatDir, flatSvg);
  }
  const threeDDir = path.join(dir, "3D");
  if (fs.existsSync(threeDDir)) {
    const png = fs.readdirSync(threeDDir).find((f) => f.endsWith("_3d.png"));
    if (png) return path.join(threeDDir, png);
  }
  return null;
}

function buildFluentMap() {
  const assetsDir = path.join(FLUENT_REPO, "assets");
  const map = new Map();
  for (const folder of fs.readdirSync(assetsDir)) {
    const asset = pickFluentAsset(path.join(assetsDir, folder));
    if (!asset) continue;
    map.set(normalizeName(folder), asset);
  }
  return map;
}

function findFluentAsset(fluentMap, names) {
  for (const name of names) {
    const hit = fluentMap.get(normalizeName(name));
    if (hit) return hit;
  }
  return null;
}

/** Map lowercase "1f600_1f3fb" style keys → absolute path */
function buildNotoMap() {
  const svgDir = path.join(NOTO_REPO, "svg");
  const pngDir = path.join(NOTO_REPO, "png", "128");
  const map = new Map();

  if (fs.existsSync(svgDir)) {
    for (const file of fs.readdirSync(svgDir)) {
      if (!file.startsWith("emoji_u") || !file.endsWith(".svg")) continue;
      const key = file.slice("emoji_u".length, -".svg".length);
      map.set(key, path.join(svgDir, file));
    }
  }
  if (fs.existsSync(pngDir)) {
    for (const file of fs.readdirSync(pngDir)) {
      if (!file.startsWith("emoji_u") || !file.endsWith(".png")) continue;
      const key = file.slice("emoji_u".length, -".png".length);
      if (!map.has(key)) map.set(key, path.join(pngDir, file));
    }
  }
  return map;
}

function notoKeyCandidates(cps) {
  const full = cps.map((c) => c.toLowerCase());
  const noFe0f = full.filter((c) => c !== "fe0f");
  const keys = [];
  const push = (arr) => {
    if (arr.length) keys.push(arr.join("_"));
  };
  push(full);
  push(noFe0f);
  // Some Noto assets drop VS16 and keep ZWJ sequence order as-is
  if (full.length !== noFe0f.length) {
    push(noFe0f.filter((c) => c !== "200d").length ? noFe0f : full);
  }
  return [...new Set(keys)];
}

function findNotoAsset(notoMap, cps) {
  for (const key of notoKeyCandidates(cps)) {
    const hit = notoMap.get(key);
    if (hit) return hit;
  }
  return null;
}

function main() {
  ensureFluentRepo();
  ensureNotoRepo();

  const cacheDir = path.join(CACHE, "data");
  ensureDir(cacheDir);
  const emojiTestPath = path.join(cacheDir, "emoji-test.txt");
  const cldrPath = path.join(cacheDir, "cldr-annotations.json");
  download(EMOJI_TEST_URL, emojiTestPath);
  download(CLDR_URL, cldrPath);

  const emojiTest = parseEmojiTest(fs.readFileSync(emojiTestPath, "utf8"));
  const cldrKeywords = loadCldrKeywords(cldrPath);
  const fluentMap = buildFluentMap();
  const notoMap = buildNotoMap();

  if (fs.existsSync(OUT_DIR)) {
    fs.rmSync(OUT_DIR, { recursive: true });
  }
  ensureDir(GOOGLE_OUT);
  ensureDir(FLUENT_OUT);

  const categories = new Set();
  const emoji = [];
  let googleCount = 0;
  let fluentCount = 0;

  for (const entry of emojiTest) {
    if (emoji.length >= MAX_EMOJI) break;
    if (entry.group === "Component") continue;

    const cldr = cldrKeywords.get(entry.char) ?? [];
    const searchNames = [entry.name, ...cldr];
    const id = charToHexId(entry.char);
    const cps = entry.cps?.length ? entry.cps : cpsFromChar(entry.char);

    const notoSrc = findNotoAsset(notoMap, cps);
    const fluentSrc = findFluentAsset(fluentMap, searchNames);
    if (!notoSrc && !fluentSrc) continue;

    if (notoSrc) {
      const ext = path.extname(notoSrc);
      fs.copyFileSync(notoSrc, path.join(GOOGLE_OUT, `${id}${ext}`));
      googleCount++;
    }
    if (fluentSrc) {
      const ext = path.extname(fluentSrc);
      fs.copyFileSync(fluentSrc, path.join(FLUENT_OUT, `${id}${ext}`));
      fluentCount++;
    }

    const category = GROUP_LABELS[entry.group] ?? entry.group;
    categories.add(category);

    const keywords = [entry.name, ...cldr, category.toLowerCase()]
      .join(" ")
      .toLowerCase()
      .split(/\s+/)
      .filter(Boolean);

    emoji.push({
      char: entry.char,
      name: entry.name,
      category,
      keywords: [...new Set(keywords)],
      id,
      packs: {
        google: Boolean(notoSrc),
        fluent: Boolean(fluentSrc),
      },
    });
  }

  const index = {
    version: 2,
    categories: [...categories],
    packs: ["google", "fluent"],
    emoji,
  };

  ensureDir(path.dirname(INDEX_PATH));
  fs.writeFileSync(INDEX_PATH, JSON.stringify(index));
  console.log(
    `Wrote ${emoji.length} emoji (google files: ${googleCount}, fluent files: ${fluentCount}) to ${OUT_DIR}`,
  );
  console.log(`Index: ${INDEX_PATH}`);
}

main();
