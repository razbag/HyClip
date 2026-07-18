const { invoke } = window.__TAURI__.core;

async function loadAboutInfo() {
  const info = await invoke("get_about_info");
  document.getElementById("about-name").textContent = info.name;
  document.getElementById("about-version").textContent = `Version ${info.version}`;
}

document.addEventListener("DOMContentLoaded", loadAboutInfo);
