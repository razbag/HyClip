const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const numberInput = document.getElementById("max-history");
const rangeInput = document.getElementById("max-history-slider");
const launchAtLoginInput = document.getElementById("launch-at-login");
const retentionSelect = document.getElementById("retention-select");
const deletePrompt = document.getElementById("delete-prompt");
const deleteConfirm = document.getElementById("delete-confirm");
const deleteHistoryBtn = document.getElementById("delete-history-btn");
const cancelDeleteBtn = document.getElementById("cancel-delete");
const confirmDeleteBtn = document.getElementById("confirm-delete");

function setBoth(value) {
  numberInput.value = value;
  rangeInput.value = value;
}

async function loadPreferences() {
  const prefs = await invoke("get_preferences");
  numberInput.min = prefs.min;
  numberInput.max = prefs.max;
  rangeInput.min = prefs.min;
  rangeInput.max = prefs.max;
  setBoth(prefs.max_history);
  launchAtLoginInput.checked = prefs.launch_at_login;
  retentionSelect.value = prefs.retention;
  deletePrompt.classList.remove("hidden");
  deleteConfirm.classList.add("hidden");
}

async function applyValue(rawValue) {
  const parsed = parseInt(rawValue, 10);
  if (Number.isNaN(parsed)) {
    loadPreferences();
    return;
  }
  const clamped = await invoke("set_max_history", { value: parsed });
  setBoth(clamped);
}

rangeInput.addEventListener("input", () => {
  numberInput.value = rangeInput.value;
});

rangeInput.addEventListener("change", () => {
  applyValue(rangeInput.value);
});

numberInput.addEventListener("input", () => {
  rangeInput.value = numberInput.value || rangeInput.value;
});

numberInput.addEventListener("change", () => {
  applyValue(numberInput.value);
});

launchAtLoginInput.addEventListener("change", async () => {
  const result = await invoke("set_launch_at_login", {
    enabled: launchAtLoginInput.checked,
  });
  launchAtLoginInput.checked = result;
});

retentionSelect.addEventListener("change", () => {
  invoke("set_retention", { value: retentionSelect.value });
});

deleteHistoryBtn.addEventListener("click", () => {
  deletePrompt.classList.add("hidden");
  deleteConfirm.classList.remove("hidden");
});

cancelDeleteBtn.addEventListener("click", () => {
  deleteConfirm.classList.add("hidden");
  deletePrompt.classList.remove("hidden");
});

confirmDeleteBtn.addEventListener("click", async () => {
  await invoke("clear_history");
  deleteConfirm.classList.add("hidden");
  deletePrompt.classList.remove("hidden");
});

listen("preferences-shown", loadPreferences);
