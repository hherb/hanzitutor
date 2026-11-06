import { describe, expect, it } from "vitest";

import { unwrittenLabel } from "./due";

describe("unwrittenLabel", () => {
  it("names a traced character as traced, not as untouched", () => {
    expect(unwrittenLabel(10)).toBe("traced 10× · not yet written from memory");
  });

  it("says a character never traced has not been written from memory", () => {
    expect(unwrittenLabel(undefined)).toBe("not yet written from memory");
    expect(unwrittenLabel(0)).toBe("not yet written from memory");
  });
});
