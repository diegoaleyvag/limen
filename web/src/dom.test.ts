import { describe, expect, it } from "vitest";
import { appendChildren, clearChildren, el, replaceChildren, text } from "./dom";

describe("el", () => {
  it("creates an element with class, attrs, and text children", () => {
    const node = el(
      "button",
      { class: "my-button", attrs: { "aria-label": "Close", "data-id": "42" } },
      ["Close"],
    );
    expect(node.tagName).toBe("BUTTON");
    expect(node.className).toBe("my-button");
    expect(node.getAttribute("aria-label")).toBe("Close");
    expect(node.getAttribute("data-id")).toBe("42");
    expect(node.textContent).toBe("Close");
  });

  it("assigns IDL properties (not just attributes) for things like value/disabled", () => {
    const input = el("input", { type: "text", value: "hello", disabled: true });
    expect(input.value).toBe("hello");
    expect(input.disabled).toBe(true);
  });

  it("applies inline style declarations", () => {
    const node = el("div", { style: { color: "red", fontWeight: "bold" } });
    expect(node.style.color).toBe("red");
    expect(node.style.fontWeight).toBe("bold");
  });

  it("nests real DOM element children, not just strings", () => {
    const child = el("span", {}, ["inner"]);
    const parent = el("div", {}, [child, " and text"]);
    expect(parent.children.length).toBe(1);
    expect(parent.children[0]).toBe(child);
    expect(parent.textContent).toBe("inner and text");
  });

  it("skips null/undefined/false children without throwing", () => {
    const parent = el("div", {}, ["a", null, undefined, false, "b"]);
    expect(parent.textContent).toBe("ab");
    expect(parent.childNodes.length).toBe(2);
  });
});

describe("appendChildren / clearChildren / replaceChildren", () => {
  it("appendChildren appends onto existing content", () => {
    const parent = el("div", {}, ["a"]);
    appendChildren(parent, ["b"]);
    expect(parent.textContent).toBe("ab");
  });

  it("clearChildren removes every child node", () => {
    const parent = el("div", {}, [el("span", {}, ["x"]), "y"]);
    expect(parent.childNodes.length).toBe(2);
    clearChildren(parent);
    expect(parent.childNodes.length).toBe(0);
  });

  it("replaceChildren fully replaces prior content", () => {
    const parent = el("div", {}, ["old content"]);
    replaceChildren(parent, [el("p", {}, ["new content"])]);
    expect(parent.textContent).toBe("new content");
    expect(parent.childNodes.length).toBe(1);
  });
});

describe("text", () => {
  it("creates a real Text node with the given data", () => {
    const node = text("hello");
    expect(node.nodeType).toBe(Node.TEXT_NODE);
    expect(node.data).toBe("hello");
  });
});
