# Switchloom — Taskliste

Stand: 17. September 2026. Änderungen sind lokal umgesetzt und noch nicht
committed oder veröffentlicht. Haken bedeuten im genannten Umfang geprüft.

**Aktuell:** Version 1.0.0 ist lokal vorbereitet, noch nicht releasefähig.
Die Hauptansicht ist das Capability-Board: Luna max zuerst, Sol medium, Astra
high. Alle Modelle sind austauschbar und deaktivierbar. Jev ist standardmäßig
an; Vanilla benötigt weder CLI noch API-Key. Der Prompt verwendet nur aktive
Zuweisungen. Der echte Desktop-Ablauf und Benchmarks bleiben offen.

## Capability-Board

- [x] B01: Vorschau durch die Hauptansicht ersetzen; kein paralleler Generator.
- [x] B02: Capabilities frei verschieben; Browser use separat aufnehmen.
- [x] B03: Reaktivieren stellt Modell, Effort und Default-Capabilities wieder her; genau ein Besitzer. Globaler Icon-Reset setzt alles zurück.
- [x] B04: Katalog, Prompt und CLI von festen Rollen auf Capability-Besitzer umstellen. Keine Übersetzung des alten Rollenvertrags.
- [x] B05: Jev an/aus, Vanilla ohne Key; explizite Zuweisungen ohne API-Aufruf.
- [x] B06: Reset-Zyklen, Ownership, deaktivierte Prompt-Abschnitte und neue GPT-IDs gezielt testen: 7 Website- und 21 Rust-Tests bestanden.
- [x] B07: Browser prüft Drag-and-drop, Modellwechsel, Luna-Reaktivierung, Vanilla-Prompt, Kopieren und globalen Reset. Typecheck/Build/Clippy sauber; React Doctor 100/100.
- [x] B08: Neue Capability-Klassifikation mit `jev-1.13.0` live geprüft: Implementation → Sol (0,95), Planning → Astra (0,99). Ein erster Aufruf hatte wegen fehlender Ausführungsdetails zu vorsichtig abstain gewählt; die Klassifikationsregel wurde präzisiert. Insgesamt 3 Aufrufe, 2.090 Input-/225 Output-Tokens; kein Desktop-Dispatch und kein Benchmark.

Die folgenden Punkte dokumentieren vorherige Arbeitsschritte; B01–B08 ersetzen
den früheren Advanced-/Rollen-Ansatz.

## Erledigter Fokuswechsel

- [x] C01: Aktiven Cursor-, Claude-Code-, OpenCode-, Pi- und Planr-Support entfernen.
- [x] C02: Terra und zusätzliche Modell-/Provider-/Preset-Auswahl entfernen.
- [x] C03: Website auf Worker, Advisor und optionale Luna-Koordination reduzieren.
- [x] H01: Evidence-/Retained-Evidence-Infrastruktur, Runtime-Zertifizierung, alte Fixtures und Oracles entfernen.
- [x] H02: CI-Router, Metatests, Alchemy-Vorabcheck und Cloudflare-Wrapper entfernen; direkte CI und Deployment-Befehle verwenden.
- [x] H03: Ownership prüfen. Routing, Rollen, HTTP und Cleanup haben je einen Besitzer; siehe [Ownership](docs/ownership.md).
- [x] H08: Die frühere Terra-/Planr-Installation dieses Checkouts deinstallieren; Ownership bereits fehlender Dateien korrekt entfernen.

## Zweiter Hard-Cut

