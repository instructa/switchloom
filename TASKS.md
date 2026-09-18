# Switchloom — Taskliste

Stand: 18. September 2026. Die 1.0-Vorbereitung ist noch nicht veröffentlicht.
Haken bedeuten im genannten Umfang geprüft.

**Aktuell:** Version 1.0.0 ist lokal vorbereitet, noch nicht releasefähig.
Die Hauptansicht ist das Capability-Board: Luna max zuerst, Sol medium, Astra
high. Alle Modelle sind austauschbar und deaktivierbar. Jev ist standardmäßig
an. Vanilla nutzt denselben Handoff ohne TypeSafe-Aufruf und ohne API-Key. Der Prompt verwendet nur aktive
Zuweisungen. Der echte Desktop-Ablauf bleibt offen; zwei Pokedex-Piloten sind ausgewertet.
TanStack Start mit öffentlichem Jev-Playground und lokalem Codex-Harness ist
implementiert. Der im Pilot beobachtete Jev-Abbruch zwischen Fähigkeiten desselben
Ziel-Tasks ist im Code korrigiert und im zweiten Pilot live überwunden. Beide
persistenten Teams erreichten dort das Zeitlimit vor der finalen Abnahme;
ein Qualitäts- oder Kostenvorteil durch Jev ist nicht belegt. Desktop-Abnahme,
belastbare Wiederholungen und Deployment bleiben offen.

## Routing-Review

