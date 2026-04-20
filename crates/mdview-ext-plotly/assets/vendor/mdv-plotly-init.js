(function () {
  "use strict";

  function themedLayout(userLayout) {
    var root = document.documentElement;
    var style = getComputedStyle(root);
    var bg =
      style.getPropertyValue("--mdv-bg").trim() ||
      style.getPropertyValue("background-color").trim() ||
      "#ffffff";
    var base = { paper_bgcolor: bg, plot_bgcolor: bg };
    if (!userLayout) return base;
    for (var k in userLayout) {
      if (Object.prototype.hasOwnProperty.call(userLayout, k)) {
        base[k] = userLayout[k];
      }
    }
    return base;
  }

  function mount(div) {
    if (div.dataset.mdvPlotlyMounted === "1") return;
    if (!window.Plotly) return;
    var raw = div.getAttribute("data-spec");
    if (!raw) return;
    var spec;
    try {
      spec = JSON.parse(raw);
    } catch (e) {
      div.textContent = "[plotly: invalid JSON]";
      div.dataset.mdvPlotlyMounted = "1";
      return;
    }
    if (!spec || !Array.isArray(spec.data)) {
      div.textContent = "[plotly: missing data array]";
      div.dataset.mdvPlotlyMounted = "1";
      return;
    }
    var layout = themedLayout(spec.layout);
    window.Plotly.newPlot(div, spec.data, layout, {
      responsive: true,
      displaylogo: false,
    });
    div.dataset.mdvPlotlyMounted = "1";
  }

  function mountAll() {
    var nodes = document.querySelectorAll(".plotly-chart");
    for (var i = 0; i < nodes.length; i++) mount(nodes[i]);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", mountAll);
  } else {
    mountAll();
  }

  window.__mdvPlotlyRefresh = mountAll;
})();
