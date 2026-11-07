import { describe, expect, it } from "vitest";
import { routeKey, shortcutFor, targetKind } from "./shortcuts";
import type { KeyChord, KeyContext, TargetLike } from "./shortcuts";

/** A keystroke with no modifiers, plus whatever the case overrides. */
const press = (key: string, modifiers: Partial<KeyChord> = {}): KeyChord => ({
  key,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  ...modifiers,
});

/** The board on screen, nothing over it, focus on the page itself. */
const BOARD: KeyContext = {
  target: "other",
  startupSheet: false,
  navOpen: false,
  shortcutsOpen: false,
  practising: true,
};
const on = (overrides: Partial<KeyContext>): KeyContext => ({ ...BOARD, ...overrides });

/** What the handler would do — the action, and whether the browser is told no. */
const route = (chord: KeyChord, context: KeyContext = BOARD) => routeKey(chord, context);

const NOTHING = { action: null, preventDefault: false };

/** A stand-in for a DOM element: the four things `targetKind` reads. */
const element = (
  tagName: string,
  attributes: Record<string, string> = {},
  isContentEditable = false,
): TargetLike => ({
  tagName,
  isContentEditable,
  hasAttribute: (name) => name in attributes,
  getAttribute: (name) => attributes[name] ?? null,
});

describe("the board's keys on the board", () => {
  it("checks on Enter, and claims the key", () => {
    expect(route(press("Enter"))).toEqual({ action: "check", preventDefault: true });
  });

  it("moves with the arrows", () => {
    expect(route(press("ArrowLeft"))).toEqual({ action: "previous", preventDefault: true });
    expect(route(press("ArrowRight"))).toEqual({ action: "next", preventDefault: true });
  });

  it("undoes on Backspace and on ⌘Z or ⌃Z", () => {
    expect(route(press("Backspace")).action).toBe("undo");
    expect(route(press("z", { metaKey: true })).action).toBe("undo");
    expect(route(press("Z", { ctrlKey: true })).action).toBe("undo");
    expect(route(press("z")), "a plain z is nothing").toEqual(NOTHING);
  });

  it("takes the letters only plain, leaving ⌘H and the like to the system", () => {
    expect(route(press("s")).action).toBe("strokes");
    expect(route(press("H")).action, "the letter, whatever the case").toBe("hear");
    for (const modifier of ["metaKey", "ctrlKey", "altKey"] as const) {
      expect(route(press("h", { [modifier]: true })), modifier).toEqual(NOTHING);
      expect(route(press("s", { [modifier]: true })), modifier).toEqual(NOTHING);
    }
  });

  it("toggles the key list on ? or a plain /", () => {
    expect(route(press("?")).action).toBe("help");
    expect(route(press("/")).action).toBe("help");
    expect(route(press("/", { metaKey: true }))).toEqual(NOTHING);
  });

  it("leaves a key it has no use for alone", () => {
    expect(route(press("x"))).toEqual(NOTHING);
    expect(route(press(" ")), "Space is no shortcut").toEqual(NOTHING);
    expect(route(press("Escape")), "Escape with nothing open").toEqual(NOTHING);
  });
});

describe("a screen other than the board", () => {
  // Settings, About, the list screens: `practising` is false for all of them.
  const elsewhere = on({ practising: false });

  it("does nothing with any of the board's keys", () => {
    for (const key of ["Enter", "ArrowLeft", "ArrowRight", "Backspace", "s", "h", "?"]) {
      expect(route(press(key), elsewhere), key).toEqual(NOTHING);
    }
    expect(route(press("z", { metaKey: true }), elsewhere)).toEqual(NOTHING);
  });

  it("still lets Escape close a key list left open", () => {
    expect(route(press("Escape"), on({ practising: false, shortcutsOpen: true }))).toEqual({
      action: "close-shortcuts",
      preventDefault: true,
    });
  });
});

