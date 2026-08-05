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

// Default job request
const defaultJob = {
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
  bindings: [
    {
      node: "principal",
      distribution: { kind: "uniform", low: 90, high: 110 },
    },
  ],
  target: "fv",
  universe_count: 1000,
  seed: 42,
};
$("jobJson").value = JSON.stringify(defaultJob, null, 2);

$("createJobBtn").addEventListener("click", async () => {
  try {
    const body = JSON.parse($("jobJson").value);
    const data = await api("/simulate/jobs", {
      method: "POST",
      body: JSON.stringify(body),
    });
    show("jobOut", data);
  } catch (err) {
    show("jobOut", err.message, true);
  }
});

$("getJobBtn").addEventListener("click", async () => {
  try {
    const id = $("jobId").value.trim();
    if (!id) throw new Error("job id is required");
    const data = await api(`/simulate/jobs/${encodeURIComponent(id)}`);
    show("jobOut", data);
  } catch (err) {
    show("jobOut", err.message, true);
  }
});

$("cancelJobBtn").addEventListener("click", async () => {
  try {
    const id = $("jobId").value.trim();
    if (!id) throw new Error("job id is required");
    const data = await api(`/simulate/jobs/${encodeURIComponent(id)}/cancel`, {
      method: "POST",
    });
    show("jobOut", data);
  } catch (err) {
    show("jobOut", err.message, true);
  }
});

$("auditBtn").addEventListener("click", async () => {
  try {
    const limit = $("auditLimit").value || 10;
    const offset = $("auditOffset").value || 0;
    const data = await api(`/audit?limit=${limit}&offset=${offset}`);
    show("auditOut", data);
  } catch (err) {
    show("auditOut", err.message, true);
  }
});

function adminHeaders() {
  const key = $("adminKey").value.trim();
  if (!key) throw new Error("admin key is required");
  return { "Content-Type": "application/json", "X-Admin-Key": key };
}

async function adminApi(path, options = {}) {
  const res = await fetch(`${baseUrl()}${path}`, {
    ...options,
    headers: { ...adminHeaders(), ...options.headers },
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

$("adminListTenantsBtn").addEventListener("click", async () => {
  try {
    const data = await adminApi("/admin/tenants");
    show("adminOut", data);
  } catch (err) {
    show("adminOut", err.message, true);
  }
});

$("adminProvisionTenantBtn").addEventListener("click", async () => {
  try {
    const id = prompt("Tenant ID:");
    if (!id) return;
    const data = await adminApi("/admin/tenants", {
      method: "POST",
      body: JSON.stringify({ id }),
    });
    show("adminOut", data);
  } catch (err) {
    show("adminOut", err.message, true);
  }
});

$("adminTenantStatsBtn").addEventListener("click", async () => {
  try {
    const id = $("adminTenantId").value.trim();
    if (!id) throw new Error("tenant id is required");
    const data = await adminApi(`/admin/tenants/${encodeURIComponent(id)}/stats`);
    show("adminOut", data);
  } catch (err) {
    show("adminOut", err.message, true);
  }
});

$("adminCreateKeyBtn").addEventListener("click", async () => {
  try {
    const tenantId = $("adminKeyTenantId").value.trim();
    const workspaceId = $("adminKeyWorkspaceId").value.trim();
    if (!tenantId || !workspaceId) throw new Error("tenant and workspace ids are required");
    const data = await adminApi("/admin/keys", {
      method: "POST",
      body: JSON.stringify({ tenant_id: tenantId, workspace_id: workspaceId }),
    });
    show("adminOut", data);
  } catch (err) {
    show("adminOut", err.message, true);
  }
});

$("adminRevokeKeyBtn").addEventListener("click", async () => {
  try {
    const id = $("adminRevokeKeyId").value.trim();
    if (!id) throw new Error("key id is required");
    const data = await adminApi(`/admin/keys/${encodeURIComponent(id)}/revoke`, {
      method: "POST",
    });
    show("adminOut", data);
  } catch (err) {
    show("adminOut", err.message, true);
  }
});
