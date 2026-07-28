import { afterEach, describe, expect, it } from "vitest";
import { stripFindWidgetTooltips, watchFindWidgetTooltips } from "./findWidgetTooltips";

let root: HTMLElement | null = null;

function mount(html: string): HTMLElement {
  root = document.createElement("div");
  root.innerHTML = html;
  document.body.append(root);
  return root;
}

afterEach(() => {
  root?.remove();
  root = null;
});

describe("stripFindWidgetTooltips", () => {
  it("検索バー内のtitleを外しaria-labelへ移す", () => {
    const container = mount(
      `<div class="find-widget"><div class="button" title="Close"></div></div>`,
    );
    stripFindWidgetTooltips(container);
    const button = container.querySelector(".button")!;
    expect(button.hasAttribute("title")).toBe(false);
    expect(button.getAttribute("aria-label")).toBe("Close");
  });

  it("既にaria-labelがあれば上書きしない", () => {
    const container = mount(
      `<div class="find-widget"><div class="button" title="Close" aria-label="閉じる"></div></div>`,
    );
    stripFindWidgetTooltips(container);
    expect(container.querySelector(".button")!.getAttribute("aria-label")).toBe("閉じる");
  });

  it("検索バーの外のtitleは残す", () => {
    const container = mount(
      `<div><span id="outside" title="パス"></span><div class="find-widget"></div></div>`,
    );
    stripFindWidgetTooltips(container);
    expect(container.querySelector("#outside")!.getAttribute("title")).toBe("パス");
  });

  it("空のtitleはaria-labelを作らない", () => {
    const container = mount(`<div class="find-widget"><div class="button" title=""></div></div>`);
    stripFindWidgetTooltips(container);
    const button = container.querySelector(".button")!;
    expect(button.hasAttribute("title")).toBe(false);
    expect(button.hasAttribute("aria-label")).toBe(false);
  });
});

describe("watchFindWidgetTooltips", () => {
  it("後から差し込まれた検索バーにも適用する", async () => {
    const container = mount("<div></div>");
    const dispose = watchFindWidgetTooltips(container);
    const widget = document.createElement("div");
    widget.className = "find-widget";
    widget.innerHTML = `<div class="button" title="Previous match"></div>`;
    container.append(widget);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(container.querySelector(".button")!.hasAttribute("title")).toBe(false);
    dispose();
  });

  it("破棄後は監視しない", async () => {
    const container = mount("<div></div>");
    watchFindWidgetTooltips(container)();
    const widget = document.createElement("div");
    widget.className = "find-widget";
    widget.innerHTML = `<div class="button" title="Next match"></div>`;
    container.append(widget);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(container.querySelector(".button")!.getAttribute("title")).toBe("Next match");
  });
});
