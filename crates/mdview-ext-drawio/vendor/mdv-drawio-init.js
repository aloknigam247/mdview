(function () {
  "use strict";
  function mount() {
    if (window.GraphViewer && typeof window.GraphViewer.processElements === "function") {
      window.GraphViewer.processElements(".drawio-viewer");
    }
  }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", mount);
  } else {
    mount();
  }
})();
