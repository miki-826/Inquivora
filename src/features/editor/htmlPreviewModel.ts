/**
 * HTMLファイルを「見たまま」表示するために、ワークスペース内の相対参照
 * （CSS・画像・フォント等）を data URL へ差し替えた1枚のHTMLを組み立てる。
 * スクリプトは実行させないため取り除く（表示先のiframeもsandboxで無効化する）。
 */

/** パスからbase64を取得する関数（Tauriのfile_read_base64を想定） */
export type AssetReader = (absolutePath: string) => Promise<string>;

export type PreviewDocument = {
  document: string;
  /** 読み込めなかった参照（元の記述のまま） */
  missing: string[];
};

const MIME_BY_EXTENSION: Record<string, string> = {
  css: "text/css",
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  jpe: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  avif: "image/avif",
  bmp: "image/bmp",
  ico: "image/x-icon",
  svg: "image/svg+xml",
  woff: "font/woff",
  woff2: "font/woff2",
  ttf: "font/ttf",
  otf: "font/otf",
  eot: "application/vnd.ms-fontobject",
  mp3: "audio/mpeg",
  wav: "audio/wav",
  m4a: "audio/mp4",
  ogg: "audio/ogg",
  mp4: "video/mp4",
  webm: "video/webm",
  pdf: "application/pdf",
};

