<script lang="ts">
  import { tick } from "svelte";
  import Sleeve from "./Sleeve.svelte";
  import type { Album, SortKey } from "$lib/types";

  interface Props {
    albums: Album[];
    total: number;
    page: number;
    pageCount: number;
    sort: SortKey;
    query: string;
    scanning: boolean;
    onAdd: () => void;
    onRescan: () => void;
    onSelect: (album: Album) => void;
    onPage: (page: number) => void;
    onSort: () => void;
    onQuery: (value: string) => void;
  }

  let {
    albums,
    total,
    page,
    pageCount,
    sort,
    query,
    scanning,
    onAdd,
    onRescan,
    onSelect,
    onPage,
    onSort,
    onQuery,
  }: Props = $props();

  const sortLabels: Record<SortKey, string> = {
    added: "· ZULETZT HINZUGEFÜGT ·",
    artist: "· KÜNSTLER ·",
    album: "· ALBUM A–Z ·",
    year: "· JAHR ·",
  };

  let searchOpen = $state(false);
  let input: HTMLInputElement | undefined = $state();

  const cells = $derived.by(() => {
    const out: (Album | null)[] = albums.slice(0, 9);
    while (out.length < 9) out.push(null);
    return out;
  });

  async function toggleSearch() {
    searchOpen = !searchOpen;
    if (searchOpen) {
      await tick();
      input?.focus();
    } else {
      onQuery("");
    }
  }
</script>