- [x] R01: Eigenen Subagent-Installer samt UI, Setup-Rezepten, Bundle-Erzeugung, Doctor, Update, Rollback und Installationsprüfungen entfernen. Nur `status` und `uninstall` für vorhandene Installationen behalten. Normale Codex-Subagents bleiben Sache von Codex.
- [x] R02: Statische Routen, Fallbacks und Host-/Preset-Metadaten entfernen. `catalog.toml` und `src/catalog.rs` liefern die Task-Definitionen direkt an Jev und die Website.
- [x] R03: CLI-Ownership-TOML, kopierte Befehlslisten, Dokumentations-Dateinamen-Check und Quelltext-Architekturtest entfernen. Verhaltens-, HTTP-, Paket- und Cleanup-Prüfungen behalten.
- [x] R04: Überlappende Dokumentation auf Routing, Ownership und Paket-/Release-Regeln zusammenführen. Historische Grok-/Pi-Vergleiche und vier Bilder aus dem Produktauftritt entfernen; sie bleiben im Git-Verlauf.
- [x] R05: Bundle-Downloads, `scripts/build-site.mjs` samt Tests, Website-/Installer-Parität und ungenutzte Coverage-Abhängigkeit entfernen. Astro baut die Website direkt.
- [x] R06: Persistierte Installationsmanifeste und Transaktionsjournale isoliert in `src/cleanup.rs` lesen. Veränderte Dateien und fremde Codex-Einstellungen erhalten; Pfade, Symlinks und Recovery-Grenzen prüfen.

## TypeSafe/Jev

