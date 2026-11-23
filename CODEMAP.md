Stand: 3f7a296

# Longplay — Codemap

## 1. Zweck + Stack
Skeuomorpher Plattenspieler für lokale Musikdateien: Alben als Platten in einer Holzkiste, Tonarm von Hand aufsetzen, 33/45, A/B-Seiten (README.md:3-9).
Tauri 2 + SvelteKit mit `adapter-static`/SPA (`ssr=false`) als dünne UI über einem Rust-Kern (svelte.config.js:12-14, src/routes/+layout.ts:5, README.md:33-35).
Rust-Crates: lofty, rusqlite, symphonia, cpal (nur non-Linux), image, blake3, walkdir (src-tauri/Cargo.toml:24-32,43-44); Edition 2024 (:6).

## 2. Befehle
- `npm run tauri dev` startet Vite (via `beforeDevCommand`) + Tauri-App (README.md:44, src-tauri/tauri.conf.json:7-8).
- `npm run dev` allein: Vite auf Port 1430, `strictPort`, Host 127.0.0.1 oder `TAURI_DEV_HOST`; HMR 1431 nur mit letzterem (package.json:7, vite.config.js:5,19-27).
- `npm run build` = Vite/SvelteKit-Build nach `build/`; komplettes App-Bundle via `npm run tauri build` (package.json:8, README.md:73,80, tauri.conf.json:9-10).
- `npm run check` = `svelte-kit sync` + `svelte-check` (package.json:11); `npm run preview` (package.json:9).
- Rust-Tests: `cargo test --manifest-path src-tauri/Cargo.toml` (README.md:74); kein Watch-/Frontend-Testskript (package.json:6-13).

## 3. Verzeichnis-/Modulkarte
- `src/routes/+page.svelte` (224): Gesamtlayout, Albumauswahl, clientseitige Suche/Sortierung, 9 Alben/Seite (`:51`), 250-ms-Polling (`:75-102`), Handler (`:115-135`).
- `src/routes/+layout.ts` (5): `ssr=false`.
- `src/lib/types.ts` (90): IPC-DTOs, 1:1 zu Serde-CamelCase.
- `src/lib/library.svelte.ts` (79): Library-Store, Ordnerdialog/Scan (`:18-68`), `coverUrl` via `convertFileSrc` (`:70-72`).
- `src/lib/playback.svelte.ts` (77): invoke-Wrapper für alle Playback-Commands, `formatClock` (`:73`).
- `src/lib/groove.ts` (48): Frontend-Tonarmwinkel und Radius→Cue-Zeit (`:4-13`, `:22-47`).
- `src/lib/theme.css` (61): Farb-/Font-Variablen (`:2-19`).
- `src/lib/components/Crate.svelte` (450): Kiste, Suche, Sortierschalter, Pager (`:89-117`), 3×3-Raster (`:47-48`, `:140`).
- `src/lib/components/Sleeve.svelte` (161): Coverkarte (`:7`), CSS-Farben aus Palette (`:15-17`), Fallback (`:25-28`).
- `src/lib/components/Deck.svelte` (892): Plattenspieler, Tonarm-Ziehen/Cue (`:196-258`), rAF-Animation von Teller und Arm (`:104-174`).
- `src/lib/components/Vinyl.svelte` (353): Platte, Canvas-Rillen aus `GrooveBand` (`:45-100`).
- `src/lib/components/WoodBackground.svelte` (62): statischer Holz-Hintergrund (`:2-3`).
- `src-tauri/src/lib.rs` (197): `AppState` (`:21-25`), 12 Tauri-Commands (`:28-142`, Registrierung `:181-194`), Setup/DB/Audio (`:145-196`).
- `src-tauri/src/audio/session.rs` (648): Orchestrierung: Laden (`:169-221`), Prefetch (`:142-167`), Conductor (`:114-120`), PlaybackState (`:353-431`).
- `src-tauri/src/audio/engine.rs` (700): Echtzeit-DSP, Position, Auto-Advance (`:237-249`), Limiter (`:421`).
- `src-tauri/src/audio/decode.rs` (340): symphonia-Decode eines Tracks inkl. Pregap (`:131`).
- `src-tauri/src/audio/resample.rs` (150): Catmull-Rom (`:7`); `noise.rs` (130): Knacken/Rumpeln (`:44`); `output.rs` (108): cpal (`:14`) bzw. Null-Sink (`:96`).
- `src-tauri/src/audio/timeline.rs` (215): Seite → Track-Spans (`:34`); `groove.rs` (335): mm-Rillenlayout (`:92`), Zeit↔Radius (`:74`).
- `src-tauri/src/library/scanner.rs` (818): rekursiver Scan (`:70`, `:123`), Seiten-/Format-Heuristik (`:511`, `:425`).
- `src-tauri/src/library/cue.rs` (845): CUE-Parser, Bytes/Text (`:85`, `:103`).
- `src-tauri/src/library/db.rs` (256): SQLite (`:45`, `:74`, `:104`); `model.rs` (148): Album/Side/Track/Palette (`:7`); `metadata.rs` (90): lofty (`:35`); `artwork.rs` (87): Cover (`:47`); `palette.rs` (207): Farben (`:12`).
- `src-tauri/src/playback/geometry.rs` (206): Tonarmmaße und Winkel (`:36-52`); `platform/mod.rs` (1): nur Stub/Doc, keine Implementierung (ganze Datei).