<section class="crate" aria-label="Plattenkiste">
  <div class="spotlight" aria-hidden="true"></div>
  <header class="head" data-tauri-drag-region>
    <button
      class="icon"
      class:active={searchOpen}
      type="button"
      aria-label="Bibliothek durchsuchen"
      onclick={toggleSearch}
    >
      <svg viewBox="0 0 24 24" width="17" height="17" aria-hidden="true">
        <circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" stroke-width="1.8" />
        <path
          d="M15.5 15.5 21 21"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
        />
      </svg>
    </button>

    <button class="plaque" type="button" onclick={onSort} title="Sortierung ändern">
      <span>{sortLabels[sort]}</span>
    </button>

    <div class="nav">
      {#if pageCount > 1}
        <button
          class="arrow"
          type="button"
          aria-label="Vorherige Seite"
          disabled={page === 0}
          onclick={() => onPage(page - 1)}>‹</button
        >
        {#if pageCount <= 9}
          <div class="pager">
            {#each Array(pageCount) as _, index}
              <button
                class="dot"
                class:active={index === page}
                type="button"
                aria-label={`Seite ${index + 1}`}
                onclick={() => onPage(index)}
              ></button>
            {/each}
          </div>
        {:else}
          <span class="count">{page + 1} / {pageCount}</span>
        {/if}
        <button
          class="arrow"
          type="button"
          aria-label="Nächste Seite"
          disabled={page >= pageCount - 1}
          onclick={() => onPage(page + 1)}>›</button
        >
      {/if}
    </div>
  </header>

  {#if searchOpen}
    <div class="search">
      <input
        bind:this={input}
        value={query}
        oninput={(event) => onQuery(event.currentTarget.value)}
        placeholder="Alben oder Künstler suchen"
        spellcheck="false"
      />
      <button type="button" aria-label="Suche schließen" onclick={toggleSearch}>×</button>
    </div>
  {/if}

  <div class="shelves">
    {#each [0, 1, 2] as row}
      <div class="shelf">
        <div class="slots">
          {#each cells.slice(row * 3, row * 3 + 3) as album}
            {#if album}
              <Sleeve {album} onselect={() => onSelect(album)} />
            {:else if total === 0 && row === 0}
              <button class="slot add" type="button" onclick={onAdd} title="Musikordner hinzufügen">
                <span class="plus">+</span>
                <span class="label">
                  {#if scanning}
                    Scanne …
                  {:else}
                    Ordner<br />hinzufügen
                  {/if}
                </span>
              </button>
            {:else}
              <div class="slot" aria-hidden="true"></div>
            {/if}
          {/each}
        </div>
        <div class="board"></div>
      </div>
    {/each}
  </div>

  <div class="foot">
    {#if scanning}
      <span class="status"><span class="spinner"></span> Bibliothek wird gescannt …</span>
    {:else if total === 0}
      <span class="hint">Noch keine Platten — füge einen Ordner mit deiner Musik hinzu.</span>
    {:else}
      <span class="hint">{total} {total === 1 ? "Album" : "Alben"}</span>
      <button class="text-button" type="button" disabled={scanning} onclick={onRescan}>
        Aktualisieren
      </button>
      <button class="text-button" type="button" onclick={onAdd}>＋ Ordner hinzufügen</button>
    {/if}
  </div>
</section>

<style>
  .crate {
    position: relative;
    height: 100%;
    min-width: 0;
    padding: 0 26px 14px;
    display: flex;
    flex-direction: column;
    background:
      linear-gradient(90deg, rgba(0, 0, 0, 0.42), transparent 9%, transparent 91%, rgba(0, 0, 0, 0.38)),
      repeating-linear-gradient(90deg, rgba(0, 0, 0, 0.2) 0 1px, transparent 1px 154px),
      linear-gradient(180deg, rgba(0, 0, 0, 0.28), transparent 8%, transparent 92%, rgba(0, 0, 0, 0.34));
  }

  .spotlight {
    position: absolute;
    top: 0;
    left: 10%;
    right: 10%;
    height: 46%;
    pointer-events: none;
    background: conic-gradient(
      from 180deg at 50% 0%,
      transparent 36%,
      rgba(255, 222, 175, 0.11) 50%,
      transparent 64%
    );
    filter: blur(18px);
    opacity: 0.8;
  }

  .head {
    display: grid;
    grid-template-columns: 84px 1fr 96px;
    align-items: center;
    gap: 10px;
    height: 60px;
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

  .icon.active {
    box-shadow:
      0 0 0 1px rgba(255, 246, 210, 0.5) inset,
      0 0 12px rgba(232, 207, 122, 0.5);
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

  .nav {
    justify-self: end;
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .arrow {
    width: 18px;
    height: 18px;
    color: rgba(239, 230, 207, 0.55);
    font-size: 15px;
    line-height: 1;
  }

  .arrow:disabled {
    opacity: 0.2;
  }

  .pager {
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

  .count {
    font-size: 9px;
    letter-spacing: 0.12em;
    color: rgba(239, 230, 207, 0.55);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: -6px 0 10px;
    padding: 0 10px;
    height: 32px;
    border-radius: 4px;
    background: rgba(20, 11, 5, 0.65);
    box-shadow:
      0 0 0 1px rgba(201, 162, 39, 0.22) inset,
      0 4px 10px rgba(0, 0, 0, 0.3) inset;
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--cream);
    font: inherit;
    font-size: 12px;
    letter-spacing: 0.04em;
    outline: none;
  }

  .search input::placeholder {
    color: rgba(239, 230, 207, 0.35);
    font-style: italic;
  }

  .search button {
    color: rgba(239, 230, 207, 0.5);
    font-size: 18px;
    line-height: 1;
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
    aspect-ratio: 1;
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

  .slot::after {
    content: "";
    position: absolute;
    inset: 12%;
    border: 1px dashed rgba(239, 230, 207, 0.1);
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

  .foot {
    flex: 0 0 auto;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 14px;
  }

  .hint,
  .status {
    font-family: var(--font-display);
    font-size: 12px;
    font-style: italic;
    color: rgba(239, 230, 207, 0.42);
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .spinner {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 2px solid rgba(239, 230, 207, 0.18);
    border-top-color: var(--brass-light);
    animation: spin 900ms linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .text-button {
    font-size: 9px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: rgba(232, 207, 122, 0.7);
  }

  .text-button:hover {
    color: var(--brass-light);
  }
</style>
