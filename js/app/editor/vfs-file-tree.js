// FILE: ./js/app/editor/vfs-file-tree.js
import { VfsTreeModel } from "../vfs-tree-model.js";

export class VfsFileTree {
  constructor(container, callbacks) {
    this.container = container;
    this.onFileSelect = callbacks.onFileSelect;
    this.model = new VfsTreeModel();
    this.activePath = null;
  }

  setFromSnapshot(snapshotArray) {
    this.model.seedFromSnapshot(snapshotArray);
    this.render();
  }

  applyWrite(path, isDir, sha) {
    this.model.applyWrite(path, isDir, sha);
    this.render();
  }

  applyDelete(path) {
    this.model.applyDelete(path);
    this.render();
  }

  render() {
    if (!this.container) return;
    this.container.innerHTML = "";
    const roots = this.model.rootChildren();
    if (!roots.length) {
      const div = document.createElement("div");
      div.textContent = "VFS is empty.";
      div.style.fontSize = "12px";
      div.style.color = "#6b7280";
      this.container.appendChild(div);
      return;
    }
    const ul = document.createElement("ul");
    ul.style.listStyle = "none";
    ul.style.margin = "0";
    ul.style.padding = "0";
    roots.forEach(node => ul.appendChild(this._renderNode(node, 0)));
    this.container.appendChild(ul);
  }

  _renderNode(node, depth) {
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
    icon.style.fontFamily = "monospace";
    icon.style.fontSize = "11px";
    icon.textContent = node.isDir ? "▸" : "•";
    row.appendChild(icon);

    const label = document.createElement("span");
    label.textContent = node.name;
    label.style.fontSize = "12px";
    label.style.color = depth === 2 ? "#60a5fa" : "#e5e7eb"; // CC‑DEEP visual hint.[file:2]
    label.dataset.ccPath = node.path;
    row.appendChild(label);

    if (!node.isDir) {
      row.addEventListener("click", () => {
        this.activePath = node.path;
        this._highlightActive(node.path);
        this.onFileSelect(node.path);
      });
    }

    li.appendChild(row);

    if (node.children && node.children.length) {
      const ul = document.createElement("ul");
      ul.style.listStyle = "none";
      ul.style.margin = "0";
      ul.style.padding = "0";
      node.children
        .slice()
        .sort((a, b) => a.path.localeCompare(b.path))
        .forEach(child => ul.appendChild(this._renderNode(child, depth + 1)));
      li.appendChild(ul);
    }
    return li;
  }

  _highlightActive(path) {
    if (!this.container) return;
    const labels = this.container.querySelectorAll("[data-cc-path]");
    labels.forEach(el => {
      if (el.getAttribute("data-cc-path") === path) {
        el.style.backgroundColor = "#111827";
      } else {
        el.style.backgroundColor = "transparent";
      }
    });
  }
}
