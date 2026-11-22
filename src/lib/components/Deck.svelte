<section class="deck" aria-label="Plattenspieler">
  <header class="brand" data-tauri-drag-region>
    <h1>LONGPLAY</h1>
    <p>DIRECT DRIVE · MODEL 01</p>
  </header>

  <div class="stage">
    <div class="platter-wrap">
      <div class="platter">
        <svg class="strobe" viewBox="0 0 400 400" aria-hidden="true">
          <circle
            cx="200"
            cy="200"
            r="188"
            fill="none"
            stroke="rgba(239,230,207,0.30)"
            stroke-width="4"
            stroke-dasharray="2.4 11"
          />
        </svg>
        <div class="mat"></div>
        <div class="spindle"></div>
      </div>
      <p class="deck-hint">Platte aus der Kiste auflegen</p>
    </div>

    <svg class="arm" viewBox="0 0 300 400" aria-hidden="true">
      <defs>
        <linearGradient id="chrome" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stop-color="#6d6d6d" />
          <stop offset="22%" stop-color="#e9e9e9" />
          <stop offset="52%" stop-color="#8f8f8f" />
          <stop offset="78%" stop-color="#d8d8d8" />
          <stop offset="100%" stop-color="#5a5a5a" />
        </linearGradient>
        <linearGradient id="weight" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stop-color="#4b4b4b" />
          <stop offset="30%" stop-color="#cfcfcf" />
          <stop offset="70%" stop-color="#7c7c7c" />
          <stop offset="100%" stop-color="#3d3d3d" />
        </linearGradient>
        <radialGradient id="base" cx="38%" cy="30%" r="70%">
          <stop offset="0%" stop-color="#9a9a9a" />
          <stop offset="55%" stop-color="#4e4e4e" />
          <stop offset="100%" stop-color="#232323" />
        </radialGradient>
        <filter id="armShadow" x="-30%" y="-20%" width="170%" height="150%">
          <feDropShadow dx="4" dy="7" stdDeviation="6" flood-color="#000" flood-opacity="0.55" />
        </filter>
      </defs>

      <g filter="url(#armShadow)">
        <!-- Gegengewicht -->
        <rect x="150" y="18" width="34" height="58" rx="14" fill="url(#weight)" />
        <rect x="150" y="18" width="34" height="58" rx="14" fill="none" stroke="rgba(0,0,0,0.5)" />

        <!-- Armrohr -->
        <path d="M216 72 L248 338" stroke="#181818" stroke-width="11" stroke-linecap="round" />
        <path d="M216 72 L248 338" stroke="url(#chrome)" stroke-width="7" stroke-linecap="round" />

        <!-- Pivot -->
        <circle cx="216" cy="72" r="28" fill="url(#base)" />
        <circle cx="216" cy="72" r="28" fill="none" stroke="rgba(255,255,255,0.18)" />
        <circle cx="216" cy="72" r="9" fill="#1c1c1c" />
        <circle cx="213" cy="68" r="3" fill="rgba(255,255,255,0.55)" />

        <!-- Headshell + System -->
        <g transform="rotate(6 248 338)">
          <rect x="233" y="328" width="30" height="44" rx="3" fill="#2a2a2a" />
          <rect x="236" y="331" width="24" height="30" rx="2" fill="#c0392b" />
          <rect x="240" y="361" width="16" height="10" rx="1" fill="#8f8f8f" />
        </g>

        <!-- Armablage -->
        <path d="M268 292 l20 -14 l10 14 l-20 14 z" fill="#2b2b2b" stroke="rgba(255,255,255,0.12)" />
      </g>
    </svg>
  </div>

  <div class="controls">
    <button class="start" type="button" aria-disabled="true" title="Noch keine Platte aufgelegt">
      <span>START</span>
      <span class="sub">STOP</span>
    </button>

    <div class="rpm" role="group" aria-label="Drehzahl">
      <button class="speed active" type="button" aria-disabled="true">33</button>
      <button class="speed" type="button" aria-disabled="true">45</button>
    </div>

    <button class="brush" type="button" aria-disabled="true" title="Anti-Static-Bürste">
      ANTI-STATIC
    </button>
  </div>
</section>