- [x] A01: Routingvertrag und externe Referenzen prüfen. 13 Live-Jev-Proben mit dem damaligen Katalog reproduzierten CLI-Suche → Computer/Astra sowie gewöhnliche Implementierungen → Planning/Astra. Ein isolierter Versuch mit präziseren Kriterien korrigierte diese Proben ohne geänderte Schwellen. Die folgenden Punkte übertragen die Korrekturen in den Produktcode; keine unabhängige Kalibrierung.
- [x] A02: Mechanical umfasst einfache Such-/Leseaufgaben und eindeutige Befehle; Computer/Browser verlangen GUI-Bedienung. CLI/MCP allein bestimmt weder Fähigkeit noch Schwierigkeit. Beide Suchfälle live korrekt an Luna geroutet.
- [x] A03: Gewöhnliche Implementierung startet ohne vorherigen Advisor-Plan. Planning bleibt für konkrete blockierende Entscheidungen oder explizite Planungsaufträge. Drei Implementierungsproben live an Sol, eine Architekturentscheidung an Astra geroutet.
- [x] A04: Jev besitzt die Auswahl neuer unzugewiesener Arbeitsschritte. Kern, Harness und Desktop-Prompt verwenden `routing.mode` mit `jev` oder `assigned`. Der automatische Koordinator kann keine Capability setzen. Explizite Benutzerzuweisungen und gespeicherte Fortsetzungen bleiben direkt.
- [x] A05: Koordination bleibt Board-Zuständigkeit, ist aber keine Arbeitsfähigkeit für Jev. Den Plan-Schalter entfernt. Luna → Astra → Luna → Sol → Luna mit drei echten persistenten Tasks geprüft. Blockierter Sol → Beratung → Wiederaufnahme desselben Auftrags ohne erneute Klassifikation zusätzlich im Harness-Test geprüft.
- [x] A06: Fehlerdiagnose zeigt den bereinigten TypeSafe-HTTP-Status. Während der Untersuchung lieferten sowohl direkte als auch lokale Aufrufe upstream HTTP 503; danach acht lokale Live-Requests erfolgreich. Das erklärt den beobachteten Ausfall während dieser Untersuchung, beweist aber nicht rückwirkend die Ursache jedes früheren HTTP 502.
- [x] A07: Eigene Vergleichsvariante „Coordinator + subagents“ ergänzt: Delegation zwischen den konfigurierten Modellen ausdrücklich erlaubt. Vanilla und persistente Teams deaktivieren Subagents pro Task. Der normale Desktop-Prompt behält die Same-Model-Regel; Codex bietet dafür keine zuverlässige technische Sperre.
- [ ] A08: Originale Advisor-Antwort mit Herkunft und prozesslokal deaktivierte Codex-Memory implementiert; Wiederaufnahme mit unverändertem Quellergebnis im Test geprüft. Den vollständigen Blocker → Advisor → gleicher Worker-Ablauf zusätzlich live prüfen. Gemeinsame Projektdateien bleiben zugänglich.
- [x] A09: PNG-Referenz pro Run kopieren, beim ersten Turn je Task anhängen und Prompt-/Bild-Hashes speichern. Subagent-Ereignisse/Usage separat erfassen; fehlende Werte als unvollständig kennzeichnen. 22 Website-Tests, Build/Typecheck und React Doctor 100/100 bestehen. Nativer Preflight ohne Modell-Turn bestätigt Luna max und Memory aus.
- [x] A10: API-Zugang für Luna, Sol und Astra im ersten Preflight bestätigt. Explizite Logins nutzen einen flüchtigen Credential-Store; gespeicherter Login bleibt erhalten. Lokales `OPENAI_API_KEY` meldet den Harness nach Neustarts automatisch an. Vier Varianten und gemeinsames Prüfprotokoll außerhalb des Repos vorbereitet, Vanilla-Baseline ist Astra high. Vorhandene Zeit-/Turn-Limits verwenden; kein zusätzliches Budget-System bauen.
- [x] A11: Erster Luna/Subagent-Preflight über API: Luna max, Sol medium und Astra high tatsächlich verwendet; ca. 13 Minuten und 1,88 USD geschätzter Verbrauch. App gebaut, aber Harness brach bei einer MCP-Rückfrage zum Screenshot-Export ab. Ergebnis außerhalb des Repos erhalten und nicht als abgeschlossener Vergleich gewertet. Zwei konkrete Fehler korrigiert: Rückmeldungen an den Coordinator erzeugen keinen Subagent-Eintrag; interaktive MCP-Rückfragen erhalten `cancel`, ohne den gesamten Lauf zu stoppen. Sieben gezielte Runner-Tests und Typecheck bestehen.
- [x] A12: API-Zugang wiederhergestellt. Der zuvor geladene Umgebungs-Key war nicht der beabsichtigte Key. Nach Aktualisierung bestätigt ein echter Responses-Minimalaufruf HTTP 200; der lokale Harness ist mit dem neuen Key im flüchtigen API-Modus angemeldet. Drei frühere Preflights bleiben separat erhalten. Vier frische Vergleichsläufe sind gestartet.
- [x] A13: Auswahl bei gleichem Ziel-Task im Rust-Kern korrigiert: Review 0,53 / Visual 0,47 kann an den gemeinsamen Besitzer gehen; Verschieben einer Capability zu einem anderen Besitzer und fehlender Kontext verhindern diesen Dispatch. Jev-Confidence bleibt unverändert sichtbar; `owner_probability` ist separat. Der zentrale Abschlussvertrag verlangt angeforderte visuelle Prüfung durch den konfigurierten Besitzer und dokumentierte Einschränkungen. Der erste Worker erhält den Originalauftrag nur einmal. 18 Library-, 6 Handoff- und 24 Website-Tests sowie Build/Typecheck bestehen. Live-Smoke nach präziserem Koordinator-Briefing: Sol → Luna → Astra → Luna, vier Turns, zwei Jev-Aufrufe, 217 Sekunden, gerenderte Seite bei 1440×900 und 390×844 geprüft. In Runde 2 dispatcht Implementation 0,64 + Validation 0,09 erfolgreich an denselben Sol-Task; vollständige Pokedex-Abnahme des Jev-Teams bleibt wegen Zeitlimit offen.
- [x] A14: Zweiten Pokedex-Pilot am 18.09. ausgeführt und unabhängig ausgewertet: Astra abgeschlossen, 6,98 USD, 14 Checks bestanden; Luna/native Subagents abgeschlossen, 3,41 USD, 11 bestanden / 2 Fehler / 1 offen; Team ohne Jev am 30-Minuten-Limit, 4,85 USD, 13 bestanden / 1 offen; Team mit Jev am Limit vor finalem Astra-Urteil, 6,72 USD, 12 bestanden / 1 Fehler / 1 offen. Vier Jev-Entscheidungen kosten zusätzlich geschätzt 0,0003675 USD. Alle vier Projekte bestehen eigene Tests, Typecheck und Build. 45 eingefrorene Harness-Dateien sowie Prompt-/Bild-Hashes unverändert. Cache-Read/Write und tatsächliche Modelle inklusive nativer Subagents erfasst; Desktop-/Mobilbilder vereinheitlicht. Browserausfälle, Freigabewartezeiten und teilweise parallel laufende Beobachterprüfungen begrenzen Zeitvergleiche. Ein Versuch je Variante, keine allgemeine Spar- oder Qualitätsaussage. Bericht und Rohdaten außerhalb des Repos unter `<external-benchmark-store>/round-2/evaluation/`.
- [x] A15: Externe Research mit beiden Piloten abgeglichen. Capability-Board und persistente Tasks bleiben; für diesen Workload ist kein Jev-Vorteil belegt. Weitere bezahlte Vergleiche warten auf eine konkrete Anwendung mit unterschiedlichen Aufgaben. Die englischen Archivseiten dokumentieren Ergebnisse und Grenzen.