describe("a focused control", () => {
  const onButton = on({ target: "control" });

  it("keeps Enter and Space for itself, unclaimed", () => {
    // The button must activate, so the browser's default is left to run.
    expect(route(press("Enter"), onButton)).toEqual(NOTHING);
    expect(route(press(" "), onButton)).toEqual(NOTHING);
  });

  it("does not stop the other board keys", () => {
    expect(route(press("ArrowRight"), onButton).action).toBe("next");
    expect(route(press("Backspace"), onButton).action).toBe("undo");
    expect(route(press("h"), onButton).action).toBe("hear");
  });

  it("does not stop Escape from closing the key list", () => {
    expect(route(press("Escape"), on({ target: "control", shortcutsOpen: true })).action).toBe(
      "close-shortcuts",
    );
  });
});

describe("a text field", () => {
  const inField = on({ target: "text" });

  it("keeps every key, Escape included", () => {
    for (const key of ["Enter", "ArrowLeft", "Backspace", "h", "?", "Escape"]) {
      expect(route(press(key), inField), key).toEqual(NOTHING);
    }
    expect(route(press("Escape"), on({ target: "text", startupSheet: true }))).toEqual(NOTHING);
  });
});

describe("something over the board", () => {
  it("lets only Escape through the startup reading, which it dismisses", () => {
    const sheet = on({ startupSheet: true });
    expect(route(press("Enter"), sheet)).toEqual(NOTHING);
    expect(route(press("s"), sheet)).toEqual(NOTHING);
    expect(route(press("Escape"), sheet)).toEqual({
      action: "finish-startup",
      preventDefault: true,
    });
    // Whatever else is open, the reading is on top.
    expect(
      route(press("Escape"), on({ startupSheet: true, navOpen: true, shortcutsOpen: true })).action,
    ).toBe("finish-startup");
  });

  it("lets only Escape through the phone's navigation sheet, which it closes", () => {
    const sheet = on({ navOpen: true });
    expect(route(press("Enter"), sheet)).toEqual(NOTHING);
    expect(route(press("ArrowLeft"), sheet)).toEqual(NOTHING);
    expect(route(press("Escape"), sheet)).toEqual({ action: "close-nav", preventDefault: true });
    expect(route(press("Escape"), on({ navOpen: true, shortcutsOpen: true })).action).toBe(
      "close-nav",
    );
  });

  it("leaves the key list's own board live — it is a card, not a sheet", () => {
    const card = on({ shortcutsOpen: true });
    expect(route(press("Enter"), card).action).toBe("check");
    expect(route(press("?"), card).action, "the same key closes it").toBe("help");
    expect(route(press("Escape"), card)).toEqual({
      action: "close-shortcuts",
      preventDefault: true,
    });
  });
});

describe("what a keystroke landed on", () => {
  it("calls the typing elements text", () => {
    expect(targetKind(element("INPUT"))).toBe("text");
    expect(targetKind(element("INPUT", { type: "checkbox" }))).toBe("text");
    expect(targetKind(element("TEXTAREA"))).toBe("text");
    expect(targetKind(element("SELECT"))).toBe("text");
    expect(targetKind(element("DIV", {}, true)), "content-editable").toBe("text");
  });

  it("calls anything that activates a control", () => {
    expect(targetKind(element("BUTTON"))).toBe("control");
    expect(targetKind(element("A", { href: "#x" }))).toBe("control");
    expect(targetKind(element("SUMMARY"))).toBe("control");
    expect(targetKind(element("DIV", { role: "button" }))).toBe("control");
    expect(targetKind(element("SPAN", { role: "link" }))).toBe("control");
    expect(targetKind(element("DIV", { tabindex: "0" }))).toBe("control");
  });

  it("calls the page, the board and a merely scriptable focus other", () => {
    expect(targetKind(null)).toBe("other");
    expect(targetKind(element("BODY"))).toBe("other");
    expect(targetKind(element("CANVAS"))).toBe("other");
    expect(targetKind(element("A")), "an anchor with no href is not a link").toBe("other");
    expect(targetKind(element("DIV", { tabindex: "-1" })), "the startup sheet").toBe("other");
    expect(targetKind(element("DIV", { role: "dialog" }))).toBe("other");
  });
});

describe("the lookup the card and the handler share", () => {
  it("still finds a shortcut from a plain object", () => {
    expect(shortcutFor(press("Enter"))?.id).toBe("check");
    expect(shortcutFor(press("q"))).toBeNull();
  });
});
