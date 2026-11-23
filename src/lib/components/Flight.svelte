<script lang="ts">
  import type { Album } from "$lib/types";
  import { coverUrl } from "$lib/library.svelte";

  interface Rect {
    left: number;
    top: number;
    width: number;
    height: number;
  }

  interface Target {
    x: number;
    y: number;
    r: number;
  }

  let {
    album,
    from,
    target,
    onDone,
  }: { album: Album; from: Rect; target: Target; onDone: () => void } = $props();

  const cover = $derived(coverUrl(album));
  const playlist = $derived(album.source === "playlist");

  let viewW = $state(window.innerWidth);
  let viewH = $state(window.innerHeight);

  const size = $derived(Math.min(viewW * 0.46, viewH * 0.56));
  const destX = $derived(viewW * 0.5 - size / 2);
  const destY = $derived(viewH * 0.46 - size / 2);

  let lifted = $state(false);
  let phase = $state<"lift" | "draw" | "drop">("lift");

  const discTransform = $derived.by(() => {
    if (phase === "draw") {
      return `translate(${size * 0.56}px, ${-size * 0.05}px) rotate(7deg) scale(1.01)`;
    }
    if (phase === "drop") {
      const scale = (target.r * 2) / (size * 0.94);
      const dx = target.x - (destX + size / 2);
      const dy = target.y - (destY + size / 2);
      return `translate(${dx}px, ${dy}px) rotate(0deg) scale(${scale})`;
    }
    return "translate(0, 0) rotate(0deg) scale(1)";
  });

  $effect(() => {
    const frame = requestAnimationFrame(() => {
      lifted = true;
    });
    const draw = setTimeout(() => {
      phase = "draw";
    }, 600);
    const drop = setTimeout(() => {
      phase = "drop";
    }, 1120);
    const done = setTimeout(() => {
      onDone();
    }, 1820);
    const onResize = () => {
      viewW = window.innerWidth;
      viewH = window.innerHeight;
    };
    window.addEventListener("resize", onResize);

    return () => {
      cancelAnimationFrame(frame);
      clearTimeout(draw);
      clearTimeout(drop);
      clearTimeout(done);
      window.removeEventListener("resize", onResize);
    };
  });
</script>