const EXTERNAL_REFERENCE = /^(?:[a-z][a-z0-9+.-]*:|\/\/)/i;
const CSS_URL = /url\(\s*(['"]?)([^'")]+)\1\s*\)/gi;
const CSS_IMPORT = /@import\s+(?:url\(\s*(['"]?)([^'")]+)\1\s*\)|(['"])([^'"]+)\3)\s*;?/gi;
const MAX_IMPORT_DEPTH = 8;

/** src属性で参照されうる要素 */
const SRC_SELECTOR = "img[src], source[src], video[src], audio[src], embed[src], iframe[src]";

/** ファイルパスの親ディレクトリを返す（区切り文字は元の表記を保つ） */
export function directoryOf(filePath: string): string {
  const index = Math.max(filePath.lastIndexOf("\\"), filePath.lastIndexOf("/"));
  return index === -1 ? "" : filePath.slice(0, index);
}

/** アプリで解決できない参照（絶対URL・アンカー・空）かどうか */
export function isExternalReference(reference: string): boolean {
  const trimmed = reference.trim();
  if (trimmed === "" || trimmed.startsWith("#")) return true;
  return EXTERNAL_REFERENCE.test(trimmed);
}

/** 相対参照をワークスペース内の絶対パス（Windows表記）へ解決する */
export function resolveAssetPath(baseDir: string, reference: string): string | null {
  if (isExternalReference(reference)) return null;
  const withoutFragment = reference.trim().split("#")[0].split("?")[0];
  if (withoutFragment === "") return null;
  let decoded = withoutFragment;
  try {
    decoded = decodeURIComponent(withoutFragment);
  } catch {
    // 不正なエスケープはそのまま扱う
  }
  const segments = `${baseDir}/${decoded}`.replace(/\\/g, "/").split("/");
  const stack: string[] = [];
  for (const segment of segments) {
    if (segment === ".") continue;
    if (segment === "..") {
      stack.pop();
      continue;
    }
    if (segment === "" && stack.length > 0) continue;
    stack.push(segment);
  }
  return stack.join("\\");
}

function mimeForPath(path: string): string {
  const extension = path.split(".").pop()?.toLowerCase() ?? "";
  return MIME_BY_EXTENSION[extension] ?? "application/octet-stream";
}

function decodeBase64Text(base64: string): string {
  const binary = atob(base64);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder("utf-8").decode(bytes);
}

/** 参照の解決とキャッシュ・欠落記録をまとめて扱う */
class AssetResolver {
  private readonly cache = new Map<string, Promise<string>>();
  readonly missing: string[] = [];

  constructor(
    private readonly read: AssetReader,
    private readonly baseDir: string,
  ) {}

  private load(absolutePath: string): Promise<string> {
    const cached = this.cache.get(absolutePath);
    if (cached) return cached;
    const pending = this.read(absolutePath);
    this.cache.set(absolutePath, pending);
    return pending;
  }

  private recordMissing(reference: string): void {
    if (!this.missing.includes(reference)) this.missing.push(reference);
  }

  /** 参照をdata URLへ変換する。外部参照は元のまま、失敗時はnull */
  async toDataUrl(reference: string, fromDir = this.baseDir): Promise<string | null> {
    const absolutePath = resolveAssetPath(fromDir, reference);
    if (absolutePath === null) return reference;
    try {
      const base64 = await this.load(absolutePath);
      return `data:${mimeForPath(absolutePath)};base64,${base64}`;
    } catch {
      this.recordMissing(reference);
      return null;
    }
  }

  /** CSSテキストを読み込み、@importとurl()を解決した内容を返す */
  async loadCss(reference: string, fromDir: string, depth: number): Promise<string | null> {
    const absolutePath = resolveAssetPath(fromDir, reference);
    if (absolutePath === null) return null;
    try {
      const base64 = await this.load(absolutePath);
      return await this.inlineCss(decodeBase64Text(base64), directoryOf(absolutePath), depth);
    } catch {
      this.recordMissing(reference);
      return null;
    }
  }

  /** CSS内の@importを展開し、url()をdata URLへ置き換える */
  async inlineCss(css: string, fromDir: string, depth = 0): Promise<string> {
    let result = css;
    if (depth < MAX_IMPORT_DEPTH) {
      result = await replaceAsync(result, CSS_IMPORT, async (_match, _q1, urlRef, _q2, plainRef) => {
        const reference = urlRef ?? plainRef ?? "";
        if (isExternalReference(reference)) return "";
        const imported = await this.loadCss(reference, fromDir, depth + 1);
        return imported ?? "";
      });
    }
    return replaceAsync(result, CSS_URL, async (match, _quote, reference: string) => {
      if (isExternalReference(reference)) return match;
      const dataUrl = await this.toDataUrl(reference, fromDir);
      return dataUrl === null ? match : `url("${dataUrl}")`;
    });
  }

  /** srcset の各候補を解決する */
  async inlineSrcset(value: string): Promise<string> {
    const candidates = await Promise.all(
      value
        .split(",")
        .map((candidate) => candidate.trim())
        .filter(Boolean)
        .map(async (candidate) => {
          const [reference, ...descriptors] = candidate.split(/\s+/);
          const dataUrl = await this.toDataUrl(reference);
          if (dataUrl === null) return null;
          return [dataUrl, ...descriptors].join(" ");
        }),
    );
    return candidates.filter((candidate): candidate is string => candidate !== null).join(", ");
  }
}

/** 非同期置換（String.prototype.replaceは非同期関数を扱えないため） */
async function replaceAsync(
  input: string,
  pattern: RegExp,
  replacer: (...args: string[]) => Promise<string>,
): Promise<string> {
  const replacements: Promise<string>[] = [];
  input.replace(pattern, (...args: unknown[]) => {
    replacements.push(replacer(...(args.slice(0, -2) as string[])));
    return "";
  });
  const resolved = await Promise.all(replacements);
  let index = 0;
  return input.replace(pattern, () => resolved[index++]);
}

async function inlineElementAttribute(
  element: Element,
  attribute: string,
  resolver: AssetResolver,
): Promise<void> {
  const value = element.getAttribute(attribute);
  if (value === null || isExternalReference(value)) return;
  const dataUrl = await resolver.toDataUrl(value);
  if (dataUrl === null) {
    element.removeAttribute(attribute);
    return;
  }
  element.setAttribute(attribute, dataUrl);
}

/**
 * HTML本文と基準ディレクトリから、iframeのsrcdocへ渡せる自己完結HTMLを作る。
 * @param html 対象ファイルの内容
 * @param baseDir 対象ファイルのあるディレクトリ
 * @param read 絶対パスからbase64を得る関数
 */
export async function buildPreviewDocument(
  html: string,
  baseDir: string,
  read: AssetReader,
): Promise<PreviewDocument> {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const resolver = new AssetResolver(read, baseDir);

  doc.querySelectorAll("script").forEach((element) => element.remove());

  const stylesheets = Array.from(doc.querySelectorAll("link[rel~='stylesheet'][href]"));
  await Promise.all(
    stylesheets.map(async (link) => {
      const href = link.getAttribute("href") ?? "";
      if (isExternalReference(href)) return;
      const css = await resolver.loadCss(href, baseDir, 0);
      if (css === null) {
        link.remove();
        return;
      }
      const style = doc.createElement("style");
      style.textContent = css;
      link.replaceWith(style);
    }),
  );

  // favicon等の残りのlinkは表示に不要なうえ解決できないため取り除く
  doc.querySelectorAll("link[href]").forEach((link) => {
    if (!isExternalReference(link.getAttribute("href") ?? "")) link.remove();
  });

  await Promise.all(
    Array.from(doc.querySelectorAll("style")).map(async (style) => {
      style.textContent = await resolver.inlineCss(style.textContent ?? "", baseDir);
    }),
  );

  await Promise.all([
    ...Array.from(doc.querySelectorAll(SRC_SELECTOR)).map((element) =>
      inlineElementAttribute(element, "src", resolver),
    ),
    ...Array.from(doc.querySelectorAll("video[poster]")).map((element) =>
      inlineElementAttribute(element, "poster", resolver),
    ),
    ...Array.from(doc.querySelectorAll("object[data]")).map((element) =>
      inlineElementAttribute(element, "data", resolver),
    ),
    ...Array.from(doc.querySelectorAll("[srcset]")).map(async (element) => {
      const value = element.getAttribute("srcset") ?? "";
      const resolved = await resolver.inlineSrcset(value);
      if (resolved === "") {
        element.removeAttribute("srcset");
        return;
      }
      element.setAttribute("srcset", resolved);
    }),
    ...Array.from(doc.querySelectorAll("[style]")).map(async (element) => {
      const value = element.getAttribute("style") ?? "";
      if (!value.includes("url(")) return;
      element.setAttribute("style", await resolver.inlineCss(value, baseDir));
    }),
  ]);

  return {
    document: `<!DOCTYPE html>${doc.documentElement.outerHTML}`,
    missing: resolver.missing,
  };
}
