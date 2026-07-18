// A tiny but genuine React 19 CSR todo app. Mounted with createRoot (the
// concurrent-mode paradigm React 18/19 use, NOT React 17's ReactDOM.render)
// into an initially-empty <div id="root">. It exercises the SPA-critical env
// surface that the field trial's React 19 todomvc crashed on: history.state
// (SPA routers read it on first render) and new URL(location.href).searchParams
// (routers parse the current URL to pick the initial route/filter). Both were
// absent then and are shimmed now — this app renders only if they work.
import React, { useState } from "react";
import { createRoot } from "react-dom/client";

// Router-style init: read the initial route from history + the parsed URL.
// If either shim were missing this throws before the first paint (exactly the
// field-trial crash), so a clean render is the regression guard.
function initialFilter() {
  var fromHistory = history.state && history.state.filter;
  var url = new URL(window.location.href);
  return fromHistory || url.searchParams.get("filter") || "all";
}

function App() {
  var seed = ["Buy milk", "Write tests", "Ship frot"];
  var pair = useState(seed);
  var items = pair[0];
  var textPair = useState("");
  var text = textPair[0];
  var filter = initialFilter();
  return React.createElement(
    "main",
    null,
    React.createElement("h1", null, "Todos"),
    React.createElement("p", null, "Filter: " + filter),
    React.createElement("input", {
      type: "text",
      "aria-label": "New todo",
      placeholder: "What needs doing?",
      value: text,
    }),
    React.createElement(
      "ul",
      null,
      items.map(function (label, i) {
        return React.createElement("li", { key: i }, label);
      })
    )
  );
}

createRoot(document.getElementById("root")).render(React.createElement(App));
