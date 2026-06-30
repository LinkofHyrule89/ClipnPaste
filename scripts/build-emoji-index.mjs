#!/usr/bin/env node
/**
 * Builds public/assets/emoji-index.json and copies Fluent UI Emoji flat PNGs.
 * Requires: git, network on first run (clones microsoft/fluentui-emoji).
 */
import fs from "fs";
import path from "path";
import { execSync } from "child_process";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.join(__dirname, "..");
const CACHE = path.join(__dirname, ".cache");
const FLUENT_REPO = path.join(CACHE, "fluentui-emoji");
const OUT_DIR = path.join(ROOT, "public", "assets", "emoji");
const INDEX_PATH = path.join(ROOT, "public", "assets", "emoji-index.json");

const EMOJI_TEST_URL =
  "https://unicode.org/Public/emoji/16.0/emoji-test.txt";
const CLDR_URL =
  "https://raw.githubusercontent.com/unicode-org/cldr-json/main/cldr-json/cldr-annotations-full/annotations/en/annotations.json";

const MAX_EMOJI = 900;

const GROUP_LABELS = {
  "Smileys & Emotion": "Smileys",
  "People & Body": "People",
  "Animals & Nature": "Nature",
  "Food & Drink": "Food",
  "Travel & Places": "Travel",
  "Activities": "Activities",
  "Objects": "Objects",
  "Symbols": "Symbols",
  "Flags": "Flags",
  "Component": "Component",
};

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function download(url, dest) {
  if (fs.existsSync(dest)) return;
  ensureDir(path.dirname(dest));
  execSync(`curl -fsSL "${url}" -o "${dest}"`, { stdio: "inherit" });
}

function ensureFluentRepo() {
  if (fs.existsSync(path.join(FLUENT_REPO, "assets"))) return;
  ensureDir(CACHE);
  console.log("Cloning fluentui-emoji (one-time)…");
  execSync(
    `git clone --depth 1 https://github.com/microsoft/fluentui-emoji.git "${FLUENT_REPO}"`,
    { stdio: "inherit" },
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

function charToHexId(char) {
  return [...char]
    .map((c) => c.codePointAt(0).toString(16))
    .join("-");
}

function parseEmojiTest(text) {
  const entries = [];
  let currentGroup = "Other";
  for (const line of text.split("\n")) {
    if (line.startsWith("# group:")) {
      currentGroup = line.replace("# group:", "").trim();
      continue;
    }
    const match = line.match(
      /^([0-9A-F ]+);\s*fully-qualified\s+#\s+(.+)$/,
    );
    if (!match) continue;
    const [, cps, rest] = match;
    const parts = rest.split(/\s+/);
    const name = parts.slice(2).join(" ").trim();
    if (!name) continue;
    entries.push({
      char: codepointsToChar(cps.trim()),
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

function pickAssetFile(dir, folder) {
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
    const asset = pickAssetFile(path.join(assetsDir, folder), folder);
    if (!asset) continue;
    map.set(normalizeName(folder), asset);
  }
  return map;
}

function findFluentAsset(fluentMap, names) {
  for (const name of names) {
    const key = normalizeName(name);
    const hit = fluentMap.get(key);
    if (hit) return hit;
  }
  return null;
}

function main() {
  ensureFluentRepo();

  const cacheDir = path.join(CACHE, "data");
  ensureDir(cacheDir);
  const emojiTestPath = path.join(cacheDir, "emoji-test.txt");
  const cldrPath = path.join(cacheDir, "cldr-annotations.json");
  download(EMOJI_TEST_URL, emojiTestPath);
  download(CLDR_URL, cldrPath);

  const emojiTest = parseEmojiTest(fs.readFileSync(emojiTestPath, "utf8"));
  const cldrKeywords = loadCldrKeywords(cldrPath);
  const fluentMap = buildFluentMap();

  if (fs.existsSync(OUT_DIR)) {
    fs.rmSync(OUT_DIR, { recursive: true });
  }
  ensureDir(OUT_DIR);

  const categories = new Set();
  const emoji = [];

  for (const entry of emojiTest) {
    if (emoji.length >= MAX_EMOJI) break;
    if (entry.group === "Component") continue;

    const cldr = cldrKeywords.get(entry.char) ?? [];
    const searchNames = [entry.name, ...cldr];
    const srcAsset = findFluentAsset(fluentMap, searchNames);
    if (!srcAsset) continue;

    const id = charToHexId(entry.char);
    const ext = path.extname(srcAsset);
    const destName = `${id}${ext}`;
    fs.copyFileSync(srcAsset, path.join(OUT_DIR, destName));

    const category = GROUP_LABELS[entry.group] ?? entry.group;
    categories.add(category);

    const keywords = [
      entry.name,
      ...cldr,
      category.toLowerCase(),
    ]
      .join(" ")
      .toLowerCase()
      .split(/\s+/)
      .filter(Boolean);

    emoji.push({
      char: entry.char,
      name: entry.name,
      category,
      keywords: [...new Set(keywords)],
      image: `/assets/emoji/${destName}`,
    });
  }

  const index = {
    version: 1,
    categories: [...categories],
    emoji,
  };

  ensureDir(path.dirname(INDEX_PATH));
  fs.writeFileSync(INDEX_PATH, JSON.stringify(index));
  console.log(
    `Wrote ${emoji.length} emoji to ${OUT_DIR} and ${INDEX_PATH}`,
  );
}

main();