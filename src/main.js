const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const searchInput = document.getElementById("search");
const listEl = document.getElementById("list");
const emptyEl = document.getElementById("empty");

let allItems = [];
let filtered = [];
let selectedIndex = 0;

function render() {
  const query = searchInput.value.trim().toLowerCase();
  filtered = query
    ? allItems.filter((item) => item.toLowerCase().includes(query))
    : allItems;

  if (selectedIndex >= filtered.length) selectedIndex = 0;

  listEl.innerHTML = "";

  if (filtered.length === 0) {
    listEl.classList.add("hidden");
    emptyEl.classList.remove("hidden");
    emptyEl.querySelector("p").textContent = query
      ? "No matches"
      : "No clipboard history yet";
    emptyEl.querySelector("span").textContent = query
      ? "Try a different search"
      : "Copy something to get started";
    return;
  }

  listEl.classList.remove("hidden");
  emptyEl.classList.add("hidden");

  filtered.forEach((item, index) => {
    const li = document.createElement("li");
    li.className = "item" + (index === selectedIndex ? " selected" : "");

    const text = document.createElement("div");
    text.className = "item-text";
    text.textContent = item;

    const meta = document.createElement("div");
    meta.className = "item-meta";
    const len = item.length;
    meta.textContent = `${len} character${len === 1 ? "" : "s"}`;

    li.appendChild(text);
    li.appendChild(meta);

    li.addEventListener("mouseenter", () => {
      selectedIndex = index;
      updateSelection();
    });

    li.addEventListener("click", () => {
      copyItem(item);
    });

    listEl.appendChild(li);
  });

  scrollSelectedIntoView();
}

function updateSelection() {
  [...listEl.children].forEach((li, index) => {
    li.classList.toggle("selected", index === selectedIndex);
  });
  scrollSelectedIntoView();
}

function scrollSelectedIntoView() {
  const el = listEl.children[selectedIndex];
  if (el) el.scrollIntoView({ block: "nearest" });
}

function copyItem(text) {
  invoke("copy_and_hide", { text });
}

async function loadHistory() {
  allItems = await invoke("get_history");
  render();
}

searchInput.addEventListener("input", () => {
  selectedIndex = 0;
  render();
});

searchInput.addEventListener("keydown", (e) => {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    if (filtered.length === 0) return;
    selectedIndex = (selectedIndex + 1) % filtered.length;
    updateSelection();
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    if (filtered.length === 0) return;
    selectedIndex = (selectedIndex - 1 + filtered.length) % filtered.length;
    updateSelection();
  } else if (e.key === "Enter") {
    e.preventDefault();
    const item = filtered[selectedIndex];
    if (item !== undefined) copyItem(item);
  } else if (e.key === "Escape") {
    e.preventDefault();
    invoke("hide_popup_cmd");
  }
});

listen("history-updated", (event) => {
  allItems = event.payload.items;
  render();
});

listen("popup-shown", () => {
  searchInput.value = "";
  selectedIndex = 0;
  loadHistory();
  searchInput.focus();
});

document.addEventListener("DOMContentLoaded", () => {
  searchInput.focus();
  loadHistory();
});
