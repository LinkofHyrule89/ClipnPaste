import { describe, expect, it } from "vitest";
import { base64FromDataUrl, stripImageDataUrl } from "./imageDataUrl";

describe("stripImageDataUrl", () => {
  it("strips png data URL", () => {
    expect(stripImageDataUrl("data:image/png;base64,aGk=")).toBe("aGk=");
  });

  it("strips jpeg data URL case-insensitively", () => {
    expect(stripImageDataUrl("DATA:IMAGE/JPEG;BASE64,abc")).toBe("abc");
  });

  it("accepts plain base64", () => {
    expect(stripImageDataUrl("aGk=")).toBe("aGk=");
  });

  it("returns null for empty or non-image", () => {
    expect(stripImageDataUrl("")).toBeNull();
    expect(stripImageDataUrl("   ")).toBeNull();
    expect(stripImageDataUrl("hello world")).toBeNull();
    expect(stripImageDataUrl("data:text/plain;base64,aGk=")).toBeNull();
  });

  it("returns null for empty payload after prefix", () => {
    expect(stripImageDataUrl("data:image/png;base64,")).toBeNull();
  });
});

describe("base64FromDataUrl", () => {
  it("splits on first comma", () => {
    expect(base64FromDataUrl("data:image/png;base64,AAA")).toBe("AAA");
  });

  it("returns empty when no comma", () => {
    expect(base64FromDataUrl("not-a-data-url")).toBe("");
  });
});
