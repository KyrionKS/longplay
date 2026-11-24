<script lang="ts">
  import type { Album, GrooveBand, Side } from "$lib/types";
  import { formatDuration } from "$lib/library.svelte";

  let {
    album,
    side,
    grooves = [],
  }: { album: Album; side: Side; grooves?: GrooveBand[] } = $props();

  const sideDuration = $derived(
    side.tracks.reduce((total, track) => total + track.duration + track.pregap, 0),
  );
  const durationLabel = $derived(
    sideDuration >= 60 ? `${Math.round(sideDuration / 60)} MIN` : formatDuration(sideDuration),
  );
  const rpm = $derived(side.rpm === "45" ? "45" : "33⅓");
  const testPressing = $derived(album.source === "playlist");
  const handRpm = $derived(rpm === "33⅓" ? "33 1/3" : rpm);


  let canvas: HTMLCanvasElement | undefined = $state();
  let host: HTMLDivElement | undefined = $state();
  let size = $state(0);
  let lastKey = "";

  $effect(() => {
    if (!host) return;
    const observer = new ResizeObserver((entries) => {
      size = entries[0]?.contentRect.width ?? host!.clientWidth;
    });
    observer.observe(host);
    size = host.clientWidth;
    return () => observer.disconnect();
  });

  $effect(() => {
    if (!canvas || size <= 0) return;
    const seed = `${album.id}:${side.label}`;
    const key = `${size}|${seed}|${grooves.length}|${grooves
      .map((band) => `${band.startNorm.toFixed(5)}:${band.endNorm.toFixed(5)}`)
      .join(",")}`;
    if (key === lastKey) return;
    lastKey = key;
    drawGrooves(canvas, size, grooves, seed);
  });

  function hashSeed(text: string): number {
    let hash = 2166136261;
    for (let index = 0; index < text.length; index += 1) {
      hash ^= text.charCodeAt(index);
      hash = Math.imul(hash, 16777619);
    }
    return hash >>> 0;
  }

  function makeRandom(seed: number) {
    let state = seed || 1;
    return () => {
      state = (state * 1664525 + 1013904223) >>> 0;
      return state / 4294967296;
    };
  }

  function drawGrooves(
    target: HTMLCanvasElement,
    width: number,
    bands: GrooveBand[],
    seed: string,
  ) {
    const dpr = Math.min(window.devicePixelRatio || 1, 3);
    const pixels = Math.max(1, Math.round(width * dpr));
    if (target.width !== pixels || target.height !== pixels) {
      target.width = pixels;
      target.height = pixels;
    }
    const context = target.getContext("2d");
    if (!context) return;

    context.setTransform(dpr, 0, 0, dpr, 0, 0);
    context.clearRect(0, 0, width, width);
    const center = width / 2;
    const tau = Math.PI * 2;

    for (const band of bands) {
      const outer = band.startNorm * center;
      const inner = band.endNorm * center;
      const bandWidth = outer - inner;
      if (bandWidth <= 0.15) continue;

      if (band.kind === "track") {
        for (let radius = outer - 0.35; radius > inner; radius -= 0.8) {
          const alpha = 0.07 + 0.045 * (0.5 + 0.5 * Math.sin(radius * 1.9));
          context.beginPath();
          context.arc(center, center, radius, 0, tau);
          context.strokeStyle = `rgba(255,255,255,${alpha.toFixed(3)})`;
          context.lineWidth = 0.6;
          context.stroke();
        }
        for (const [edge, alpha] of [
          [outer, 0.26],
          [inner, 0.1],
        ] as const) {
          context.beginPath();
          context.arc(center, center, edge, 0, tau);
          context.strokeStyle = `rgba(255,255,255,${alpha})`;
          context.lineWidth = 1;
          context.stroke();
        }
      } else if (band.kind === "gap") {
        // The land between songs is smooth, ungrooved lacquer: a darker, glossy
        // band with a faint edge on each side so it reads as a gap.
        context.beginPath();
        context.arc(center, center, (outer + inner) / 2, 0, tau);
        context.strokeStyle = "rgba(0,0,0,0.38)";
        context.lineWidth = bandWidth;
        context.stroke();
        for (const edge of [outer, inner]) {
          context.beginPath();
          context.arc(center, center, edge, 0, tau);
          context.strokeStyle = "rgba(255,255,255,0.18)";
          context.lineWidth = 0.6;
          context.stroke();
        }
      } else {
        for (let radius = outer; radius > inner; radius -= 2.4) {
          context.beginPath();
          context.arc(center, center, radius, 0, tau);
          context.strokeStyle = "rgba(255,255,255,0.03)";
          context.lineWidth = 0.5;
          context.stroke();
        }
      }
    }

    // Dust and the odd hairline scratch, deterministic per album and side.
    const random = makeRandom(hashSeed(seed));
    for (let index = 0; index < 62; index += 1) {
      const angle = random() * tau;
      const radius = (0.36 + random() * 0.62) * center;
      const x = center + Math.cos(angle) * radius;
      const y = center + Math.sin(angle) * radius;
      context.beginPath();
      context.arc(x, y, 0.5 + random() * 1.4, 0, tau);
      context.fillStyle =
        random() > 0.5 ? "rgba(255,255,255,0.16)" : "rgba(0,0,0,0.2)";
      context.fill();
    }

    for (let index = 0; index < 3; index += 1) {
      const start = random() * tau;
      const radius = (0.45 + random() * 0.45) * center;
      const length = 0.15 + random() * 0.5;
      context.beginPath();
      context.arc(center, center, radius, start, start + length);
      context.strokeStyle =
        index === 0 ? "rgba(255,255,255,0.07)" : "rgba(0,0,0,0.055)";
      context.lineWidth = 0.5 + random() * 0.6;
      context.stroke();
    }
  }
