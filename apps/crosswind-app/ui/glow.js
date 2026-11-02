const VERTEX_SOURCE = `
attribute vec2 corner;
void main() {
  gl_Position = vec4(corner, 0.0, 1.0);
}
`;

const FRAGMENT_SOURCE = `
precision highp float;

uniform vec2 canvasSize;
uniform vec2 origin;
uniform float time;
uniform float intensity;
uniform float waveElapsed;
uniform float horizontalDirection;
uniform float pixelRatio;

const float sweepOriginOffset = 0.45;
const float sweepSpeed = 1.1;
const float sweepDuration = 1.3;

float squared(float value) {
  return value * value;
}

float cellHash(vec2 cell) {
  cell = fract(cell * vec2(123.34, 456.21));
  cell += dot(cell, cell + 45.32);
  return fract(cell.x * cell.y);
}

float valueNoise(vec2 point) {
  vec2 cell = floor(point);
  vec2 local = fract(point);
  vec2 smoothed = local * local * (3.0 - 2.0 * local);
  float a = cellHash(cell);
  float b = cellHash(cell + vec2(1.0, 0.0));
  float c = cellHash(cell + vec2(0.0, 1.0));
  float d = cellHash(cell + vec2(1.0, 1.0));
  return mix(mix(a, b, smoothed.x), mix(c, d, smoothed.x), smoothed.y);
}

vec3 dispersionTint(float signedOffset) {
  vec3 innerEdge = vec3(0.78, 1.00, 0.84);
  vec3 outerEdge = vec3(0.96, 0.82, 1.00);
  vec3 neutral = vec3(1.0);
  return signedOffset < 0.0
    ? mix(neutral, innerEdge, clamp(-signedOffset, 0.0, 1.0))
    : mix(neutral, outerEdge, clamp(signedOffset, 0.0, 1.0));
}

void main() {
  vec2 position = vec2(gl_FragCoord.x, canvasSize.y * pixelRatio - gl_FragCoord.y) / pixelRatio;
  float scale = max(canvasSize.x, canvasSize.y);
  vec2 offset = (position - origin) / scale;
  float distanceFromOrigin = length(offset);

  float breathing = 1.0 + 0.05 * sin(time * 2.4) + 0.025 * sin(time * 4.1 + 1.3);
  float isLandscape = step(canvasSize.y, canvasSize.x);
  float bloomRadius = max(intensity * mix(0.26, 0.42, isLandscape) * breathing, 0.0001);
  vec2 stretched = vec2(offset.x * 1.35, offset.y * 0.72);
  float stretchedDistance = length(stretched);

  float shimmer = 0.85 + 0.3 * valueNoise(position / 38.0 + vec2(time * 0.6 * horizontalDirection, time * 0.25));
  float core = exp(-squared(stretchedDistance / (bloomRadius * 0.55))) * intensity;
  float halo = exp(-squared(stretchedDistance / bloomRadius) * 1.6) * intensity * 0.55 * shimmer;
  float edgeLine = exp(-squared(abs(position.x - origin.x) / 3.0))
    * exp(-squared((position.y - origin.y) / (scale * bloomRadius * 1.3))) * intensity;

  vec2 sweepOrigin = vec2(origin.x - horizontalDirection * sweepOriginOffset * scale, origin.y) / scale;
  float sweepDistance = length(position / scale - sweepOrigin);
  float waveRadius = sweepOriginOffset + waveElapsed * sweepSpeed;
  float waveIsActive = step(0.0001, waveElapsed) * step(waveElapsed, sweepDuration);
  float waveFade = pow(clamp(1.0 - waveElapsed / sweepDuration, 0.0, 1.0), 0.8) * waveIsActive;
  float bandWidth = 0.03;
  float signedBandOffset = (sweepDistance - waveRadius) / bandWidth;
  float waveBand = exp(-signedBandOffset * signedBandOffset) * waveFade * 0.6;
  float fringeBand = exp(-squared(abs(signedBandOffset) - 1.5) * 1.2) * waveFade * 0.35;
  float flash = exp(-squared(distanceFromOrigin / 0.3)) * exp(-waveElapsed * 6.0) * waveIsActive * 0.8;

  float peerIsRight = step(horizontalDirection, 0.0);
  float distanceToPeerEdge = peerIsRight > 0.5 ? canvasSize.x - position.x : position.x;
  float distanceToTop = position.y;
  float distanceToBottom = canvasSize.y - position.y;
  float distanceToNearestRim = min(distanceToPeerEdge, min(distanceToTop, distanceToBottom));
  float pathAlongPeerEdge = abs(position.y - origin.y);
  float pathAlongTop = origin.y + distanceToPeerEdge;
  float pathAlongBottom = (canvasSize.y - origin.y) + distanceToPeerEdge;
  float perimeterPath = distanceToPeerEdge <= min(distanceToTop, distanceToBottom)
    ? pathAlongPeerEdge
    : (distanceToTop < distanceToBottom ? pathAlongTop : pathAlongBottom);
  float rimFalloff = scale * 0.5;
  float rimThickness = scale * mix(0.03, 0.05, isLandscape) * exp(-perimeterPath / rimFalloff) + 1.5;
  float rimShimmer = 0.85 + 0.15 * valueNoise(vec2(perimeterPath / 60.0 - time * 1.2, time * 0.4));
  float screenRim = exp(-distanceToNearestRim / rimThickness) * exp(-perimeterPath / (rimFalloff * 1.3)) * intensity * 0.85 * rimShimmer;

  vec2 sparkleCell = floor(position / 18.0);
  float sparkleSeed = cellHash(sparkleCell);
  vec2 sparkleCenter = (sparkleCell + 0.2 + 0.6 * vec2(cellHash(sparkleCell + 7.1), cellHash(sparkleCell + 3.7))) * 18.0;
  vec2 sparkleDelta = abs(position - sparkleCenter);
  float sparkleStar = exp(-squared(length(sparkleDelta) / 1.1))
    + 0.5 * exp(-sparkleDelta.x / 0.6 - sparkleDelta.y / 4.0)
    + 0.5 * exp(-sparkleDelta.y / 0.6 - sparkleDelta.x / 4.0);
  float twinkle = step(0.9, sparkleSeed) * pow(0.5 + 0.5 * sin(time * 6.0 + sparkleSeed * 90.0), 4.0);
  float sparkleRegion = (1.0 - smoothstep(bloomRadius * 0.4, bloomRadius * 1.2, stretchedDistance)) * intensity
    + exp(-signedBandOffset * signedBandOffset * 0.15) * waveFade;
  float sparkle = min(sparkleStar, 1.0) * twinkle * sparkleRegion;

  float whiteAmount = core + halo + edgeLine + waveBand + flash + sparkle + screenRim;
  float fringeAmount = fringeBand;
  float coverage = whiteAmount + fringeAmount;
  if (coverage < 0.002) {
    gl_FragColor = vec4(0.0);
    return;
  }

  vec3 glowTone = mix(vec3(1.0), vec3(0.84, 0.97, 0.90), clamp(stretchedDistance / bloomRadius, 0.0, 1.0));
  vec3 color = (glowTone * (core + halo + flash) + vec3(0.92, 1.0, 0.95) * screenRim
    + vec3(1.0) * (edgeLine + sparkle + waveBand)
    + dispersionTint(sign(signedBandOffset)) * fringeAmount) / coverage;
  float alpha = clamp(coverage, 0.0, 1.0);
  gl_FragColor = vec4(color * alpha, alpha);
}
`;