Der native Codex-App-Server hatte bei drei Smoke-Projekten selbst Trust-Einträge
angelegt. Diese normalen Einträge sind erwünscht und bleiben bestehen. Tasks
starten regulär mit workspace-write; der Umweg über read-only wurde entfernt.
Modell- und Agent-Einstellungen werden nicht in die persönliche TOML geschrieben.

Aktueller Nachweis für A02–A06: 26 fokussierte Rust-Tests, 19 Website-Tests,
Build, Typecheck, Clippy und React Doctor 100/100 bestanden. Acht Live-Routingproben
entsprachen den erwarteten Fähigkeiten. Ein echter Jev/Codex-Run implementierte
und testete eine Funktion mit Sol, danach Rückgabe an Luna: zwei Turns,
ein Jev-Aufruf. Der separate Team-Run ohne Jev benötigte fünf Turns.
Die Runs verwendeten prozesslokal deaktivierte Codex-Subagents; persönliche
Codex-Konfigurationsdateien wurden nicht verändert. Kein Pokedex-Qualitätsbenchmark.

## Playground und lokale Benchmarks

Entwurf und Grenzen: [Playground](docs/playground.md).

- [x] G01: Codex-Schnittstelle prüfen. App Server unterstützt Tasks, Turns, Ereignisse, Login und Freigaben. Lokaler stdio-Handshake und `account/read` mit Codex 0.154.0 erfolgreich; bestehender ChatGPT-Login erkannt, kein Modellaufruf.
- [x] G02: Gemeinsamen Rust-Routingkern im öffentlichen Server verfügbar machen; Rust-WASM wird in Cloudflare und Node verwendet, keine zweite Routing-Implementierung in TypeScript.
- [x] G03: Website auf aktuelle stabile TanStack-Start-Version umstellen, Homepage/Board erhalten und Navigation Playground ergänzen; Astro danach entfernen.
- [x] G04: Prompt-Limit zunächst 4.000 Zeichen, Jev an/aus, Capability-Auswahl ohne Jev, Graph, echte Wahrscheinlichkeiten und Verlauf implementieren.
- [x] G05: Öffentlichen TypeSafe-Key serverseitig halten; atomare Session-/IP-/Gesamtlimits, begrenzte Requests und bereinigte Logs umsetzen. Startvorschlag: 10 Jev-Aufrufe je Session/Tag.
- [x] G06: Serverflag `DEVELOPER_MODE=false` und getrennten lokalen Einstieg ergänzen. Public-Build enthält keine Login-, Prozess-, Dateisystem- oder Run-Endpunkte; Localhost-Zugriff zusätzlich absichern.
- [x] G07: Lokalen Codex-App-Server anbinden: vorhandenes Login, explizites API-Key-/ChatGPT-Login, TypeSafe-Key lokal, persistente Tasks, Handoffs, echte Ereignisse, Freigaben und Stop.
- [x] G08: Ausgabeordner wählen, neue isolierte Run-Verzeichnisse anlegen, Konfiguration/Ergebnisse speichern und Screenshots/Logs/Checks anzeigen; Vorschau vom Steuerungs-UI isolieren.
- [x] G09: Ersten Pokedex-Pilot in vier Modi abgeschlossen und unabhängig ausgewertet. Astra solo: 14/14 Funktionsfälle, geschätzt 6,57 USD; Luna/Subagents: 13/14, 2,13 USD; Team ohne Jev: 14/14, 2,53 USD; Team mit Jev: App 14/14, 1,95 USD, Workflow vor Review blockiert. Jev separat etwa 0,000187 USD. Prompt-/Bild-Hashes identisch, Harness unverändert. Beobachterverzögerter erster Luna-Versuch und drei Preflights bleiben erhalten; Astra nach Unterbrechungen im selben Kontext mit `proceed` fortgesetzt. Zeitvergleiche eingeschränkt, visuelle Prüfung nicht verblindet, keine allgemeine Qualitäts-/Sparprognose. Bericht und Screenshots liegen außerhalb des Repos unter `<external-benchmark-store>/evaluation/`.
- [x] G10: Grenzen gezielt prüfen: Public-Endpunkte nicht vorhanden, lokale Fremd-Origin-Aufrufe abgelehnt, Quota bei parallelen Requests korrekt, keine Secrets in Client/Logs, sichere Pfade, Abbruch und unveränderte Board-Funktionen.
- [x] G11: Englisches Benchmark-Archiv unter `/benchmarks/` und Detailseiten für beide Pokedex-Runden ergänzen. Kosten, Laufzeiten, tatsächliche Modelle, Token-/Cache-Zahlen, Originalbilder und unabhängige Checks archivieren; fehlende Reviews, Zeitlimits und unbewertete Fälle sichtbar lassen. Ausgewählte Daten und 16 Screenshots liegen in der Website, private Run-Logs bleiben extern. 24 Website-Tests, Build und Typecheck bestehen; Desktop/Mobile, Bildwechsel, Prüfergebnisse, Archivnavigation und 404 geprüft. Neue Archivkomponenten ohne React-Doctor-Befund. Kein Deployment und keine neuen bezahlten Runs.

