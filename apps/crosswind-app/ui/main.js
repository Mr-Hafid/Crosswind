import { createEdgeGlow } from "./glow.js";
import { createFlyingCardLayer } from "./flying-card.js";
import {
  connectGlowToTransfers,
  readCardEnabled,
  readPeerSide,
  saveCardEnabled,
  savePeerSide,
} from "./transfer-visuals.js";

const { invoke } = window.__TAURI__.core;
const { listen, emit } = window.__TAURI__.event;

const SETTINGS_KEY = "crosswind.settings";
const RESUME_RECEIVING_KEY = "crosswind.resumeReceiving";
const PREVIEW_REQUEST = {
  id: -1,
  senderName: "Preview",
  senderModel: "iPhone",
  files: [{ name: "Photo.heic", fileType: "public.heic", size: 2400000 }],
  urls: [],
  autoAccepted: true,
};

const elements = {
  statusDot: document.getElementById("status-dot"),
  statusTitle: document.getElementById("status-title"),
  statusDetail: document.getElementById("status-detail"),
  statusError: document.getElementById("status-error"),
  toggleReceiving: document.getElementById("toggle-receiving"),
  incomingSection: document.getElementById("incoming-section"),
  incomingList: document.getElementById("incoming-list"),
  receivedList: document.getElementById("received-list"),
  receivedEmpty: document.getElementById("received-empty"),
  interfaceSelect: document.getElementById("interface-select"),
  downloadDir: document.getElementById("download-dir"),
  autoAccept: document.getElementById("auto-accept"),
  peerSide: document.getElementById("peer-side"),
  showFileCard: document.getElementById("show-file-card"),
  launchAtLogin: document.getElementById("launch-at-login"),
  launchAtLoginRow: document.getElementById("launch-at-login-row"),
  previewGlow: document.getElementById("preview-glow"),
};

const isMobile = /Android|iPhone|iPad/i.test(navigator.userAgent);
let receiverStatus = null;
const incomingRequests = new Map();

function loadSettings() {
  try {
    return JSON.parse(localStorage.getItem(SETTINGS_KEY)) ?? {};
  } catch {
    return {};
  }
}

function saveSettings() {
  try {
    localStorage.setItem(
      SETTINGS_KEY,
      JSON.stringify({
        interfaceName: elements.interfaceSelect.value,
        downloadDir: elements.downloadDir.value,
        autoAccept: elements.autoAccept.checked,
      }),
    );
  } catch {}
}

