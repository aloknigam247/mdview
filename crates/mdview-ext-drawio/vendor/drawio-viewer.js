// Minimal offline drawio viewer shim.
// Exposes window.GraphViewer with a single `processElements` entry point that
// mounts an <iframe> pointing at a local data: URL containing the raw XML.
// The real viewer ships with draw.io but cannot be embedded here for licensing
// reasons; this shim renders the diagram as an inline SVG stub so the viewer
// div has visible content for tests and offline sessions.
(function () {
  "use strict";

  function decodeB64(str) {
    try {
      return decodeURIComponent(
        atob(str)
          .split("")
          .map(function (c) {
            return "%" + ("00" + c.charCodeAt(0).toString(16)).slice(-2);
          })
          .join(""),
      );
    } catch (e) {
      return "";
    }
  }

  function extractShapes(xml) {
    var shapes = [];
    var re = /<mxCell\b[^>]*\bvalue="([^"]*)"[^>]*>[\s\S]*?<mxGeometry\b([^/]*)\/>/g;
    var m;
    while ((m = re.exec(xml)) !== null) {
      var value = m[1];
      var attrs = m[2];
      var get = function (name) {
        var r = new RegExp(name + '="([^"]*)"');
        var mm = r.exec(attrs);
        return mm ? parseFloat(mm[1]) : 0;
      };
      shapes.push({
        value: value,
        x: get("x"),
        y: get("y"),
        w: get("width") || 120,
        h: get("height") || 40,
      });
    }
    return shapes;
  }

  function renderSvg(xml, scale) {
    var shapes = extractShapes(xml);
    if (shapes.length === 0) {
      shapes = [{ value: "drawio", x: 20, y: 20, w: 160, h: 60 }];
    }
    var bounds = shapes.reduce(
      function (acc, s) {
        return {
          minX: Math.min(acc.minX, s.x),
          minY: Math.min(acc.minY, s.y),
          maxX: Math.max(acc.maxX, s.x + s.w),
          maxY: Math.max(acc.maxY, s.y + s.h),
        };
      },
      { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity },
    );
    var pad = 20;
    var vbW = bounds.maxX - bounds.minX + pad * 2;
    var vbH = bounds.maxY - bounds.minY + pad * 2;
    var width = Math.max(200, vbW * (scale || 1));
    var height = Math.max(120, vbH * (scale || 1));
    var parts = [];
    parts.push(
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="' +
        (bounds.minX - pad) +
        " " +
        (bounds.minY - pad) +
        " " +
        vbW +
        " " +
        vbH +
        '" width="' +
        width +
        '" height="' +
        height +
        '">',
    );
    parts.push(
      '<rect x="' +
        (bounds.minX - pad) +
        '" y="' +
        (bounds.minY - pad) +
        '" width="' +
        vbW +
        '" height="' +
        vbH +
        '" fill="#ffffff" stroke="#e2e8f0" rx="12" />',
    );
    for (var i = 0; i < shapes.length; i++) {
      var s = shapes[i];
      parts.push(
        '<rect x="' +
          s.x +
          '" y="' +
          s.y +
          '" width="' +
          s.w +
          '" height="' +
          s.h +
          '" rx="10" ry="10" fill="#dae8fc" stroke="#6c8ebf" />',
      );
      parts.push(
        '<text x="' +
          (s.x + s.w / 2) +
          '" y="' +
          (s.y + s.h / 2 + 5) +
          '" text-anchor="middle" font-family="ui-sans-serif,system-ui" font-size="14" fill="#1f2937">' +
          (s.value || "") +
          "</text>",
      );
    }
    parts.push("</svg>");
    return parts.join("");
  }

  var GraphViewer = {
    processElements: function (selector) {
      var nodes = document.querySelectorAll(selector || ".drawio-viewer");
      for (var i = 0; i < nodes.length; i++) {
        var el = nodes[i];
        if (el.dataset.mounted === "1") continue;
        var xml = decodeB64(el.dataset.xmlB64 || "");
        var scale = parseFloat(el.dataset.scale || "1") || 1;
        el.innerHTML = renderSvg(xml, scale);
        el.dataset.mounted = "1";
      }
    },
  };

  window.GraphViewer = GraphViewer;
})();