Verifiziert am 17.09.: 18 Website-Tests und 28 Rust-Routing-/Handoff-Tests,
Build, Typecheck, Clippy und React Doctor 100/100. Öffentlicher Worker:
15 parallele Anfragen → 10 zugelassen, 5 limitiert (Provider gemockt), lokale
Endpunkte 404; Developer-Flag im Public-Build abgelehnt. Kein lokaler Harness
im Client-/Worker-Bundle. Secret-Check sauber; zusätzlicher Dependency-Scan
inklusive Dev-Abhängigkeiten nach gezielten Sicherheitsupdates ohne Befund.
Echte Runs: Sol erstellt und prüft eine Datei; Luna → Sol → Luna verwendet
zwei persistente Tasks in drei Turns (0 Jev-Aufrufe). Separater echter Jev-Test:
Implementation → Sol, 864 ms, 1.274 Input-/94 Output-Tokens. Screenshot einer
Codex-generierten HTML-Seite mit 1440 × 1000 sowie Ergebnisimport im Browser
geprüft. Kein Qualitätsbenchmark.

## Capability-Board

- [x] B01: Vorschau durch die Hauptansicht ersetzen; kein paralleler Generator.
- [x] B02: Capabilities frei verschieben; Browser use separat aufnehmen.
- [x] B03: Reaktivieren stellt Modell, Effort und Default-Capabilities wieder her; genau ein Besitzer. Globaler Icon-Reset setzt alles zurück.
- [x] B04: Katalog, Prompt und CLI von festen Rollen auf Capability-Besitzer umstellen. Keine Übersetzung des alten Rollenvertrags.
- [x] B05: Jev an/aus, Vanilla ohne Key; explizite Zuweisungen ohne API-Aufruf.
- [x] B06: Reset-Zyklen, Ownership, deaktivierte Prompt-Abschnitte und neue GPT-IDs gezielt testen: 7 Website- und 21 Rust-Tests bestanden.
- [x] B07: Browser prüft Drag-and-drop, Modellwechsel, Luna-Reaktivierung, Vanilla-Prompt, Kopieren und globalen Reset. Typecheck/Build/Clippy sauber; React Doctor 100/100.
- [x] B08: Neue Capability-Klassifikation mit `jev-1.13.0` live geprüft: Implementation → Sol (0,95), Planning → Astra (0,99). Ein erster Aufruf hatte wegen fehlender Ausführungsdetails zu vorsichtig abstain gewählt; die Klassifikationsregel wurde präzisiert. Insgesamt 3 Aufrufe, 2.090 Input-/225 Output-Tokens; kein Desktop-Dispatch und kein Benchmark.

