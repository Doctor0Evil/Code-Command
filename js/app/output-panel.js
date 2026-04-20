// FILE: ./js/app/output-panel.js

renderContamination(c) {
  const block = document.createElement("div");
  block.className = "cc-contamination-report";

  const icon = document.createElement("span");
  icon.className = "cc-contamination-icon";
  icon.textContent = "!";
  block.appendChild(icon);

  const title = document.createElement("div");
  title.className = "cc-contamination-title";
  title.textContent = "Blacklist violation detected";
  block.appendChild(title);

  const body = document.createElement("div");
  body.className = "cc-contamination-body";
  body.textContent =
    c.pattern +
    " (" + c.severity + ") at " +
    (c.path || "<buffer>") +
    ":" + c.line + ":" + c.column;
  block.appendChild(body);

  const context = document.createElement("pre");
  context.className = "cc-contamination-context";
  context.textContent = c.surrounding_context;
  block.appendChild(context);

  this.container.appendChild(block);
}