## 4. Datenfluss/Architektur
- UI ruft ausschließlich `invoke`: `library_load`/`library_folders` (src-tauri/src/lib.rs:33-49), `library_scan` (`:51-73`), Playback-Commands (`:75-142`).
- Scan: Command → `spawn_blocking` → `scanner::scan` (lib.rs:59-72, scanner.rs:70); zuerst CUE-Alben, deren Dateien werden aus der Tag-Gruppierung entfernt (scanner.rs:81-90), danach Datei-Gruppen (`:92-108`), zum Schluss `db.replace_albums` (`:113`).
- Abspielen: `playback_load` holt Album aus DB und ruft `Session::load` (lib.rs:75-101, session.rs:169); ein Thread dekodiert Track 0 (session.rs:205-219); der nächste Track wird bei Bedarf vorgeladen (session.rs:142-167).
- Audio-Callback: `Engine::process` mischt Track, Varispeed, Noise, Limiter (engine.rs:299-420); am Track-Ende `advance()` auf den Prefetch-Puffer mit 20-ms-Fade (engine.rs:237-249,393); Conductor-Thread tickt alle 120 ms (session.rs:114-120).
- Kein Event-Kanal: UI pollt `playback_state` alle 250 ms und spiegelt den Snapshot in den Store (+page.svelte:77-84, playback.svelte.ts:28-35, session.rs:353-431).
- Persistenz: SQLite `app_data_dir/library.sqlite3` (lib.rs:156); `albums` speichert `palette`/`format`/`sides` als JSON-Text plus `folders` (db.rs:49-68). Cover als 512-px-JPEG in `app_cache_dir/covers` (lib.rs:150-153, artwork.rs:11,47-64).
- Cover-Anzeige braucht Asset-Protokoll `$APPCACHE/**` (tauri.conf.json:29-33) und `convertFileSrc` (library.svelte.ts:70-72).
- Tonarm: Backend liefert `stylusRadius`/`armAngle`/`grooves` (session.rs:390-421); Deck animiert auf den Zielwinkel oder auf die manuell gezogene Position (Deck.svelte:104-111,157-174) und mappt Zeigerradius auf Zeit (Deck.svelte:196-206,241-258).
- Tellerrotation ist rein visuell: 1,8 s bzw. 1,333 s pro Umdrehung (Deck.svelte:118); die echte Position kommt aus dem Backend.

## 5. Wichtige Typen, Konstanten, Einstiegspunkte
- `Album`/`Side`/`Track`/`Palette` camelCase (model.rs:7-111); `DiscFormat` lowercase `lp12|single7|unknown` (model.rs:34-39); `Rpm` als String `"33"|"45"` (model.rs:50-55).
- `PlaybackState` (session.rs:24-45) entspricht `src/lib/types.ts:69-90`; `GrooveBand` (groove.rs:23-39) entspricht `types.ts:58-67`.
- `ENGINE_RATE = 48000` (session.rs:20); Track-Fade 20 ms (engine.rs:18); Cover 512 px (artwork.rs:11); Tonarm 211/230 mm (geometry.rs:36-52); 12": 146→60 mm, 7": 84→45 mm (groove.rs:54-70).
- Audio-Endungen `flac mp3 m4a aac wav aiff aif ogg opus alac` (scanner.rs:17-19).
- Einstieg: `src-tauri/src/main.rs:4-6` → `longplay_lib::run` (lib.rs:145); Frontend `src/routes/+page.svelte`; App-HTML `src/app.html:1-14`.

