const ENTRY_OFFSET_PX = 220;

function formatFileLabel(request) {
  if (request.files.length === 1) {
    return request.files[0].name;
  }
  if (request.files.length > 1) {
    return `${request.files.length} files`;
  }
  return request.urls[0] ?? "Link";
}

function createPreview(thumbnail) {
  if (!thumbnail) {
    const icon = document.createElement("div");
    icon.className = "flying-card-icon";
    return icon;
  }
  const image = document.createElement("img");
  image.className = "flying-card-thumbnail";
  image.alt = "";
  image.src = thumbnail;
  return image;
}

export function createFlyingCardLayer(layer, { peerSide = "right", enabled = true } = {}) {
  let activeCard = null;
  let side = peerSide;
  let isEnabled = enabled;

  function offscreenTransform() {
    const direction = side === "left" ? -1 : 1;
    const travel = window.innerWidth / 2 + ENTRY_OFFSET_PX;
    return `translate(calc(-50% + ${direction * travel}px), -50%) scale(0.5) rotate(${direction * -10}deg)`;
  }

  function removeActiveCard() {
    if (!activeCard) return;
    const card = activeCard;
    activeCard = null;
    card.classList.add("leaving");
    setTimeout(() => card.remove(), 400);
  }

  return {
    setPeerSide(newSide) {
      side = newSide === "left" ? "left" : "right";
    },
    setEnabled(shouldShow) {
      isEnabled = Boolean(shouldShow);
      if (!isEnabled) removeActiveCard();
    },
    arrive(request) {
      removeActiveCard();
      if (!isEnabled) return;
      const card = document.createElement("div");
      card.className = "flying-card";
      const label = document.createElement("div");
      label.className = "flying-card-label";
      label.textContent = formatFileLabel(request);
      const sender = document.createElement("div");
      sender.className = "flying-card-sender";
      sender.textContent = `from ${request.senderName}`;
      card.append(createPreview(request.thumbnail), label, sender);
      card.style.transform = offscreenTransform();
      layer.append(card);
      activeCard = card;
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          card.style.transform = "translate(-50%, -50%) scale(1.2) rotate(0deg)";
        });
      });
    },
    showThumbnail(thumbnail) {
      if (!activeCard || !thumbnail) return;
      const currentPreview = activeCard.querySelector(".flying-card-icon, .flying-card-thumbnail");
      currentPreview?.replaceWith(createPreview(thumbnail));
    },
    complete() {
      if (!activeCard) return;
      activeCard.classList.add("done");
      const card = activeCard;
      setTimeout(() => {
        if (activeCard === card) removeActiveCard();
      }, 900);
    },
    cancel() {
      removeActiveCard();
    },
  };
}
