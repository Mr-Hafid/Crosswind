import { createEdgeGlow } from "./glow.js";
import { createFlyingCardLayer } from "./flying-card.js";
import { connectGlowToTransfers, readCardEnabled, readPeerSide } from "./transfer-visuals.js";

const peerSide = readPeerSide();
const glow = createEdgeGlow(document.getElementById("screen-glow"), { peerSide });
const cards = createFlyingCardLayer(document.getElementById("screen-cards"), {
  peerSide,
  enabled: readCardEnabled(),
});

connectGlowToTransfers({ glow, cards });