function formatBytes(size) {
  if (size == null) return "";
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(0)} KB`;
  return `${(size / 1024 / 1024).toFixed(1)} MB`;
}

function fileNameOf(path) {
  return path.split(/[\\/]/).pop();
}

function showError(message) {
  elements.statusError.textContent = message ?? "";
  elements.statusError.hidden = !message;
}

function renderStatus() {
  const isReceiving = receiverStatus !== null;
  elements.statusDot.classList.toggle("ready", isReceiving);
  elements.toggleReceiving.classList.toggle("stop", isReceiving);
  elements.toggleReceiving.textContent = isReceiving ? "Stop receiving" : "Start receiving";
  if (isReceiving) {
    elements.statusTitle.textContent = `Receiving as ${receiverStatus.displayName}`;
    elements.statusDetail.textContent = `On ${receiverStatus.interfaceName}. Files go to ${receiverStatus.downloadDir}.`;
  } else {
    elements.statusTitle.textContent = "Not receiving";
    elements.statusDetail.textContent = "Start receiving so nearby iPhones can find this device in AirDrop.";
  }
}

function renderIncoming() {
  elements.incomingList.replaceChildren();
  for (const request of incomingRequests.values()) {
    const item = document.createElement("li");
    item.className = "incoming-item";
    const sender = document.createElement("div");
    sender.className = "incoming-sender";
    sender.textContent = `${request.senderName} (${request.senderModel})`;
    const files = document.createElement("ul");
    files.className = "incoming-files";
    for (const file of request.files) {
      const entry = document.createElement("li");
      entry.textContent = [file.name, formatBytes(file.size)].filter(Boolean).join(", ");
      files.append(entry);
    }
    for (const url of request.urls) {
      const entry = document.createElement("li");
      entry.textContent = url;
      files.append(entry);
    }
    item.append(sender, files);
    if (!request.autoAccepted) {
      const actions = document.createElement("div");
      actions.className = "incoming-actions";
      const accept = document.createElement("button");
      accept.type = "button";
      accept.className = "small-button accept";
      accept.textContent = "Accept";
      accept.addEventListener("click", () => answer(request.id, true));
      const decline = document.createElement("button");
      decline.type = "button";
      decline.className = "small-button";
      decline.textContent = "Decline";
      decline.addEventListener("click", () => answer(request.id, false));
      actions.append(accept, decline);
      item.append(actions);
    }
    elements.incomingList.append(item);
  }
  elements.incomingSection.hidden = incomingRequests.size === 0;
}

function addReceived({ paths, urls }, senderName) {
  for (const path of paths) {
    const item = document.createElement("li");
    item.className = "received-item";
    const label = document.createElement("div");
    const name = document.createElement("span");
    name.className = "received-name";
    name.textContent = fileNameOf(path);
    const meta = document.createElement("span");
    meta.className = "received-meta";
    meta.textContent = senderName ? `from ${senderName}` : "";
    label.append(name, meta);
    const reveal = document.createElement("button");
    reveal.type = "button";
    reveal.className = "small-button";
    reveal.textContent = "Show";
    reveal.addEventListener("click", () =>
      invoke("reveal_received", { path }).catch((error) => showError(String(error))),
    );
    item.append(label, reveal);
    elements.receivedList.prepend(item);
  }
  for (const url of urls) {
    const item = document.createElement("li");
    item.className = "received-item";
    const name = document.createElement("span");
    name.className = "received-name";
    name.textContent = url;
    item.append(name);
    elements.receivedList.prepend(item);
  }
  elements.receivedEmpty.hidden = elements.receivedList.children.length > 0;
}

async function answer(id, accept) {
  try {
    await invoke("answer_request", { id, accept });
  } catch (error) {
    showError(String(error));
  }
  const request = incomingRequests.get(id);
  if (request) {
    request.autoAccepted = true;
    if (!accept) incomingRequests.delete(id);
  }
  renderIncoming();
}

async function toggleReceiving() {
  elements.toggleReceiving.disabled = true;
  showError(null);
  try {
    if (receiverStatus) {
      await invoke("stop_receiving");
      receiverStatus = null;
      rememberReceivingChoice(false);
    } else {
      saveSettings();
      receiverStatus = await invoke("start_receiving", {
        interfaceName: elements.interfaceSelect.value,
        downloadDir: elements.downloadDir.value,
        autoAccept: elements.autoAccept.checked,
      });
      rememberReceivingChoice(true);
    }
  } catch (error) {
    showError(String(error));
  } finally {
    elements.toggleReceiving.disabled = false;
    renderStatus();
  }
}

function rememberReceivingChoice(isReceiving) {
  try {
    localStorage.setItem(RESUME_RECEIVING_KEY, String(isReceiving));
  } catch {}
}

function shouldResumeReceiving() {
  try {
    return localStorage.getItem(RESUME_RECEIVING_KEY) === "true";
  } catch {
    return false;
  }
}

async function populateLaunchAtLogin() {
  try {
    elements.launchAtLogin.checked = await invoke("launch_at_login_enabled");
  } catch {
    elements.launchAtLoginRow.hidden = true;
  }
}

async function changeLaunchAtLogin() {
  const wanted = elements.launchAtLogin.checked;
  try {
    elements.launchAtLogin.checked = await invoke("set_launch_at_login", { enabled: wanted });
  } catch (error) {
    elements.launchAtLogin.checked = !wanted;
    showError(`Could not change launch at login: ${error}`);
  }
}

async function populateSettings() {
  const saved = loadSettings();
  const interfaces = await invoke("list_interfaces");
  elements.interfaceSelect.replaceChildren();
  for (const choice of interfaces) {
    const option = document.createElement("option");
    option.value = choice.name;
    option.textContent = `${choice.name}  ${choice.address}`;
    elements.interfaceSelect.append(option);
  }
  const preferred =
    interfaces.find((choice) => choice.name === saved.interfaceName) ??
    interfaces.find((choice) => choice.isDefault) ??
    interfaces[0];
  if (preferred) elements.interfaceSelect.value = preferred.name;
  if (interfaces.length === 0) {
    showError("No network interface with an IPv6 link-local address was found.");
  }

  elements.downloadDir.value = saved.downloadDir ?? (await invoke("default_download_dir").catch(() => ""));
  elements.autoAccept.checked = Boolean(saved.autoAccept);
  elements.peerSide.value = readPeerSide();
  elements.showFileCard.checked = readCardEnabled();
}

function previewGlow() {
  emit("incoming-request", PREVIEW_REQUEST);
  invoke("preview_glow", { phase: "incoming" });
  setTimeout(() => {
    emit("transfer-finished", { id: -1, paths: [], urls: [], reason: null });
    invoke("preview_glow", { phase: "received" });
  }, 2200);
}

function setUpInAppGlow() {
  const canvas = document.getElementById("in-app-glow");
  const cardLayer = document.getElementById("in-app-cards");
  if (!isMobile) {
    canvas.hidden = true;
    cardLayer.hidden = true;
    return;
  }
  const peerSide = readPeerSide();
  connectGlowToTransfers({
    glow: createEdgeGlow(canvas, { peerSide }),
    cards: createFlyingCardLayer(cardLayer, { peerSide, enabled: readCardEnabled() }),
  });
}

function listenToTransfers() {
  listen("incoming-request", (event) => {
    if (event.payload.id < 0) return;
    incomingRequests.set(event.payload.id, event.payload);
    renderIncoming();
  });
  listen("transfer-finished", (event) => {
    if (event.payload.id < 0) return;
    const request = incomingRequests.get(event.payload.id);
    incomingRequests.delete(event.payload.id);
    renderIncoming();
    addReceived(event.payload, request?.senderName);
  });
  listen("system-airdrop-received", (event) => {
    addReceived(event.payload, "macOS AirDrop");
  });
  listen("transfer-failed", (event) => {
    incomingRequests.delete(event.payload.id);
    renderIncoming();
    showError(`Transfer failed: ${event.payload.reason}`);
  });
  listen("transfer-withdrawn", (event) => {
    incomingRequests.delete(event.payload);
    renderIncoming();
  });
}

elements.toggleReceiving.addEventListener("click", toggleReceiving);
elements.previewGlow.addEventListener("click", previewGlow);
listen("preview-glow-requested", previewGlow);
elements.peerSide.addEventListener("change", () => {
  savePeerSide(elements.peerSide.value);
  emit("peer-side-changed", elements.peerSide.value);
});
elements.launchAtLogin.addEventListener("change", changeLaunchAtLogin);
elements.showFileCard.addEventListener("change", () => {
  saveCardEnabled(elements.showFileCard.checked);
  emit("card-visibility-changed", elements.showFileCard.checked);
});
[elements.interfaceSelect, elements.downloadDir, elements.autoAccept].forEach((element) =>
  element.addEventListener("change", saveSettings),
);

setUpInAppGlow();
listenToTransfers();
await populateSettings();
await populateLaunchAtLogin();
receiverStatus = await invoke("receiver_status");
renderStatus();
if (!receiverStatus && shouldResumeReceiving()) {
  await toggleReceiving();
}
