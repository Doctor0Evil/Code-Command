// FILE: ./js/app/editor/file-tree.js

// Virtual file tree component for Code-Command.
// Renders a nested <ul>/<li> structure from a VFS-backed model and emits
// file selection events when a file node is clicked.

import { eventBus } from "../event-bus.js";
import { VfsTreeModel } from "../vfs-tree-model.js";

export class FileTree {
  /**
   * @param {HTMLElement} container
   * @param {{ onFileSelect?: (path: string) => void }} callbacks
   */
  constructor(container, callbacks = {}) {
    this.container = container;
    this.onFileSelect = callbacks.onFileSelect || (() => {});
    this.model = new VfsTreeModel();
    this.activePath = null;

    // Subscribe to VFS changes so the tree stays in sync with the engine.
    eventBus.on("vfs:updated", (evt) => this.onVfsUpdated(evt));
  }

  /**
   * Manually seed the tree model from a VFS snapshot-style array.
   * Each entry should look like:
   *   { path: "src/main.rs", isDir: false, sha: "..." }
   */
  setFromSnapshot(entries) {
    this.model.seedFromSnapshot(entries || []);
    this.render();
  }

  /**
   * Handle vfs:updated events from the event bus.
   * evt.changes is expected to be a list of:
   *   { kind: "write" | "delete", path: string, sha?: string }
   */
  onVfsUpdated(evt) {
    if (!evt || !Array.isArray(evt.changes)) {
      return;
    }

    for (const change of evt.changes) {
      if (!change || typeof change.path !== "string") continue;

      if (change.kind === "write") {
        this.model.applyWrite(change.path, /* isDir */ false, change.sha || "");
      } else if (change.kind === "delete") {
        this.model.applyDelete(change.path);
      }
    }

    this.render();
  }

  /**
   * Returns the currently active file path, if any.
   */
  getActivePath() {
    return this.activePath;
  }

  /**
   * Re-render the entire tree from the current model.
   */
  render() {
    if (!this.container) return;
    this.container.innerHTML = "";

    const roots = this.model.rootChildren || [];
    if (!roots.length) {
      const div = document.createElement("div");
      div.textContent = "No repository loaded.";
      div.style.fontSize = "12px";
      div.style.color = "#6b7280";
      this.container.appendChild(div);
      return;
    }

    const rootList = document.createElement("ul");
    rootList.style.listStyle = "none";
    rootList.style.margin = "0";
    rootList.style.padding = "0";

    roots.forEach((node) => {
      const li = this.renderNode(node, 0);
      rootList.appendChild(li);
    });

    this.container.appendChild(rootList);

    // Re-apply active path highlight after re-render.
    if (this.activePath) {
      this.highlightActive(this.activePath);
    }
  }

  /**
   * Render a single node (file or directory) and its children.
   *
   * Node shape (from VfsTreeModel):
   *   { path: string, name: string, isDir: boolean, children?: Node[] }
   */
  renderNode(node, depth) {
    const li = document.createElement("li");
    li.style.margin = "0";
    li.style.padding = "2px 0 2px 4px";

    const row = document.createElement("div");
    row.style.display = "flex";
    row.style.alignItems = "center";
    row.style.cursor = node.isDir ? "default" : "pointer";

    const indent = document.createElement("span");
    indent.style.display = "inline-block";
    indent.style.width = depth * 10 + "px";
    row.appendChild(indent);

    const icon = document.createElement("span");
    icon.style.display = "inline-block";
    icon.style.width = "12px";
    icon.style.marginRight = "4px";
    icon.textContent = node.isDir ? "" : "";
    icon.style.fontFamily = "monospace";
    icon.style.fontSize = "11px";
    row.appendChild(icon);

    const label = document.createElement("span");
    label.textContent = node.name;
    label.style.fontSize = "12px";
    label.style.color = "#e5e7eb";
    label.dataset.ccPath = node.path;

    // CC-DEEP: visually highlight nodes at depth >= 3.
    if (depth >= 2) {
      label.style.color = "#60a5fa";
    }

    if (!node.isDir) {
      row.addEventListener("click", () => {
        this.activePath = node.path;
        this.highlightActive(node.path);
        this.onFileSelect(node.path);
      });
    }

    row.appendChild(label);
    li.appendChild(row);

    if (node.isDir && node.children && node.children.length > 0) {
      const ul = document.createElement("ul");
      ul.style.listStyle = "none";
      ul.style.margin = "0";
      ul.style.padding = "0";

      node.children
        .slice()
        .sort((a, b) => a.path.localeCompare(b.path))
        .forEach((child) => {
          const childLi = this.renderNode(child, depth + 1);
          ul.appendChild(childLi);
        });

      li.appendChild(ul);
    }

    return li;
  }

  /**
   * Highlights the currently active file path in the tree.
   */
  highlightActive(path) {
    if (!this.container) return;
    const labels = this.container.querySelectorAll("[data-cc-path]");
    labels.forEach((el) => {
      if (el.getAttribute("data-cc-path") === path) {
        el.style.backgroundColor = "#111827";
      } else {
        el.style.backgroundColor = "transparent";
      }
    });
  }
}
