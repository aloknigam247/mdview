(function () {
  function run() {
    if (typeof window.mermaid === "undefined") {
      return;
    }
    try {
      window.mermaid.initialize({ startOnLoad: false, securityLevel: "loose" });
      window.mermaid.run({ querySelector: ".mermaid" });
    } catch (err) {
      console.error("mdv-mermaid-init:", err);
    }
  }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", run);
  } else {
    run();
  }
})();
