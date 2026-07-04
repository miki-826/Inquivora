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
  const index = relativePath.lastIndexOf("/");
  return index === -1 ? "" : relativePath.slice(0, index);
}

export function joinPath(parent: string, name: string): string {
  return parent === "" ? name : `${parent}/${name}`;
}

export function isSameOrDescendant(ancestor: string, path: string): boolean {
  if (ancestor === "") return true;
  return path === ancestor || path.startsWith(`${ancestor}/`);
}

export function flattenTree(
  children: Record<string, TreeEntry[] | undefined>,
  expanded: ReadonlySet<string>,
): TreeRow[] {
  const rows: TreeRow[] = [];
  const walk = (path: string, depth: number) => {
    for (const entry of children[path] ?? []) {
      rows.push({ entry, depth });
      if (entry.isFolder && expanded.has(entry.relativePath)) {
        walk(entry.relativePath, depth + 1);
      }
    }
  };
  walk("", 0);
  return rows;
}
