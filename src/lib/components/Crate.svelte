<script lang="ts">
  const rows = [0, 1, 2];
  const slots = [0, 1, 2];
</script>

<section class="crate" aria-label="Plattenkiste">
  <header class="head" data-tauri-drag-region>
    <button class="icon" type="button" aria-label="Bibliothek durchsuchen" title="Suchen">
      <svg viewBox="0 0 24 24" width="17" height="17" aria-hidden="true">
        <circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" stroke-width="1.8" />
        <path d="M15.5 15.5 21 21" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
      </svg>
    </button>

    <button class="plaque" type="button" title="Sortierung">
      <span>· ZULETZT HINZUGEFÜGT ·</span>
    </button>

    <div class="pager" aria-label="Seite 1 von 1">
      <span class="dot active"></span><span class="dot"></span><span class="dot"></span
      ><span class="dot"></span><span class="dot"></span>
    </div>
  </header>

  <div class="shelves">
    {#each rows as row}
      <div class="shelf">
        <div class="slots">
          {#each slots as slot}
            {#if row === 0 && slot === 0}
              <button class="slot empty add" type="button" title="Musikordner hinzufügen">
                <span class="plus">+</span>
                <span class="label">Ordner<br />hinzufügen</span>
              </button>
            {:else}
              <div class="slot empty" aria-hidden="true"></div>
            {/if}
          {/each}
        </div>
        <div class="board"></div>
      </div>
    {/each}
  </div>

  <p class="hint">Noch keine Platten — füge einen Ordner mit deiner Musik hinzu.</p>
</section>

<style>
  .crate {
    position: relative;
    height: 100%;
    min-width: 0;
    padding: 0 26px 18px;
    display: flex;
    flex-direction: column;
    background:
      linear-gradient(90deg, rgba(0, 0, 0, 0.42), transparent 9%, transparent 91%, rgba(0, 0, 0, 0.38)),
      linear-gradient(180deg, rgba(0, 0, 0, 0.34), transparent 10%, transparent 90%, rgba(0, 0, 0, 0.42));
  }

  .head {
    display: grid;
    grid-template-columns: 44px 1fr 84px;
    align-items: center;
    gap: 10px;
    height: 62px;
    flex: 0 0 auto;
  }

  .icon {
    justify-self: start;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    color: #2a1a0e;
    background: radial-gradient(circle at 35% 28%, #f0d88f, #c9a227 55%, #7d5f14 100%);
    box-shadow:
      0 1px 0 rgba(255, 240, 190, 0.55) inset,
      0 2px 5px rgba(0, 0, 0, 0.55);
  }

  .plaque {
    position: relative;
    justify-self: center;
    padding: 5px 18px 4px;
    border-radius: 3px;
    color: #33250b;
    font-family: var(--font-display);
    font-size: 10.5px;
    letter-spacing: 0.16em;
    background:
      linear-gradient(180deg, rgba(255, 247, 210, 0.55), transparent 42%),
      linear-gradient(180deg, #d9b64a, #b8912a 55%, #8f6d17);
    box-shadow:
      0 1px 0 rgba(255, 250, 220, 0.7) inset,
      0 -1px 0 rgba(60, 40, 0, 0.55) inset,
      0 2px 6px rgba(0, 0, 0, 0.6);
    text-shadow: 0 1px 0 rgba(255, 245, 200, 0.45);
  }

  .pager {
    justify-self: end;
    display: flex;
    gap: 7px;
  }

  .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: #6f4a24;
    box-shadow: 0 1px 0 rgba(255, 220, 160, 0.18);
  }

  .dot.active {
    background: var(--brass-light);
    box-shadow: 0 0 6px rgba(232, 207, 122, 0.6);
  }

  .shelves {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    justify-content: space-evenly;
    min-height: 0;
  }

  .shelf {
    position: relative;
    padding: 0 6px 12px;
  }

  .slots {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 18px;
    padding: 6px 6px 0;
  }

  .slot {
    position: relative;
    aspect-ratio: 1 / 1.02;
    border-radius: 2px;
    background:
      linear-gradient(180deg, rgba(0, 0, 0, 0.34), rgba(255, 180, 110, 0.03)),
      repeating-linear-gradient(90deg, rgba(0, 0, 0, 0.18) 0 2px, transparent 2px 7px),
      #241407;
    box-shadow:
      0 1px 0 rgba(255, 205, 150, 0.08),
      0 0 0 1px rgba(0, 0, 0, 0.45) inset,
      0 8px 18px rgba(0, 0, 0, 0.35) inset;
  }

  .slot.empty::after {
    content: "";
    position: absolute;
    inset: 12%;
    border: 1px dashed rgba(239, 230, 207, 0.12);
    border-radius: 2px;
  }

  .slot.add {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: rgba(239, 230, 207, 0.55);
    border: 1px dashed rgba(239, 230, 207, 0.22);
    background:
      linear-gradient(180deg, rgba(255, 200, 140, 0.05), rgba(0, 0, 0, 0.25)),
      rgba(36, 20, 7, 0.6);
    transition: color 140ms ease, border-color 140ms ease, background 140ms ease;
  }

  .slot.add:hover {
    color: var(--brass-light);
    border-color: rgba(232, 207, 122, 0.5);
  }

  .slot.add::after {
    display: none;
  }

  .plus {
    font-family: var(--font-display);
    font-size: 26px;
    line-height: 1;
  }

  .label {
    font-size: 9px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    text-align: center;
    line-height: 1.5;
  }

  .board {
    height: 12px;
    margin: 0 -6px;
    border-radius: 1px;
    background:
      linear-gradient(180deg, rgba(255, 214, 160, 0.28), transparent 30%),
      linear-gradient(180deg, #7b4a29, #4a2a17 55%, #2a1710 100%);
    box-shadow:
      0 1px 0 rgba(255, 225, 175, 0.25) inset,
      0 6px 10px rgba(0, 0, 0, 0.45);
  }

  .hint {
    flex: 0 0 auto;
    margin: 4px 0 0;
    text-align: center;
    font-family: var(--font-display);
    font-size: 12px;
    font-style: italic;
    color: rgba(239, 230, 207, 0.42);
  }
</style>