const WAVE_DURATION_MS = 1300;

const easings = {
  easeOut: (progress) => 1 - Math.pow(1 - progress, 3),
  easeIn: (progress) => progress * progress * progress,
  easeInOut: (progress) =>
    progress < 0.5 ? 4 * progress * progress * progress : 1 - Math.pow(-2 * progress + 2, 3) / 2,
};

function compileShader(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader) || "glow shader failed to compile");
  }
  return shader;
}

function createGlowProgram(gl) {
  const program = gl.createProgram();
  gl.attachShader(program, compileShader(gl, gl.VERTEX_SHADER, VERTEX_SOURCE));
  gl.attachShader(program, compileShader(gl, gl.FRAGMENT_SHADER, FRAGMENT_SOURCE));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program) || "glow program failed to link");
  }
  return program;
}

export function createEdgeGlow(canvas, { peerSide = "right", verticalPosition = 0.5 } = {}) {
  const gl = canvas.getContext("webgl", { premultipliedAlpha: true, alpha: true, antialias: false });
  if (!gl) {
    throw new Error("WebGL is not available");
  }
  const program = createGlowProgram(gl);
  gl.useProgram(program);

  const cornerBuffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, cornerBuffer);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
  const cornerLocation = gl.getAttribLocation(program, "corner");
  gl.enableVertexAttribArray(cornerLocation);
  gl.vertexAttribPointer(cornerLocation, 2, gl.FLOAT, false, 0, 0);
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);

  const uniforms = Object.fromEntries(
    ["canvasSize", "origin", "time", "intensity", "waveElapsed", "horizontalDirection", "pixelRatio"].map(
      (name) => [name, gl.getUniformLocation(program, name)],
    ),
  );

  const clockOrigin = performance.now();
  const state = {
    intensity: 0,
    intensityTween: null,
    waveStartedAt: null,
    peerSide,
    verticalPosition,
    frameRequest: null,
    scheduledSteps: [],
  };

  function resizeToDisplay() {
    const pixelRatio = window.devicePixelRatio || 1;
    const width = Math.round(canvas.clientWidth * pixelRatio);
    const height = Math.round(canvas.clientHeight * pixelRatio);
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }
    gl.viewport(0, 0, width, height);
    return pixelRatio;
  }

  function waveElapsedSeconds(now) {
    if (state.waveStartedAt === null) {
      return 0;
    }
    const elapsed = now - state.waveStartedAt;
    return elapsed >= WAVE_DURATION_MS ? 0 : Math.max(elapsed, 0) / 1000;
  }

  function isWaveRunning(now) {
    return state.waveStartedAt !== null && now - state.waveStartedAt < WAVE_DURATION_MS;
  }

  function advanceIntensity(now) {
    const tween = state.intensityTween;
    if (!tween) {
      return;
    }
    const progress = Math.min((now - tween.startedAt) / tween.durationMs, 1);
    state.intensity = tween.from + (tween.to - tween.from) * tween.easing(progress);
    if (progress >= 1) {
      state.intensityTween = null;
    }
  }

  function isIdle(now) {
    return state.intensity < 0.001 && !state.intensityTween && !isWaveRunning(now);
  }

  function drawFrame(now) {
    state.frameRequest = null;
    advanceIntensity(now);
    const pixelRatio = resizeToDisplay();
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    if (isIdle(now)) {
      canvas.style.opacity = "0";
      return;
    }
    canvas.style.opacity = "1";
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    const originX = state.peerSide === "left" ? 0 : width;
    const originY = height * Math.min(Math.max(state.verticalPosition, 0.1), 0.9);
    gl.uniform2f(uniforms.canvasSize, width, height);
    gl.uniform2f(uniforms.origin, originX, originY);
    gl.uniform1f(uniforms.time, (now - clockOrigin) / 1000);
    gl.uniform1f(uniforms.intensity, state.intensity);
    gl.uniform1f(uniforms.waveElapsed, waveElapsedSeconds(now));
    gl.uniform1f(uniforms.horizontalDirection, state.peerSide === "left" ? 1 : -1);
    gl.uniform1f(uniforms.pixelRatio, pixelRatio);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    state.frameRequest = requestAnimationFrame(drawFrame);
  }

  function ensureRendering() {
    if (state.frameRequest === null) {
      state.frameRequest = requestAnimationFrame(drawFrame);
    }
  }

  function animateIntensity(target, durationMs, easing) {
    state.intensityTween = {
      from: state.intensity,
      to: target,
      startedAt: performance.now(),
      durationMs,
      easing: easings[easing],
    };
    ensureRendering();
  }

  function startWave() {
    state.waveStartedAt = performance.now();
    ensureRendering();
  }

  function cancelScheduledSteps() {
    state.scheduledSteps.forEach((timer) => clearTimeout(timer));
    state.scheduledSteps = [];
  }

  function after(delayMs, step) {
    state.scheduledSteps.push(setTimeout(step, delayMs));
  }

  const phases = {
    nearby() {
      animateIntensity(0.35, 250, "easeOut");
      after(2500, () => animateIntensity(0, 600, "easeInOut"));
    },
    incoming() {
      startWave();
      animateIntensity(1, 250, "easeOut");
      after(700, () => animateIntensity(0.4, 600, "easeInOut"));
    },
    received() {
      after(900, () => animateIntensity(0, 600, "easeOut"));
    },
    idle() {
      animateIntensity(0, 300, "easeOut");
    },
  };

  return {
    play(phase) {
      const startPhase = phases[phase];
      if (!startPhase) {
        return;
      }
      cancelScheduledSteps();
      startPhase();
    },
    setPeerSide(side) {
      state.peerSide = side === "left" ? "left" : "right";
    },
  };
}
