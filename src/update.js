const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const states = {
  checking: document.getElementById("state-checking"),
  uptodate: document.getElementById("state-uptodate"),
  available: document.getElementById("state-available"),
  installing: document.getElementById("state-installing"),
  error: document.getElementById("state-error"),
};

let currentUpdate = null;

function showState(name) {
  for (const [key, el] of Object.entries(states)) {
    el.classList.toggle("hidden", key !== name);
  }
}

function showAvailable(info) {
  currentUpdate = info;
  document.getElementById("version-label").textContent =
    `${info.current_version} → ${info.version}`;
  document.getElementById("update-notes").textContent = info.body || "";
  showState("available");
}

async function runCheck() {
  showState("checking");
  try {
    const info = await invoke("check_for_update");
    if (info) {
      showAvailable(info);
    } else {
      document.getElementById("uptodate-version").textContent =
        `You're on the latest version.`;
      showState("uptodate");
    }
  } catch (err) {
    document.getElementById("error-message").textContent = String(err);
    showState("error");
  }
}

document.getElementById("later-btn").addEventListener("click", () => {
  invoke("hide_update_window_cmd");
});

document.getElementById("ok-btn").addEventListener("click", () => {
  invoke("hide_update_window_cmd");
});

document.getElementById("error-ok-btn").addEventListener("click", () => {
  invoke("hide_update_window_cmd");
});

document.getElementById("install-btn").addEventListener("click", async () => {
  if (!currentUpdate) return;
  showState("installing");
  try {
    await invoke("install_update", { resourceId: currentUpdate.resource_id });
  } catch (err) {
    document.getElementById("error-message").textContent = String(err);
    showState("error");
  }
});

listen("check-updates", runCheck);
listen("update-available", (event) => showAvailable(event.payload));
