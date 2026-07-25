import { describe, expect, it, vi } from "vitest";
import {
  buildPreviewDocument,
  directoryOf,
  isExternalReference,
  resolveAssetPath,
} from "./htmlPreviewModel";

const ROOT = "C:\\ws\\site";

function reader(files: Record<string, string>) {
  return vi.fn(async (path: string) => {
    const bytes = files[path];
    if (bytes === undefined) throw new Error(`not found: ${path}`);
    return btoa(unescape(encodeURIComponent(bytes)));
  });
}

describe("directoryOf", () => {
  it("Windowsパスの親ディレクトリを返す", () => {
    expect(directoryOf("C:\\ws\\site\\index.html")).toBe("C:\\ws\\site");
  });

  it("スラッシュ区切りにも対応する", () => {
    expect(directoryOf("C:/ws/site/index.html")).toBe("C:/ws/site");
  });

  it("区切りがなければ空文字", () => {
    expect(directoryOf("index.html")).toBe("");
  });
});

describe("isExternalReference", () => {
  it.each(["https://example.com/a.css", "http://x/a.png", "//cdn/a.js", "data:image/png;base64,AA", "mailto:a@b.c", "#anchor", "  "])(
    "%s は外部参照",
    (value) => {
      expect(isExternalReference(value)).toBe(true);
    },
  );

  it.each(["style.css", "./img/a.png", "../shared/a.css", "assets/f.woff2"])(
    "%s は相対参照",
    (value) => {
      expect(isExternalReference(value)).toBe(false);
    },
  );
});

describe("resolveAssetPath", () => {
  it("相対パスを絶対パスへ解決する", () => {
    expect(resolveAssetPath(ROOT, "assets/style.css")).toBe("C:\\ws\\site\\assets\\style.css");
  });

  it("./ と ../ を正規化する", () => {
    expect(resolveAssetPath(ROOT, "./a.png")).toBe("C:\\ws\\site\\a.png");
    expect(resolveAssetPath(ROOT, "../shared/a.png")).toBe("C:\\ws\\shared\\a.png");
  });

  it("クエリとフラグメントを取り除く", () => {
    expect(resolveAssetPath(ROOT, "style.css?v=3#x")).toBe("C:\\ws\\site\\style.css");
  });

  it("パーセントエンコードを復号する", () => {
    expect(resolveAssetPath(ROOT, "img/%E5%9B%B3.png")).toBe("C:\\ws\\site\\img\\図.png");
  });

  it("外部参照はnull", () => {
    expect(resolveAssetPath(ROOT, "https://example.com/a.css")).toBeNull();
  });
});

describe("buildPreviewDocument", () => {
  it("linkのCSSを<style>としてインライン化する", async () => {
    const read = reader({ "C:\\ws\\site\\style.css": "body { color: red; }" });
    const result = await buildPreviewDocument(
      `<html><head><link rel="stylesheet" href="style.css"></head><body>hi</body></html>`,
      ROOT,
      read,
    );
    expect(result.document).toContain("<style>body { color: red; }</style>");
    expect(result.document).not.toContain("<link");
    expect(result.missing).toEqual([]);
  });

  it("imgのsrcをdata URLへ置き換える", async () => {
    const read = reader({ "C:\\ws\\site\\img\\logo.png": "PNGDATA" });
    const result = await buildPreviewDocument(
      `<body><img src="img/logo.png"></body>`,
      ROOT,
      read,
    );
    expect(result.document).toContain(`src="data:image/png;base64,${btoa("PNGDATA")}"`);
  });

  it("CSS内のurl()も解決する", async () => {
    const read = reader({
      "C:\\ws\\site\\style.css": ".a { background: url('img/bg.png'); }",
      "C:\\ws\\site\\img\\bg.png": "BG",
    });
    const result = await buildPreviewDocument(
      `<head><link rel="stylesheet" href="style.css"></head>`,
      ROOT,
      read,
    );
    expect(result.document).toContain(`url("data:image/png;base64,${btoa("BG")}")`);
  });

  it("CSSの@importを再帰的に取り込む", async () => {
    const read = reader({
      "C:\\ws\\site\\style.css": `@import "base.css";\n.a { color: red; }`,
      "C:\\ws\\site\\base.css": "html { margin: 0; }",
    });
    const result = await buildPreviewDocument(
      `<head><link rel="stylesheet" href="style.css"></head>`,
      ROOT,
      read,
    );
    expect(result.document).toContain("html { margin: 0; }");
    expect(result.document).not.toContain("@import");
  });

  it("インラインstyle属性のurl()も解決する", async () => {
    const read = reader({ "C:\\ws\\site\\bg.png": "BG" });
    const result = await buildPreviewDocument(
      `<body><div style="background: url(bg.png)"></div></body>`,
      ROOT,
      read,
    );
    expect(result.document).toContain(`data:image/png;base64,${btoa("BG")}`);
  });

  it("scriptは実行させないため取り除く", async () => {
    const read = reader({});
    const result = await buildPreviewDocument(
      `<body><script src="app.js"></script><script>alert(1)</script>ok</body>`,
      ROOT,
      read,
    );
    expect(result.document).not.toContain("<script");
    expect(read).not.toHaveBeenCalled();
  });

  it("外部URLはそのまま残す", async () => {
    const read = reader({});
    const result = await buildPreviewDocument(
      `<body><img src="https://example.com/a.png"></body>`,
      ROOT,
      read,
    );
    expect(result.document).toContain(`src="https://example.com/a.png"`);
  });

  it("読み込めないアセットはmissingへ記録し参照を外す", async () => {
    const read = reader({});
    const result = await buildPreviewDocument(`<body><img src="none.png"></body>`, ROOT, read);
    expect(result.missing).toEqual(["none.png"]);
    expect(result.document).not.toContain("none.png");
  });

  it("同じアセットは一度しか読み込まない", async () => {
    const read = reader({ "C:\\ws\\site\\a.png": "A" });
    await buildPreviewDocument(
      `<body><img src="a.png"><img src="./a.png"></body>`,
      ROOT,
      read,
    );
    expect(read).toHaveBeenCalledTimes(1);
  });

  it("srcsetの候補も解決する", async () => {
    const read = reader({ "C:\\ws\\site\\a.png": "A", "C:\\ws\\site\\a2.png": "B" });
    const result = await buildPreviewDocument(
      `<body><img src="a.png" srcset="a.png 1x, a2.png 2x"></body>`,
      ROOT,
      read,
    );
    expect(result.document).toContain(`data:image/png;base64,${btoa("B")} 2x`);
  });

  it("videoのposterも解決する", async () => {
    const read = reader({ "C:\\ws\\site\\p.jpg": "P" });
    const result = await buildPreviewDocument(
      `<body><video poster="p.jpg"></video></body>`,
      ROOT,
      read,
    );
    expect(result.document).toContain(`data:image/jpeg;base64,${btoa("P")}`);
  });

  it("DOCTYPE付きの完全なHTMLを返す", async () => {
    const result = await buildPreviewDocument(`<body>hi</body>`, ROOT, reader({}));
    expect(result.document.startsWith("<!DOCTYPE html>")).toBe(true);
    expect(result.document).toContain("<html");
  });
});
