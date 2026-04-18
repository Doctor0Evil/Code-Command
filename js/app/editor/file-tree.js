// FILE: ./js/app/editor/file-tree.js

// Virtual file tree component for Code-Command. Renders a nested <ul>/<li>
// structure from a GitHub API tree object and emits file selection events. [file:2]

import * as GithubAPI from "../github/api.js";

export class FileTree {
  /**
   * @param {HTMLElement} container
   * @param {{ onFileSelect: (path: string) => void }} callbacks
   */
  constructor(container, callbacks = {}) {
    this.container = container;
    this.onFileSelect = callbacks.onFileSelect || (() => {});
    this.tree = null;
    this.activePath = null;
  }

  /**
   * Sets the internal tree model and re-renders the UI. [file:2]
   * `tree` is expected to be an array of entries like:
   *   { path: "src/main.rs", type: "blob" | "tree" }
   */
  setTree(tree) {
    this.tree = buildNestedTree(tree || []);
    this.render();
  }

  /**
   * Returns the currently active file path, if any. [file:2]
   */
  getActivePath() {
    return this.activePath;
  }

  render() {
    if (!this.container) return;
    this.container.innerHTML = "";

    if (!this.tree || this.tree.children.length === 0) {
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

    this.tree.children.forEach((node) => {
      const li = this.renderNode(node, 0);
      rootList.appendChild(li);
    });

    this.container.appendChild(rootList);
  }

  renderNode(node, depth) {
    const li = document.createElement("li");
    li.style.margin = "0";
    li.style.padding = "2px 0 2px 4px";

    const row = document.createElement("div");
    row.style.display = "flex";
    row.style.alignItems = "center";
    row.style.cursor = node.type === "blob" ? "pointer" : "default";

    const indent = document.createElement("span");
    indent.style.display = "inline-block";
    indent.style.width = depth * 10 + "px";
    row.appendChild(indent);

    const icon = document.createElement("span");
    icon.style.display = "inline-block";
    icon.style.width = "12px";
    icon.style.marginRight = "4px";
    icon.textContent = node.type === "tree" ? "" : "";
    icon.style.fontFamily = "monospace";
    icon.style.fontSize = "11px";
    row.appendChild(icon);

    const label = document.createElement("span");
    label.textContent = node.name;
    label.style.fontSize = "12px";
    label.style.color = "#e5e7eb";

    // CC-DEEP: visually highlight nodes at depth >= 3. [file:2]
    if (depth >= 2) {
      label.style.color = "#60a5fa";
    }

    row.appendChild(label);

    if (node.type === "blob") {
      row.addEventListener("click", () => {
        this.activePath = node.path;
        this.highlightActive(node.path);
        this.onFileSelect(node.path);
      });
    }

    li.appendChild(row);

    if (node.type === "tree" && node.children && node.children.length > 0) {
      const ul = document.createElement("ul");
      ul.style.listStyle = "none";
      ul.style.margin = "0";
      ul.style.padding = "0";
      node.children.forEach((child) => {
        const childLi = this.renderNode(child, depth + 1);
        ul.appendChild(childLi);
      });
      li.appendChild(ul);
    }

    return li;
  }

  /**
   * Highlights the currently active file path in the tree. [file:2]
   */
  highlightActive(path) {
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

/* ---------- Tree Construction Helpers ---------- */ [file:2]

function buildNestedTree(entries) {
  const root = {
    name: "",
    path: "",
    type: "tree",
    children: [],
  };

  for (const entry of entries) {
    const path = (entry.path || "").trim();
    if (!path) continue;

    const parts = path.split("/").filter((p) => p.length > 0);
    let cursor = root;
    let currentPath = "";

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      currentPath = currentPath ? currentPath + "/" + part : part;

      const isLeaf = i === parts.length - 1;
      if (isLeaf) {
        cursor.children.push({
          name: part,
          path: currentPath,
          type: entry.type === "tree" ? "tree" : "blob",
          children: [],
        });
      } else {
        let child = cursor.children.find(
          (c) => c.name === part && c.type === "tree"
        );
        if (!child) {
          child = {
            name: part,
            path: currentPath,
            type: "tree",
            children: [],
          };
          cursor.children.push(child);
        }
        cursor = child;
      }
    }
  }

  // Attach data-cc-path attributes after building (for highlightActive). [file:2]
  decoratePaths(root);
  return root;
}

function decoratePaths(node) {
  if (!node) return;
  if (node.type === "blob" || node.type === "tree") {
    node.dataPath = node.path;
  }
  if (node.children) {
    node.children.forEach(decoratePaths);
  }
}