- [x] B09: Klassifikatortexte getrennt von Auftragstexten. Fehlender Kontext ist ein eigenes Noul. Ein Kandidat überspringt Jev. `suggest` sendet nicht. Judge ist `jev-1.13.0`. Gelabelte Fälle liegen in `evaluations/capability-cases.toml` und sind noch nicht live gemessen. Desktop-Ablauf bleibt offen.

- [x] B10: Mechanical tasks als eigene Capability mit Luna als Default ergänzen; Board, Prompt, Playground und Jev lesen denselben Katalog. Deaktivieren/Reaktivieren stellt auch diese Zuweisung wieder her. 18 Website- und 18 Library-Tests sowie Build/Typecheck bestehen. Zwei Live-Jev-Proben: exakte Umbenennung → mechanical/Luna (0,99); neue Backoff-Logik → implementation/Sol als Vorschlag (0,68, kein Dispatch). Keine Kalibrierung.

- [x] B11: Bestehende `visual`-Capability zu „Visual design & review“ erweitern: visuelle Richtung und Review, frei zuweisbar mit Astra als Default. Kriterien für Architektur, Code-Review und eigenständige GUI-Aufträge präzisiert; beiläufige Browsernutzung bleibt Teil der Implementierung. Vier gelabelte Fälle ergänzt. Am 18.09. sieben Live-Jev-Proben korrekt und dispatchfähig: Designbrief und Live-Visual-Review → visual; Umsetzung mit Browser-Check → implementation; Code-Review → review; Architektur → planning; Browser-Reproduktion → browser; CLI-Dateisuche → mechanical. 9 Rust- und 23 Website-Tests, Build und Typecheck bestanden; neue Bezeichnung im Playground sichtbar. Diese Proben sind keine Kalibrierung; spätere Laufnachweise stehen unter A13/A14.

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
- [x] R05: Bundle-Downloads, `scripts/build-site.mjs` samt Tests, Website-/Installer-Parität und ungenutzte Coverage-Abhängigkeit entfernen. Damals Astro direkt; inzwischen durch TanStack Start ersetzt (G03).
- [x] R06: Persistierte Installationsmanifeste und Transaktionsjournale isoliert in `src/cleanup.rs` lesen. Veränderte Dateien und fremde Codex-Einstellungen erhalten; Pfade, Symlinks und Recovery-Grenzen prüfen.

## TypeSafe/Jev

