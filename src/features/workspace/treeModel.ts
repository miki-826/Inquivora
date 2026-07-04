export type FileCategory = "edit" | "preview" | "external" | "unknown";

export type TreeEntry = {
  name: string;
  relativePath: string;
  isFolder: boolean;
  hasChildren: boolean;
  sizeBytes: number;
  extension: string | null;
  category: FileCategory;
};

export type TreeRow = {
  entry: TreeEntry;
  depth: number;
};

export function parentPath(relativePath: string): string {
  throw new Error("未実装");
}

export function joinPath(parent: string, name: string): string {
  throw new Error("未実装");
}

export function isSameOrDescendant(ancestor: string, path: string): boolean {
  throw new Error("未実装");
}

export function flattenTree(
  children: Record<string, TreeEntry[] | undefined>,
  expanded: ReadonlySet<string>,
): TreeRow[] {
  throw new Error("未実装");
}
