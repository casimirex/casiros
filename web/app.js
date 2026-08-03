const $ = (id) => document.getElementById(id);

const defaultEvaluate = {
  nodes: [
    { input: { name: "principal" } },
    {
      formula: {
        name: "fv",
        kind: {
          formula: "future_value",
          present_value: { node: "principal" },
          rate: 0.05,
          periods: 10,
        },
      },
    },
  ],
  edges: [{ dependency: "principal", dependent: "fv" }],
  inputs: { principal: "100.0" },
};

const defaultSimulate = {
  nodes: [
    { input: { name: "x" } },
    {
      formula: {
        name: "doubled",
        kind: {
          formula: "future_value",
          present_value: { node: "x" },
          rate: 0,
          periods: 1,
        },
      },
    },
  ],
  edges: [{ dependency: "x", dependent: "doubled" }],
  bindings: [
    {
      node: "x",
      distribution: { kind: "uniform", low: 0, high: 100 },
    },
  ],
  target: "doubled",
  universe_count: 1000,
  seed: 42,
};

$("evaluateJson").value = JSON.stringify(defaultEvaluate, null, 2);
$("simulateJson").value = JSON.stringify(defaultSimulate, null, 2);

function headers() {
  const h = { "Content-Type": "application/json" };
  const key = $("apiKey").value.trim();
  if (key) {
    h["Authorization"] = `Bearer ${key}`;
  }
  return h;
}

function baseUrl() {
  return $("baseUrl").value.replace(/\/$/, "");
}

async function api(path, options = {}) {
  const res = await fetch(`${baseUrl()}${path}`, {
    ...options,
    headers: { ...headers(), ...options.headers },
  });
  const text = await res.text();
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    data = text;
  }
  if (!res.ok) {
    throw new Error(data.error || text || `HTTP ${res.status}`);
  }
  return data;
}

function show(id, data, isError = false) {
  const el = $(id);
  el.textContent =
    typeof data === "string" ? data : JSON.stringify(data, null, 2);
  el.className = "output " + (isError ? "error" : "success");
}

let evaluateChart = null;
let simulateChart = null;

function renderBarChart(canvasId, labels, values, title) {
  const ctx = $(canvasId).getContext("2d");
  if (window[canvasId]) {
    window[canvasId].destroy();
  }
  window[canvasId] = new Chart(ctx, {
    type: "bar",
    data: {
      labels,
      datasets: [
        {
          label: title,
          data: values,
          backgroundColor: "rgba(56, 189, 248, 0.7)",
          borderColor: "rgba(56, 189, 248, 1)",
          borderWidth: 1,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: { title: { display: true, text: title } },
      scales: {
        y: { beginAtZero: true, grid: { color: "rgba(255,255,255,0.1)" } },
        x: { grid: { color: "rgba(255,255,255,0.1)" } },
      },
    },
  });
}

$("healthBtn").addEventListener("click", async () => {
  try {
    const data = await api("/healthz");
    show("healthOut", data);
  } catch (err) {
    show("healthOut", err.message, true);
  }
});

$("evaluateBtn").addEventListener("click", async () => {
  try {
    const body = JSON.parse($("evaluateJson").value);
    const data = await api("/evaluate", {
      method: "POST",
      body: JSON.stringify(body),
    });
    show("evaluateOut", data);
    const outputs = data.outputs || {};
    renderBarChart(
      "evaluateChart",
      Object.keys(outputs),
      Object.values(outputs).map(parseFloat),
      "Evaluate outputs"
    );
  } catch (err) {
    show("evaluateOut", err.message, true);
  }
});

$("simulateBtn").addEventListener("click", async () => {
  try {
    const body = JSON.parse($("simulateJson").value);
    const data = await api("/simulate", {
      method: "POST",
      body: JSON.stringify(body),
    });
    show("simulateOut", data);
    renderBarChart(
      "simulateChart",
      ["mean", "median", "min", "max"],
      [data.mean, data.median, data.min, data.max].map(parseFloat),
      `Simulation (n=${data.count})`
    );
  } catch (err) {
    show("simulateOut", err.message, true);
  }
});

$("saveSnapshotBtn").addEventListener("click", async () => {
  try {
    const id = $("snapshotId").value.trim();
    if (!id) throw new Error("snapshot id is required");
    const body = JSON.parse($("evaluateJson").value);
    const data = await api("/snapshots", {
      method: "POST",
      body: JSON.stringify({ id, ...body }),
    });
    show("snapshotOut", data);
  } catch (err) {
    show("snapshotOut", err.message, true);
  }
});

$("loadSnapshotBtn").addEventListener("click", async () => {
  try {
    const id = $("snapshotId").value.trim();
    if (!id) throw new Error("snapshot id is required");
    const data = await api(`/snapshots/${encodeURIComponent(id)}`);
    show("snapshotOut", data);
  } catch (err) {
    show("snapshotOut", err.message, true);
  }
});

$("deleteSnapshotBtn").addEventListener("click", async () => {
  try {
    const id = $("snapshotId").value.trim();
    if (!id) throw new Error("snapshot id is required");
    await api(`/snapshots/${encodeURIComponent(id)}`, { method: "DELETE" });
    show("snapshotOut", `deleted ${id}`);
  } catch (err) {
    show("snapshotOut", err.message, true);
  }
});

$("listSnapshotsBtn").addEventListener("click", async () => {
  try {
    const data = await api("/snapshots");
    show("snapshotOut", data);
  } catch (err) {
    show("snapshotOut", err.message, true);
  }
});