</script>

<div
  class="vinyl"
  class:single={side.rpm === "45"}
  class:test-pressing={testPressing}
  bind:this={host}
  style:--vinyl={testPressing ? "#171717" : album.palette.vinyl}
  style:--light={testPressing ? "#2c2c2c" : album.palette.vinylLight}
  style:--label={testPressing ? "#efe9da" : album.palette.label}
  style:--ink={testPressing ? "#1f2228" : album.palette.dark}
>
  <div class="disc-layer" aria-hidden="true">
    <div class="grooves"></div>
    <div class="warp" aria-hidden="true"></div>
    <canvas class="grooves-canvas" bind:this={canvas} aria-hidden="true"></canvas>

    <div class="label" class:white={testPressing}>
      <div class="label-wear" aria-hidden="true"></div>
      {#if testPressing}
        <span class="stamp">Test Pressing</span>
        <span class="hand-title">{album.title}</span>
        <span class="hand-side">Seite {side.label}</span>
        <span class="hand-meta">{handRpm} RPM · {side.tracks.length} Tracks</span>
        <span class="hand-meta tiny">Promo · Not for Sale</span>
      {:else}
        <div class="top">
          <span class="cat">{side.matrix ?? "LONGPLAY"}</span>
          <span class="title">{album.title}</span>
        </div>
        <div class="bottom">
          <span class="artist">{album.albumArtist}</span>
          <span class="meta">SEITE {side.label} · {rpm} RPM</span>
          <span class="meta dim">{side.tracks.length} TRACKS · {durationLabel}</span>
        </div>
      {/if}
    </div>
  </div>

  <div class="sheen" aria-hidden="true"></div>
  <div class="edge" aria-hidden="true"></div>
  <div class="spindle" aria-hidden="true"></div>
</div>

<style>
  .vinyl {
    position: absolute;
    inset: 3.5%;
    border-radius: 50%;
    background:
      radial-gradient(circle at 36% 28%, rgba(255, 255, 255, 0.14), transparent 32%),
      radial-gradient(circle at 68% 76%, rgba(0, 0, 0, 0.4), transparent 55%),
      radial-gradient(
        circle at 50% 50%,
        color-mix(in srgb, var(--light) 60%, var(--vinyl)) 0%,
        var(--vinyl) 52%,
        #0a0a0a 82%,
        #191919 100%
      );
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.65),
      0 18px 34px rgba(0, 0, 0, 0.55),
      0 0 0 3px rgba(255, 255, 255, 0.04) inset;
    container-type: inline-size;
    animation: vinyl-in 620ms cubic-bezier(0.2, 0.85, 0.2, 1) both;
  }

  .vinyl.single {
    inset: 23%;
  }

  .vinyl.single .label {
    width: 49%;
  }

  .vinyl.single .title {
    font-size: 2.9cqw;
  }

  .vinyl.single .artist {
    font-size: 1.9cqw;
  }

  .vinyl.single .cat,
  .vinyl.single .meta {
    font-size: 1.4cqw;
  }

  .disc-layer {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    will-change: transform;
  }

  .warp {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      radial-gradient(36% 16% at 68% 36%, rgba(255, 255, 255, 0.11), transparent 65%),
      radial-gradient(28% 12% at 28% 72%, rgba(255, 255, 255, 0.055), transparent 70%);
    mix-blend-mode: screen;
    pointer-events: none;
  }

  .grooves-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    pointer-events: none;
  }

  .grooves {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      repeating-radial-gradient(
        circle at 50% 50%,
        rgba(255, 255, 255, 0.045) 0 0.6px,
        rgba(0, 0, 0, 0.10) 0.6px 2.4px,
        transparent 2.4px 3.6px
      );
    mask: radial-gradient(circle at 50% 50%, transparent 0 34%, #000 35.5% 100%);
    opacity: 0.3;
  }

  .sheen {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      radial-gradient(46% 24% at 30% 22%, rgba(255, 255, 255, 0.13), transparent 70%),
      radial-gradient(40% 18% at 74% 82%, rgba(255, 255, 255, 0.05), transparent 70%);
    mix-blend-mode: screen;
    pointer-events: none;
  }

  .edge {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.10) inset,
      0 0 0 3px rgba(0, 0, 0, 0.35) inset;
    pointer-events: none;
  }

  .label {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 33.5%;
    aspect-ratio: 1;
    transform: translate(-50%, -50%);
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    padding: 11% 12%;
    text-align: center;
    color: var(--ink);
    background:
      radial-gradient(circle at 38% 30%, rgba(255, 255, 255, 0.85), transparent 55%),
      radial-gradient(circle at 50% 50%, var(--label), color-mix(in srgb, var(--label) 78%, #b9a97f) 100%);
    box-shadow:
      0 2px 8px rgba(0, 0, 0, 0.5),
      0 0 0 1px rgba(0, 0, 0, 0.35) inset,
      0 0 0 4px rgba(0, 0, 0, 0.12) inset;
  }

  .top,
  .bottom {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    width: 100%;
  }

  .bottom {
    position: absolute;
    left: 14%;
    right: 14%;
    bottom: 13%;
  }

  .label::before {
    content: "";
    position: absolute;
    inset: 6%;
    border-radius: 50%;
    border: 1px solid rgba(0, 0, 0, 0.18);
  }

  .label-wear {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    pointer-events: none;
    background:
      radial-gradient(circle at 50% 50%, transparent 60%, rgba(120, 95, 55, 0.26) 70%, transparent 78%),
      radial-gradient(circle at 32% 22%, transparent 36%, rgba(110, 90, 50, 0.22) 66%, transparent 80%),
      radial-gradient(circle at 76% 78%, rgba(255, 255, 255, 0.32), transparent 52%),
      repeating-conic-gradient(
        from 12deg,
        rgba(90, 70, 40, 0.05) 0 3deg,
        transparent 3deg 14deg
      );
    mix-blend-mode: multiply;
  }

  .cat {
    font-family: var(--font-display);
    font-size: 1.18cqw;
    letter-spacing: 0.18em;
    opacity: 0.62;
    white-space: nowrap;
  }

  .title {
    font-family: var(--font-display);
    font-size: 2.35cqw;
    line-height: 1.08;
    letter-spacing: 0.01em;
    max-width: 100%;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .artist {
    font-family: var(--font-display);
    font-size: 1.6cqw;
    font-style: italic;
    opacity: 0.82;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    font-size: 1.18cqw;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    opacity: 0.75;
    white-space: nowrap;
  }

  .meta.dim {
    opacity: 0.55;
  }

  .label.white {
    display: block;
    padding: 0;
    overflow: hidden;
    color: #20232b;
    background-color: #f2ecdd;
    background-image:
      radial-gradient(circle at 36% 28%, rgba(255, 255, 255, 0.95), transparent 58%),
      url("/textures/paper-soft.jpg");
    background-size: auto, 120px 120px;
    background-blend-mode: screen, multiply;
  }

  .label.white::before {
    border-color: rgba(60, 52, 38, 0.12);
  }

  .label.white .label-wear {
    opacity: 0.35;
  }

  .stamp {
    position: absolute;
    top: 7%;
    left: 50%;
    padding: 0.3em 0.5em;
    border: 1px solid rgba(148, 34, 34, 0.6);
    border-radius: 1px;
    color: rgba(148, 34, 34, 0.78);
    font-family: var(--font-display);
    font-size: 1.55cqw;
    letter-spacing: 0.14em;
    line-height: 1;
    text-transform: uppercase;
    transform: translateX(-50%) rotate(-8deg);
    white-space: nowrap;
  }

  .hand-title,
  .hand-side,
  .hand-meta {
    position: absolute;
    left: 9%;
    right: 9%;
    font-family: var(--font-hand);
    text-align: center;
    line-height: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .hand-title {
    top: 20%;
    max-height: 2.1em;
    font-size: 2.9cqw;
    line-height: 0.98;
    white-space: normal;
    transform: rotate(-2deg);
    color: #232b3b;
  }

  .hand-side {
    top: 57%;
    font-size: 2.2cqw;
    transform: rotate(-1.2deg);
  }

  .hand-meta {
    top: 67%;
    font-size: 1.42cqw;
    opacity: 0.82;
    transform: rotate(-1deg);
  }

  .hand-meta.tiny {
    top: 76%;
    font-size: 1.08cqw;
    letter-spacing: 0.04em;
    opacity: 0.6;
  }

  .vinyl.single .hand-title {
    top: 13%;
    font-size: 3.4cqw;
  }

  .vinyl.single .hand-side {
    top: 58%;
    font-size: 2.4cqw;
  }

  .vinyl.single .hand-meta {
    top: 68%;
    font-size: 1.5cqw;
  }

  .vinyl.single .stamp {
    top: 10%;
    font-size: 1.1cqw;
  }

  .spindle {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 3.4%;
    aspect-ratio: 1;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    background: radial-gradient(circle at 35% 30%, #fdfdfd, #9a9a9a 55%, #3a3a3a);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.75),
      0 0 0 1px rgba(0, 0, 0, 0.45),
      0 0 0 4px rgba(120, 95, 55, 0.2);
  }

  .vinyl.single .spindle {
    width: 12%;
    background: radial-gradient(
      circle at 50% 50%,
      #181818 0 24%,
      #d8d8d8 27% 62%,
      #8a8a8a 66% 100%
    );
  }

  @keyframes vinyl-in {
    from {
      opacity: 0;
      transform: rotateY(74deg) scale(0.94);
      filter: brightness(0.55);
    }
    to {
      opacity: 1;
      transform: rotateY(0deg) scale(1);
      filter: brightness(1);
    }
  }
</style>
