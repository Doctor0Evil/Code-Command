// FILE: ./js/app/vfs-tree-model.js
export class VfsTreeModel {
  constructor() {
    this.nodesByPath = Object.create(null); // path -> { path, isDir, children: [] }
  }

  seedFromSnapshot(snapshotArray) {
    // snapshotArray: [{ path, isdir, sha, content }, ...]
    this.nodesByPath = Object.create(null);
    for (const entry of snapshotArray) {
      const path = entry.path.trim();
      if (!path) continue;
      const isDir = !!entry.isdir;
      const node = this._ensureNode(path, isDir);
      node.sha = entry.sha || "";
    }
    this._linkParents();
  }

  applyWrite(path, isDir, sha) {
    const node = this._ensureNode(path, isDir);
    node.sha = sha || "";
    this._linkParents();
  }

  applyDelete(path) {
    delete this.nodesByPath[path];
    this._linkParents();
  }

  _ensureNode(path, isDir) {
    if (!this.nodesByPath[path]) {
      this.nodesByPath[path] = { path, name: this._basename(path), isDir, children: [] };
    } else if (isDir != null) {
      this.nodesByPath[path].isDir = isDir;
    }
    return this.nodesByPath[path];
  }

  _basename(path) {
    const parts = path.split("/").filter(p => p.length > 0);
    return parts.length ? parts[parts.length - 1] : "";
  }

  _linkParents() {
    for (const n of Object.values(this.nodesByPath)) n.children.length = 0;
    for (const node of Object.values(this.nodesByPath)) {
      const parts = node.path.split("/").filter(p => p.length > 0);
      if (parts.length === 0) continue;
      const parentPath = parts.slice(0, parts.length - 1).join("/");
      const parent = parentPath ? this._ensureNode(parentPath, true) : null;
      if (parent) parent.children.push(node);
    }
  }

  // Root children (depth 1) for rendering:
  rootChildren() {
    const roots = [];
    for (const node of Object.values(this.nodesByPath)) {
      const depth = node.path.split("/").filter(p => p.length > 0).length;
      if (depth === 1) roots.push(node);
    }
    roots.sort((a, b) => a.path.localeCompare(b.path));
    return roots;
  }
}
