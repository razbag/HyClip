const { invoke } = window.__TAURI__.core;

const numberInput = document.getElementById("max-history");
const rangeInput = document.getElementById("max-history-slider");

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

document.addEventListener("DOMContentLoaded", loadPreferences);