<style>
  .deck {
    position: relative;
    height: 100%;
    min-width: 0;
    padding: 18px 30px 26px;
    display: flex;
    flex-direction: column;
    background:
      linear-gradient(180deg, rgba(255, 210, 160, 0.05), transparent 18%),
      radial-gradient(80% 60% at 70% 30%, rgba(255, 200, 140, 0.05), transparent 70%),
      linear-gradient(90deg, rgba(0, 0, 0, 0.5), transparent 6%);
    border-left: 1px solid rgba(0, 0, 0, 0.55);
    box-shadow: -14px 0 30px rgba(0, 0, 0, 0.28) inset;
  }

  .brand {
    flex: 0 0 auto;
    padding: 6px 0 0 8px;
  }

  .brand h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 21px;
    font-weight: 400;
    letter-spacing: 0.28em;
    color: rgba(239, 230, 207, 0.62);
    text-shadow:
      0 -1px 0 rgba(0, 0, 0, 0.65),
      0 1px 0 rgba(255, 220, 170, 0.10);
  }

  .brand p {
    margin: 4px 0 0;
    font-size: 9px;
    letter-spacing: 0.24em;
    color: rgba(239, 230, 207, 0.3);
  }

  .stage {
    position: relative;
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    align-items: center;
  }

  .platter-wrap {
    width: min(78%, 540px);
    margin-left: 2%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
  }

  .platter {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    border-radius: 50%;
    background:
      radial-gradient(circle at 42% 34%, rgba(255, 255, 255, 0.11), transparent 42%),
      radial-gradient(circle at 50% 50%, #2c2c2c 0 62%, #1a1a1a 63% 100%);
    box-shadow:
      0 24px 48px rgba(0, 0, 0, 0.6),
      0 2px 0 rgba(255, 255, 255, 0.08) inset,
      0 -2px 0 rgba(0, 0, 0, 0.6) inset;
  }

  .strobe {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .mat {
    position: absolute;
    inset: 4.5%;
    border-radius: 50%;
    background:
      radial-gradient(circle at 40% 30%, rgba(255, 255, 255, 0.045), transparent 55%),
      repeating-radial-gradient(
        circle at 50% 50%,
        rgba(255, 255, 255, 0.018) 0 1px,
        transparent 1px 4px
      ),
      #1e1e1e;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.6) inset,
      0 0 40px rgba(0, 0, 0, 0.55) inset;
  }

  .spindle {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 3.2%;
    aspect-ratio: 1;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    background: radial-gradient(circle at 35% 30%, #f2f2f2, #8a8a8a 55%, #3a3a3a);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.7),
      0 0 0 1px rgba(0, 0, 0, 0.5);
  }

  .deck-hint {
    margin: 0;
    font-family: var(--font-display);
    font-size: 12px;
    font-style: italic;
    color: rgba(239, 230, 207, 0.36);
  }

  .arm {
    position: absolute;
    right: -2%;
    top: 2%;
    height: 96%;
    width: auto;
    overflow: visible;
    pointer-events: none;
  }

  .controls {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 22px;
    padding: 0 8px 0 2%;
  }

  .start {
    width: 62px;
    height: 62px;
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1px;
    color: #241708;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.12em;
    background:
      radial-gradient(circle at 36% 28%, #f6f0e2, #cdc6b6 48%, #8e8878 100%);
    box-shadow:
      0 1px 0 rgba(255, 255, 255, 0.75) inset,
      0 -2px 6px rgba(0, 0, 0, 0.35) inset,
      0 6px 12px rgba(0, 0, 0, 0.55);
    opacity: 0.75;
  }

  .start .sub {
    font-size: 7px;
    opacity: 0.65;
  }

  .rpm {
    display: flex;
    gap: 9px;
  }

  .speed {
    width: 30px;
    height: 30px;
    border-radius: 5px;
    font-size: 10px;
    letter-spacing: 0.06em;
    color: rgba(239, 230, 207, 0.42);
    background: linear-gradient(180deg, #2a1a0e, #17100a);
    box-shadow:
      0 1px 0 rgba(255, 220, 170, 0.08) inset,
      0 2px 4px rgba(0, 0, 0, 0.5);
    opacity: 0.85;
  }

  .speed.active {
    color: var(--brass-light);
    box-shadow:
      0 0 0 1px rgba(201, 162, 39, 0.45) inset,
      0 0 10px rgba(201, 162, 39, 0.16);
    opacity: 1;
  }

  .brush {
    margin-left: auto;
    margin-right: 4%;
    padding: 9px 26px;
    border-radius: 5px;
    font-size: 9px;
    letter-spacing: 0.18em;
    color: rgba(239, 230, 207, 0.5);
    background:
      linear-gradient(180deg, #5a3720, #3a2112 60%, #2a1710);
    box-shadow:
      0 1px 0 rgba(255, 220, 170, 0.14) inset,
      0 3px 8px rgba(0, 0, 0, 0.45);
    opacity: 0.8;
  }
</style>