## 6. Konfiguration/Dateipfade/Umgebungsvariablen
- Tauri: devUrl `http://localhost:1430`, vor Dev/Build `npm run dev|build`, Frontend `../build` (tauri.conf.json:7-10); Fenster 1280×900, min. 1100×760, Overlay-Titlebar (tauri.conf.json:15-22).
- Rechte: `core:default`, `dialog:default` für Fenster `main` (capabilities/default.json:5-11).
- Vite ignoriert `src-tauri/**` im Watcher (vite.config.js:31); `TAURI_DEV_HOST` steuert Host/HMR (vite.config.js:5,21-27).
- `LONGPLAY_SCAN_DIR` (Doppelpunkt-getrennt, `split_paths`) scannt beim Start ohne Ordnerdialog (lib.rs:160-169; README.md:54-57).
- Pfade: DB `app_data_dir/library.sqlite3`, Cover `app_cache_dir/covers` (lib.rs:149-156).
- SPA-Fallback `index.html` (svelte.config.js:12-14); Build-Artefakte `build/` und `.svelte-kit/` ignoriert (.gitignore:3-4).

## 7. Gotchas
- Scan ist **nicht inkrementell**: jede Datei wird erneut getaggt und `replace_albums` löscht alle Alben (scanner.rs:90-113, db.rs:74-77); PLAN.md:350 nennt das selbst offen, obwohl PLAN.md:32,149 es als Feature listet.
- Kommentar-Drift: `audio/mod.rs:3-5` behauptet "ganze Seite in einem Puffer"; tatsächlich wird pro Track dekodiert (session.rs:208, decode.rs:131).
- Album anklicken lädt Seite mit `autoplay=false` und pausiert/leert die laufende Wiedergabe (page.svelte:115-119, session.rs:176-181); Seitenwechsel behält dagegen `playing` (page.svelte:121-126).
- Tonarmgeometrie existiert doppelt: Defaults 211/230 in `groove.ts:6-7` und `geometry.rs:36-52`. Nur eine Seite ändern bricht Cue/Anzeige.
- Format-Heuristik: ≤2 Tracks und ≤600 s gelten automatisch als 7″/45 rpm (scanner.rs:425-434); das ändert Plattengröße, RPM und Geometrie.
- Seiten ohne Hinweise werden an der zeitlich besten Trennung in A/B geteilt; Matrix ist synthetisch `LP-…` (scanner.rs:511-555,451).
- Linux nutzt keinen cpal-Stream, sondern einen getakteten Null-Sink (output.rs:13,95-96); cpal ist Target-spezifische Dependency (Cargo.toml:43-44).
- Keine Tauri-Events: nach jedem `invoke` muss die Antwort selbst in den Store; Polling ist die einzige Aktualisierung (playback.svelte.ts:11-71).
- Fehler werden teils geschluckt: `refreshPlayback` ignoriert Exceptions, Audio-Startfehler landen nur in `error`/stderr (playback.svelte.ts:32-34, session.rs:98-112).
- Cue auf fernen Track: UI hält die Nadel 900 ms per `cueHold` (Deck.svelte:141-147); prefetcht wird nur der direkt nächste Track (session.rs:142-167).
- Suche/Sortierung laufen rein im Frontend über Titel/Interpret (page.svelte:22-47); kein FTS, obwohl PLAN.md:85 es vorsieht.
- Album-IDs sind gekürzte blake3-Hashes aus Ordner|Titel|Interpret (scanner.rs:435-438,591-594); Pfadumbenennung erzeugt neue IDs.

## 8. Tests
- Nur Rust: 93 `#[test]` in `src-tauri/src`; Schwerpunkte cue (22), engine (17), scanner (11), groove (9), geometry (7), session (5).
- Abdeckung: CUE-Parsing/Encoding/Pregaps/Seitenhinweise, Tag-Scan und Seiten-Split, DB-Roundtrip, Palette/HSL, Resampler, Engine (Seek, Varispeed, Fades, Limiter, Auto-Advance), Session-Prefetch, Tonarm-/Rillengeometrie.
- Ausführen: `cargo test --manifest-path src-tauri/Cargo.toml` (README.md:74); Session-Tests teilweise Linux-only (session.rs:439,535).
- Scanner-Tests schreiben echte WAVs in Temp-Verzeichnisse (scanner.rs:700-733); Engine-Tests rendern offline Puffer (engine.rs:460).
- Frontend: keine Tests/kein Runner (package.json:6-13); Absicherung nur `npm run check` und `npm run build`.