<div class="flight" aria-hidden="true">
  <div
    class="record"
    class:above={phase === "drop"}
    style:left={`${destX + size * 0.03}px`}
    style:top={`${destY + size * 0.03}px`}
    style:width={`${size * 0.94}px`}
    style:height={`${size * 0.94}px`}
    style:transform={discTransform}
    style:--vinyl={playlist ? "#171717" : album.palette.vinyl}
    style:--light={playlist ? "#2c2c2c" : album.palette.vinylLight}
    style:--label={playlist ? "#efe9da" : album.palette.label}
  >
    <div class="record-grooves"></div>
    <div class="record-label">
      {#if playlist}
        <span class="label-hand">{album.title}</span>
        <span class="label-hand small">Seite {album.sides[0]?.label ?? "A"}</span>
        <span class="label-tiny">{album.sides[0]?.rpm === "45" ? "45" : "33 1/3"} RPM · {album.sides[0]?.tracks.length ?? 0} Tracks</span>
        <span class="label-tiny">Promo · Not for Sale</span>
      {:else}
        <span class="label-title">{album.title}</span>
        <span class="label-tiny">{album.albumArtist}</span>
        <span class="label-tiny">Seite {album.sides[0]?.label ?? "A"}</span>
      {/if}
    </div>
    <div class="record-sheen"></div>
  </div>

  <div
    class="sleeve"
    class:lifted
    class:fading={phase === "drop"}
    style:left={`${lifted ? destX : from.left}px`}
    style:top={`${lifted ? destY : from.top}px`}
    style:width={`${lifted ? size : from.width}px`}
    style:height={`${lifted ? size : from.height}px`}
  >
    <div class="sleeve-art">
      {#if playlist}
        <div class="test-art">
          <span class="test-stamp">Test Pressing</span>
          <span class="test-hand">{album.title}</span>
          <span class="test-small">{album.albumArtist}</span>
        </div>
      {:else if cover}
        <img src={cover} alt="" draggable="false" />
      {:else}
        <div class="fallback" style:color={album.palette.label}>
          <span>{album.title}</span>
          <span class="fallback-artist">{album.albumArtist}</span>
        </div>
      {/if}
    </div>
    <span class="paper" aria-hidden="true"></span>
    <span class="wear" aria-hidden="true"></span>
  </div>
</div>

<style>
  .flight {
    position: fixed;
    inset: 0;
    z-index: 40;
    pointer-events: none;
  }

  .sleeve {
    position: fixed;
    z-index: 2;
    border-radius: 2px;
    filter: drop-shadow(0 26px 34px rgba(0, 0, 0, 0.55));
    transition:
      left 560ms cubic-bezier(0.16, 0.84, 0.24, 1),
      top 560ms cubic-bezier(0.16, 0.84, 0.24, 1),
      width 560ms cubic-bezier(0.16, 0.84, 0.24, 1),
      height 560ms cubic-bezier(0.16, 0.84, 0.24, 1),
      transform 560ms cubic-bezier(0.16, 0.84, 0.24, 1),
      opacity 440ms ease;
    transform: rotate(0deg) scale(0.98);
  }

  .sleeve.lifted {
    transform: rotate(-2.4deg) scale(1);
  }

  .sleeve.fading {
    opacity: 0;
    transform: rotate(-6deg) translateY(-2vh) scale(0.99);
    transition-delay: 260ms;
  }

  .sleeve-art {
    position: absolute;
    inset: 0;
    overflow: hidden;
    border-radius: 1px;
    background: #14100c;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.6),
      0 0 0 2px rgba(255, 255, 255, 0.06) inset;
  }

  .sleeve-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .paper {
    position: absolute;
    inset: 1% -1.4% -1.4% 1%;
    z-index: -1;
    border-radius: 1px;
    background-color: #e6dcc2;
    background-image: url("/textures/paper-soft.jpg");
    background-size: 260px 260px;
    box-shadow: 3px 4px 12px rgba(0, 0, 0, 0.35);
  }

  .wear {
    position: absolute;
    inset: 0;
    pointer-events: none;
    border-radius: 1px;
    background:
      radial-gradient(60% 55% at 50% 50%, transparent 58%, rgba(0, 0, 0, 0.24) 63%, transparent 72%),
      radial-gradient(circle at 0 0, rgba(255, 248, 230, 0.5), transparent 11%),
      radial-gradient(circle at 100% 0, rgba(255, 248, 230, 0.34), transparent 10%),
      radial-gradient(circle at 0 100%, rgba(255, 248, 230, 0.3), transparent 9%),
      radial-gradient(circle at 100% 100%, rgba(255, 248, 230, 0.4), transparent 10%);
    mix-blend-mode: overlay;
  }

  .fallback {
    position: absolute;
    inset: 0;
    padding: 10%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 6px;
    background: linear-gradient(160deg, var(--vinyl, #2a2a2a), #0d0d0d 90%);
    font-family: var(--font-display);
    font-size: 22px;
  }

  .fallback-artist {
    font-size: 14px;
    font-style: italic;
    opacity: 0.8;
  }

  .test-art {
    position: absolute;
    inset: 0;
    padding: 9%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: flex-start;
    gap: 1.2%;
    color: #23262e;
    background-color: #efe7d2;
    background-image: url("/textures/paper-soft.jpg");
    background-size: 360px 360px;
    background-blend-mode: multiply;
  }

  .test-stamp {
    align-self: flex-end;
    margin-bottom: 6%;
    padding: 0.5% 1.2%;
    border: 1px solid rgba(150, 38, 38, 0.55);
    color: rgba(150, 38, 38, 0.75);
    font-family: var(--font-display);
    font-size: 11px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    transform: rotate(-10deg);
  }

  .test-hand {
    font-family: var(--font-hand);
    font-size: 34px;
    line-height: 1.02;
    transform: rotate(-1.6deg);
  }

  .test-small {
    font-family: var(--font-hand);
    font-size: 18px;
    opacity: 0.8;
    transform: rotate(-1.1deg);
  }

  .record {
    position: fixed;
    z-index: 1;
    border-radius: 50%;
    background:
      radial-gradient(circle at 36% 28%, rgba(255, 255, 255, 0.16), transparent 34%),
      radial-gradient(
        circle at 50% 50%,
        color-mix(in srgb, var(--light) 55%, var(--vinyl)) 0%,
        var(--vinyl) 42%,
        #101010 72%,
        #222 78%,
        #0a0a0a 82%,
        #191919 100%
      );
    box-shadow:
      0 22px 40px rgba(0, 0, 0, 0.55),
      0 0 0 1px rgba(0, 0, 0, 0.6),
      0 0 0 4px rgba(255, 255, 255, 0.04) inset;
    transition:
      transform 500ms cubic-bezier(0.3, 0.7, 0.2, 1),
      opacity 240ms ease;
  }

  .record:not(.above) {
    transition:
      transform 460ms cubic-bezier(0.16, 0.84, 0.24, 1),
      opacity 240ms ease;
  }

  .record.above {
    z-index: 4;
    transition: transform 660ms cubic-bezier(0.24, 0.72, 0.16, 1);
  }

  .record-grooves {
    position: absolute;
    inset: 4%;
    border-radius: 50%;
    background:
      repeating-radial-gradient(
        circle at 50% 50%,
        rgba(255, 255, 255, 0.05) 0 0.7px,
        rgba(0, 0, 0, 0.12) 0.7px 2.6px,
        transparent 2.6px 4px
      ),
      radial-gradient(circle at 38% 30%, rgba(255, 255, 255, 0.09), transparent 46%);
  }

  .record-label {
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
    justify-content: center;
    gap: 3%;
    padding: 6%;
    text-align: center;
    color: #20232b;
    background:
      radial-gradient(circle at 38% 30%, rgba(255, 255, 255, 0.9), transparent 58%),
      radial-gradient(circle at 50% 50%, var(--label), color-mix(in srgb, var(--label) 80%, #b9a97f) 100%);
    box-shadow:
      0 2px 8px rgba(0, 0, 0, 0.45),
      0 0 0 1px rgba(0, 0, 0, 0.3) inset;
    overflow: hidden;
  }

  .label-title {
    font-family: var(--font-display);
    font-size: 8px;
    line-height: 1.1;
    max-height: 2.2em;
    overflow: hidden;
  }

  .label-hand {
    font-family: var(--font-hand);
    font-size: 13px;
    line-height: 1.04;
    transform: rotate(-2deg);
  }

  .label-hand.small {
    font-size: 10px;
    opacity: 0.8;
  }

  .label-tiny {
    font-size: 6px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    opacity: 0.62;
  }

  .record-sheen {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background:
      radial-gradient(46% 22% at 28% 20%, rgba(255, 255, 255, 0.14), transparent 70%),
      radial-gradient(40% 18% at 74% 82%, rgba(255, 255, 255, 0.05), transparent 70%);
    mix-blend-mode: screen;
  }

  @media (prefers-reduced-motion: reduce) {
    .sleeve,
    .record {
      transition: none;
    }
  }
</style>