- [x] J01: TypeSafe-Skill unter `~/.agents/skills/typesafe-ai` verwenden; dokumentierten HTTP-/Confidence-Vertrag prüfen.
- [x] J02: Externe Referenzen prüfen; Repositories und Referenznotizen vollständig außerhalb dieses Projekts halten.
- [x] J03: Einen kleinen `route`-Befehl mit begrenztem JSON-Input und einem Jev-Choice-Aufruf implementieren. Modell und Reasoning kommen aus der Rollendefinition.
- [x] J04: Explizite Rollenwahl ohne API-Aufruf; Unsicherheit ohne Zuweisung. HTTP-Fehler, ungültige Antworten und Zeitüberschreitung erzeugen keine Modellzuweisung.
- [x] J05: API-Key nur aus der Umgebung; tatsächliche Usage ausgeben. Keine Dateisammlung, kein Session-Speicher, Proxy oder Routing pro Tool-Aufruf.
- [x] J06: Entscheidungsregeln, lokalen HTTP-Vertrag und CLI-Verhalten prüfen.
- [x] J09: [Desktop-Handoff festlegen](docs/routing.md#dispatch-end-turn-and-resume): Jev für jedes neue unzugewiesene Arbeitsziel, gespeicherte Fortsetzungen direkt, Modell/Reasoning beim Dispatch, Rücknachricht an den Koordinator.
- [x] J10a: `handoff` implementieren und offline prüfen: konkrete Task-/Host-ID, fertige Codex-Tool-Argumente, begrenzter Kontext, nur konfigurierte Rollen für Jev, Unsicherheit ohne Dispatch, explizites `continue_here` für den aktuellen Thread.
- [x] J10b: Skill auf ereignisbasierte Handoffs umstellen: Ziel einmal prüfen, senden, Turn beenden, `Result for:` dem offenen Auftrag zuordnen. Rückadresse mit Thread und Host in der CLI-Ausgabe. Kein Polling, keine Bestätigungsnachrichten, keine zweite Runtime oder Subagent-Ersatz für persistente Tasks.
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
- [ ] V07: Native Release-Artefakte aller vier Plattformen und Provenance aus dem finalen Commit erzeugen, Release-Gates ausführen und Paket/Website koordiniert veröffentlichen. Noch kein Release-Tag oder Deployment ausgeführt.
- [x] V08: Branch vor dem Push geprüft. BetterLeaks für Git-Historie und vollständigen Commit-Kandidaten ohne Befund; Trivy inklusive Entwicklungsabhängigkeiten ohne Befund; bestehende Datei-/Inhalt-Hooks und Workflow-Scan bestanden. Lokale Laufwerkspfade entfernt, Paket-Dokumentation korrigiert und veraltete Release-Pfade gelöscht. Cargo enthält 27 Dateien (40,2 KiB komprimiert), keine Website oder Benchmark-Bilder. 39 Rust-, 11 Node- und 24 Website-Tests sowie Clippy, Build und Typecheck bestanden; alle 33 Produkt-Tests zusätzlich aus dem entpackten Cargo-Paket bestanden. Öffentliche Bundles enthalten keine lokalen Executor-/Login-Implementierungen; Benchmark-PNGs enthalten keine Text-/EXIF-Metadaten.

React Doctor wurde diesmal auch auf ungetrackte Dateien angewendet. Die zwölf
Fehler zu unreinen State-Updatern betreffen den eigenen asynchronen `action`
Event-Handler in `DeveloperPanel.tsx`, keinen React-State-Updater. Keine
Suppressions ergänzt. Fehlende HTTP-Statusprüfungen beim lokalen Laden und ein
normaler Anchor für interne Navigation sind korrigiert. Die verbleibenden
Hinweise betreffen überwiegend Komplexität, optionale Parallelisierung und
Render-Optimierung; dies ist kein pauschaler 100/100-Nachweis.

**Nächster Schritt:** Vor der Veröffentlichung die native Release-Matrix
und den Desktop-Test abschließen.
Release-Abnahme und Deployment sind noch offen. Der Desktop-Test J10/V03
bleibt zusätzlich offen; der App Server ersetzt diesen Nachweis nicht.
Der TypeSafe-Zugang ist geprüft.
Routing-Optimierung ist erst nach Messungen belegt. Tool-Routing bleibt außerhalb
des aktuellen Builds.

## Review-Fixes

- [x] Eine einzelne Capability ohne expliziten Auftrag erzeugt keinen Dispatch; CLI-Regression für `Continue` mit ausschließlich Browser use.
- [x] Koordination ist ausschließlich Ablaufzuständigkeit; der frühere Plan-Schalter und seine Gates sind entfernt. Gewöhnliche Implementierung braucht keinen vorgeschalteten Advisor-Plan.
- [x] Rust wertet den Katalog-Regex mit `regex-lite` aus; keine zweite Zeichenregel. Frontend und Rust prüfen vollständige Modell-IDs, einschließlich Tests mit geändertem Pattern.

## Desktop-Test vorbereiten

- [x] Konkrete Benutzeranleitung, isolierter Testauftrag und Protokollregeln: [Desktop-Smoke-Test](docs/desktop-smoke.md). Lokales Debug-Binary gebaut; keine globale Installation vorausgesetzt.
- [ ] Anleitung im neuen Codex-Task ausführen und tatsächlichen Bericht prüfen. Die Anleitung ist noch kein Durchführungsnachweis.
