const PEER_SIDE_KEY = "crosswind.peerSide";
const CARD_ENABLED_KEY = "crosswind.showFileCard";

export function readPeerSide() {
  try {
    return localStorage.getItem(PEER_SIDE_KEY) === "left" ? "left" : "right";
  } catch {
    return "right";
  }
}

export function savePeerSide(side) {
  try {
    localStorage.setItem(PEER_SIDE_KEY, side);
  } catch {}
}

export function readCardEnabled() {
  try {
    return localStorage.getItem(CARD_ENABLED_KEY) !== "false";
  } catch {
    return true;
  }
}

export function saveCardEnabled(enabled) {
  try {
    localStorage.setItem(CARD_ENABLED_KEY, String(enabled));
  } catch {}
}

export function connectGlowToTransfers({ glow, cards }) {
  const { listen } = window.__TAURI__.event;
  listen("glow", (event) => glow.play(event.payload));
  listen("incoming-request", (event) => cards.arrive(event.payload));
  listen("card-thumbnail", (event) => cards.showThumbnail(event.payload.thumbnail));
  listen("transfer-finished", () => cards.complete());
  listen("transfer-failed", () => cards.cancel());
  listen("transfer-withdrawn", () => cards.cancel());
  listen("peer-side-changed", (event) => {
    glow.setPeerSide(event.payload);
    cards.setPeerSide(event.payload);
  });
  listen("card-visibility-changed", (event) => cards.setEnabled(event.payload));
}
