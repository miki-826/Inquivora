import { describe, expect, it } from "vitest";
import {
  flattenTree,
  isSameOrDescendant,
  joinPath,
  parentPath,
  type TreeEntry,
} from "./treeModel";

function folder(relativePath: string, hasChildren = true): TreeEntry {
  const name = relativePath.split("/").pop() ?? relativePath;
  return {
    name,
    relativePath,
    isFolder: true,
    hasChildren,
    sizeBytes: 0,
    extension: null,
    category: "unknown",
  };
}

function file(relativePath: string): TreeEntry {
  const name = relativePath.split("/").pop() ?? relativePath;
  return {
    name,
    relativePath,
    isFolder: false,
    hasChildren: false,
    sizeBytes: 1,
    extension: name.split(".").pop() ?? null,
    category: "edit",
  };
}

describe("parentPath / joinPath", () => {
  it("親パスを返す", () => {
    expect(parentPath("docs/sub/a.md")).toBe("docs/sub");
    expect(parentPath("a.md")).toBe("");
    expect(parentPath("")).toBe("");
  });

  it("パスを結合する", () => {
    expect(joinPath("", "a.md")).toBe("a.md");
    expect(joinPath("docs", "sub")).toBe("docs/sub");
  });
});

describe("isSameOrDescendant", () => {
  it("自身と子孫を判定する", () => {
    expect(isSameOrDescendant("docs", "docs")).toBe(true);
    expect(isSameOrDescendant("docs", "docs/sub/a.md")).toBe(true);
    expect(isSameOrDescendant("", "anything")).toBe(true);
  });

  it("前方一致だけの別パスは子孫ではない", () => {
    expect(isSameOrDescendant("docs", "docs2/a.md")).toBe(false);
    expect(isSameOrDescendant("docs/sub", "docs")).toBe(false);
  });
});

describe("flattenTree", () => {
  const children: Record<string, TreeEntry[] | undefined> = {
    "": [folder("docs"), folder("empty", false), file("readme.md")],
    docs: [folder("docs/sub"), file("docs/a.md")],
    "docs/sub": [file("docs/sub/deep.md")],
  };

  it("展開されたフォルダの子だけを深さ付きで返す", () => {
    const rows = flattenTree(children, new Set(["docs"]));
    expect(rows.map((r) => r.entry.relativePath)).toEqual([
      "docs",
      "docs/sub",
      "docs/a.md",
      "empty",
      "readme.md",
    ]);
    expect(rows.map((r) => r.depth)).toEqual([0, 1, 1, 0, 0]);
  });

  it("全展開時は子孫も含む", () => {
    const rows = flattenTree(children, new Set(["docs", "docs/sub"]));
    expect(rows.map((r) => r.entry.relativePath)).toEqual([
      "docs",
      "docs/sub",
      "docs/sub/deep.md",
      "docs/a.md",
      "empty",
      "readme.md",
    ]);
  });

  it("未展開ならルート直下のみ", () => {
    const rows = flattenTree(children, new Set());
    expect(rows.map((r) => r.entry.relativePath)).toEqual(["docs", "empty", "readme.md"]);
  });

  it("展開済みでも未読み込みの子は無視する", () => {
    const rows = flattenTree({ "": [folder("docs")] }, new Set(["docs"]));
    expect(rows.map((r) => r.entry.relativePath)).toEqual(["docs"]);
  });
});
