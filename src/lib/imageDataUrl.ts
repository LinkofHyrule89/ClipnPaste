/** Strip a data-URL or return raw base64 if already plain. */
export function stripImageDataUrl(content: string): string | null {
  const trimmed = content.trim();
  if (!trimmed) return null;

  const prefixes = [
    "data:image/png;base64,",
    "data:image/jpeg;base64,",
    "data:image/jpg;base64,",
    "data:image/webp;base64,",
  ];

  for (const prefix of prefixes) {
    if (trimmed.toLowerCase().startsWith(prefix)) {
      const b64 = trimmed.slice(prefix.length).trim();
      return b64.length > 0 ? b64 : null;
    }
  }

  // Canvas toDataURL style: only one comma separating header from payload.
  if (trimmed.startsWith("data:image/") && trimmed.includes(";base64,")) {
    const idx = trimmed.indexOf(";base64,");
    const b64 = trimmed.slice(idx + ";base64,".length).trim();
    return b64.length > 0 ? b64 : null;
  }

  // Plain base64 (no data URL) — used by CaptureResult.pngBase64.
  // No spaces: "hello world" must not match.
  const compact = trimmed.replace(/\s/g, "");
  if (
    compact.length >= 4 &&
    compact.length % 4 === 0 &&
    /^[A-Za-z0-9+/]+={0,2}$/.test(compact)
  ) {
    return compact;
  }

  return null;
}

/** Extract base64 from a canvas `toDataURL("image/png")` result. */
export function base64FromDataUrl(dataUrl: string): string {
  const comma = dataUrl.indexOf(",");
  if (comma < 0) return "";
  return dataUrl.slice(comma + 1);
}