- [x] J01: TypeSafe-Skill unter `~/.agents/skills/typesafe-ai` verwenden; dokumentierten HTTP-/Confidence-Vertrag prüfen.
- [x] J02: Externe Referenzen prüfen; Repositories und Referenznotizen vollständig außerhalb dieses Projekts halten.
- [x] J03: Einen kleinen `route`-Befehl mit begrenztem JSON-Input und einem Jev-Choice-Aufruf implementieren. Modell und Reasoning kommen aus der Rollendefinition.
- [x] J04: Explizite Rollenwahl ohne API-Aufruf; Unsicherheit ohne Zuweisung. HTTP-Fehler, ungültige Antworten und Zeitüberschreitung erzeugen keine Modellzuweisung.
- [x] J05: API-Key nur aus der Umgebung; tatsächliche Usage ausgeben. Keine Dateisammlung, kein Session-Speicher, Proxy oder Routing pro Tool-Aufruf.
- [x] J06: Entscheidungsregeln, lokalen HTTP-Vertrag und CLI-Verhalten prüfen.
- [x] J09: [Desktop-Handoff festlegen](docs/routing.md#dispatch-end-turn-and-resume): eine Entscheidung je unklarem Schritt, feste Besitzer direkt verwenden, Modell/Reasoning beim Dispatch, Rücknachricht an den Aufrufer.
- [x] J10a: `handoff` implementieren und offline prüfen: konkrete Task-/Host-ID, fertige Codex-Tool-Argumente, begrenzter Kontext, nur konfigurierte Rollen für Jev, Unsicherheit ohne Dispatch, explizites `continue_here` für den aktuellen Thread.
- [x] J10b: Skill auf ereignisbasierte Handoffs umstellen: Ziel einmal prüfen, senden, Turn beenden, `Result for:` dem offenen Auftrag zuordnen. Rückadresse mit Thread und Host in der CLI-Ausgabe. Kein Polling, keine Bestätigungsnachrichten, keine zweite Runtime oder Subagents.
- [x] J07: Fünf echte Requests mit `jev-1.13.0` geprüft: Worker, Advisor, Orchestrator, fehlender Kontext und vorbereiteter Advisor-Handoff mit ausschließlich konfigurierten Rollen. Alle fünf entsprechen den vorab erwarteten Smoke-Ergebnissen; API-Usage insgesamt 2.974 Input-/234 Output-Tokens. Kein Codex-Dispatch ausgeführt, keine Aussage zu Benchmark-Qualität oder Einsparung.
- [ ] J10/V03: Desktop live prüfen: Bootstrap mit zwei/drei Rollen, Wiederverwendung und ausstehende Task-Erstellung, Jev für einen unklaren Schritt, Rücknachricht startet Folgeturn, Arbeit/Prüfung wird abgeschlossen. Keine Ergebnis-Echos, verlorenen Fortsetzungen oder doppelten Tasks. Die Browser-Prüfung des Generators ersetzt diesen Test nicht.
- [ ] J08/V04: Sol allein, Sol + Astra und optional Luna sowie Jev-Vorschläge auf gleichen Aufgaben vergleichen. Unnötige Astra-Aufrufe, übersehene schwierige Fälle, Abstention, Latenz, Nutzung und Nacharbeit messen. Schwelle `0.7` getrennt kalibrieren; API-Kosten und Abo-Quota auseinanderhalten.

## Prüfung und Release

- [x] V01: 31 Rust-Tests, Clippy, 6 npm-/Release-Script-Tests und Paket-Inventar bestehen. Der vorbereitete Handoff läuft auch aus dem gepackten Quellcode; der Skill ist dort enthalten. Optimierten lokalen 1.0.0-Build und dessen Handoff-Ausgabe ohne Modellaufruf geprüft.
- [x] P01: Einen gemeinsamen Bootstrap-Prompt statt drei Rollen-Prompts erzeugen. Modellvorschläge und Defaults aus `catalog.toml`; entfernte Prompt-Felder nicht doppelt weiterpflegen.
- [x] P02: Eingeklappte Advanced-Checkboxen für Review, Validierung, 3D und visuelle Prüfung. Luna kann Validierung übernehmen. Inaktive Bereiche und abgewählte Luna-Rolle fehlen vollständig im Prompt.
- [x] P03: 64 Kombinationen in einem kompakten Prompt-Vertragstest prüfen; Browser-Umschaltung kontrolliert zwei/drei Rollen, alleinigen Test-Besitzer, Specialist-Abschnitte und zurückgesetzten Kopierstatus.
- [x] P04: Rücknachrichten- und Modellvertrag: 15 Library-, 5 CLI-Handoff- und 2 CLI-Routing-Tests bestehen. Gewählte Modelle/Reasoning bleiben bei expliziten und Jev-Zuweisungen erhalten; ungültige Einstellungen scheitern vor dem API-Aufruf. Clippy ohne Warnungen.
- [x] P05: Modellvorschläge, Reasoning und Rollen-Defaults zentral in `catalog.toml`. Advanced bietet Modellfeld mit Vorschlägen und freier GPT-ID sowie Reasoning pro aktiver Rolle. Browser prüft neue IDs, Modellwechsel, gesperrtes Kopieren bei ungültiger Kombination und korrekten Clipboard-Inhalt. Kein Release unbekannter Modelle behauptet; Codex prüft Verfügbarkeit.
- [x] V02: 8 Website-Tests, Typecheck und Build bestehen. Browser prüft dynamischen Prompt und identischen Clipboard-Inhalt. Den React-Cache-Konflikt durch getrennte Vite-Caches pro Modus behoben.
- [x] V05: React-Doctor-Diffscan: 100/100, keine Diagnose. Die zwei pnpm-Policy-Hinweise des früheren Vollscans (49/100) bleiben eine separate Wartungsentscheidung, kein Grund für weitere Routing-Infrastruktur oder eine globale Policy-Änderung in diesem Release.
- [x] V06a: Cargo, npm, xtask und Changelog auf 1.0.0 vorbereiten; README, Website und Routing-Dokumentation kennzeichnen den Stand als unveröffentlichte Vorschau.
- [x] V06b: Vorhandenen `pnpm security:check` ausführen. `time` im nicht aktivierten Cookie-Abhängigkeitspfad auf die gepatchte Version 0.3.47 aktualisieren. Rust 1.85 baut und testet den aktiven Graph weiterhin; erneuter Scan ohne Befund.
- [ ] V06: Gesamten Release-Diff abschließend reviewen, nach erfolgreicher Live-Prüfung Vorschauhinweise entfernen und Release-Datum setzen.
- [ ] V07: Native Release-Artefakte aller vier Plattformen und Provenance aus dem finalen Commit erzeugen, Release-Gates ausführen und Paket/Website koordiniert veröffentlichen. Kein Commit, Tag, Push oder Deployment ausgeführt.

**Nächster Schritt:** Den neuen Startprompt für eine konkret benannte Aufgabe
live ausführen (J10/V03); danach der kleine Vergleich J08/V04.
Der TypeSafe-Zugang ist geprüft.
Routing-Optimierung ist erst nach Messungen belegt. Tool-Routing bleibt außerhalb
des aktuellen Builds.
