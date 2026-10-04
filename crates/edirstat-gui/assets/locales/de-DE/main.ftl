# Menu Bar Dropdowns
file = Datei
view = Ansicht
help = Hilfe

# Menu Bar Actions
new-scan = 📁 Neuer Scan
save-snapshot = 💾 Schnappschuss speichern
load-snapshot = 📖 Snapshot laden

# Menu Bar Status
idle = Bereit

# View Menu Options
monospace-paths = 🅰 Monospace-Pfade
highlight-duplicates = ✨ Duplikate hervorheben
treemap-borders = 🔳 Treemap-Rahmen
treemap-style =  Treemap-Stil
treemap-style-vertical = Vertikaler Verlauf
treemap-style-offset-vertical = Versetzter vertikaler Verlauf
treemap-style-diagonal = Diagonaler Verlauf
treemap-style-cushion = Kissen-Schattierung
deletion-confirmation = 🗑 Bestätigung vor Löschen
trash-confirmation = ♻ Bestätigung vor In-den-Papierkorb-Verschieben
time-format = 🕒 Zeitformat
language = 💬 Sprache
layout-mode = Layout-Modus:
classic-layout = Klassisches Layout
windirstat-layout = WinDirStat-Layout
vis-mode-treemap = 📊 Treemap
vis-mode-plots = 📈 Diagramme
select-plot-label = Diagramm auswählen:
vis-mode-deduplicator = 👥 Duplikatsuche
search-filter-label = 🔍 Filter:

# Panel Toggles
toggle-left-panel = { $collapsed ->
    [true] ▶ Linkes Panel anzeigen (F9)
   *[false] ◀ Linkes Panel ausblenden (F9)
}

toggle-right-panel = { $collapsed ->
    [true] { $is_classic ->
        [true] ◀ Rechte Panel anzeigen (F11)
       *[false] ▶ Erweiterungstafel anzeigen (F11)
    }
   *[false] { $is_classic ->
        [true] ▶ Rechte Panel ausblenden (F11)
       *[false] ◀ Erweiterungstafel ausblenden (F11)
    }
}

collapse-all = ⏏ Alles einklappen
about = ℹ Über eDirStat
web-not-available = In der Web-Version nicht verfügbar

# Status Indicators
scanning-disk = Dateisystem wird gescannt...
scan-complete = Scan abgeschlossen
scan-cancelled = Scan abgebrochen
path-label = Pfad: { $path }
worker-threads = ⚡ { $count } Arbeiter-Threads
worker-threads-hover = Die Anzahl der parallelen Prozessorkerne für das Durchsuchen des Verzeichnisses (Work-Stealing).

# Stats Panel (Bottom)
directories-count = 📁 Ordner: { $count }
files-count = 📄 Dateien: { $count }
total-size = 💾 Gesamtgröße: { $size }
elapsed-time = ⏱ Zeit: { $time }
scan-speed = ⚡ Geschwindigkeit: { $speed }/s

# Selection Info
selection-path = Auswahl: { $path }
selection-items = Auswahl: { $count ->
    [one] 1 Element
   *[other] { $count } Elemente
}

# Plot Types
plot-size-distribution = 📊 Dateigrößenverteilung
plot-age-size = 🌌 Dateialter verglichen mit Dateigröße
plot-dir-composition = 🍰 Verzeichniszusammensetzung
plot-extension-boxplot = 📦 Dateigrößen nach Erweiterung
plot-temporal-timeline = ⏱ Verknüpfte zeitliche Verläufe
plot-deduplicator-waste = 👥 Duplikat-Platzverschwendung nach Erweiterung

# --- Deduplicator Strings ---
dedup-desc = Suchen und sicheres Entfernen von Dateien mit identischem Inhalt mithilfe kryptografisch sicherer BLAKE3-Hashes.
dedup-how-it-works = ℹ Funktionsweise
dedup-min-size = Mindestgröße:
dedup-ignore-system = Systemdateien ignorieren
dedup-ignore-hidden = Versteckte Dateien ignorieren
dedup-start-scan = ⚡ Duplikat-Scan starten
dedup-scan-first = Bitte scannen Sie zuerst ein Verzeichnis.
dedup-cancelled-msg = Scan wurde abgebrochen. Starten Sie einen neuen Scan, um Duplikate zu finden.
dedup-analyzing = Analysiere Dateien...
dedup-no-duplicates = Keine Duplikate gefunden. Verringern Sie die Mindestgröße oder scannen Sie einen anderen Ordner.
no-permission = Keine Berechtigung
hardlink-badge = Harte Verknüpfung
dedup-select-items = 🎯 Elemente auswählen...
dedup-select-all-but-oldest = 🎯 Alle außer der ältesten
dedup-select-all-but-newest = 🎯 Alle außer der neuesten
dedup-select-all-but-shortest = 🎯 Alle außer dem kürzesten Pfad
dedup-select-all-but-rootmost = 🎯 Alle außer der obersten (wurzelnächsten)
dedup-select-all-but-longest = 🎯 Alle außer dem längsten Pfad
dedup-pref-dir-pattern = Bevorzugtes Verzeichnismuster:
dedup-select-all-but-pref = 🎯 Alle außer dem bevorzugten Verzeichnis
dedup-clear-selection = ❌ Auswahl aufheben
dedup-link-menu = 🔗 Verknüpfen... ({ $count } Dateien)
dedup-link-menu-disabled = 🔗 Verknüpfen... (0 Dateien)
dedup-link-hardlinks = 🔗 Ausgewählte durch harte Verknüpfungen ersetzen
dedup-link-softlinks = 🔗 Ausgewählte durch symbolische Verknüpfungen ersetzen
dedup-remove-menu = 🗑 Löschen... ({ $count } Dateien, { $size })
dedup-remove-menu-disabled = 🗑 Löschen... (0 Dateien)
dedup-remove-trash = ♻ Ausgewählte in den Papierkorb verschieben
dedup-remove-delete = 🗑 Ausgewählte dauerhaft löschen
dedup-warning-title = ⚠ WARNUNG VOR DATENVERLUST
dedup-warning-desc = { $count ->
    [one] Alle Versionen von 1 Datei werden gelöscht
   *[other] Alle Versionen von { $count } Dateien werden gelöscht
}
dedup-warning-no-original = Keine Originalkopie bleibt übrig:
dedup-warning-details = Sie haben sowohl das Original als auch alle Duplikate der unten aufgeführten Dateien ausgewählt. Das Löschen führt wahrscheinlich zu dauerhaftem Datenverlust:
dedup-cancel-hover = Klicken zum Abbrechen des Scans
scan-cancel-hover = Klicken zum Abbrechen des Scans
dedup-current-label = Aktuell
dedup-phase1-size = Phase 1/7: Dateien nach Größe gruppieren...
dedup-phase1-filter = Phase 1/7: Ausschlusskriterien filtern...
dedup-phase2-prefix = Phase 2/7: Hashing der Dateianfänge (erste 4KB)...
dedup-phase3-midpoint = Phase 3/7: Hashing der Dateimitten...
dedup-phase4-suffix = Phase 4/7: Hashing der Dateiendungen (letzte 4KB)...
dedup-phase5-multirange = Phase 5/7: Mehrbereichs-Hashing großer Dateien...
dedup-phase6-full = Phase 6/7: Vollständiges BLAKE3-Hashing der verbleibenden Kandidaten...
dedup-phase7-validation = Phase 7/7: Abschließende Überprüfung der Zeitstempel...
dedup-phase-finished = Fertig in { $duration }! { $count } Duplikatgruppen gefunden. Potenzial freizugebender Speicherplatz: { $space }
dedup-scan-cancelled-with-error = Scan wurde abgebrochen: { $error }

# Deduplicator Table Headers
dedup-hdr-checkbox = [     ]
dedup-hdr-filename = Dateiname
dedup-hdr-directory = Elternverzeichnis
dedup-hdr-size = Größe
dedup-hdr-reclaimable = Einsparbar
dedup-hdr-created = Erstellt
dedup-hdr-modified = Geändert
dedup-copies-selected = ({ $count ->
    [one] 1 Kopie ausgewählt
   *[other] { $count } Kopien ausgewählt
})

# --- Explorer Details Panel ---
explorer-details-header = ℹ Details
explorer-deselect-hover = Auswahl aufheben
explorer-deselect-single-hover = Auswahl aufheben
explorer-selected-items-count = { $count ->
    [one] 1 ausgewähltes Element
   *[other] { $count } ausgewählte Elemente
}
explorer-total-size = Gesamtgröße: { $size }
explorer-files = Dateien: { $count }
explorer-directories = Ordner: { $count }
explorer-actions-title = Aktionen
explorer-actions-operations = Operationen:
explorer-action-refresh-hover = Alle ausgewählten Unterverzeichnisse aktualisieren
explorer-grid-type = Typ:
explorer-grid-size = Größe:
explorer-grid-bytes = Bytes:
explorer-grid-items = Elemente:
explorer-grid-files = Dateien:
explorer-grid-subdirs = Unterordner:
explorer-grid-user = Benutzer:
explorer-grid-group = Gruppe:
explorer-grid-permissions = Berechtigungen:
explorer-grid-path = Vollständiger Pfad:

# Explorer Type Names
type-symlink = Symbolischer Link
type-directory = Ordner
type-file = Datei

# Explorer Actions
explorer-action-copy-path = 📋 Pfad kopieren
explorer-action-open-file = 📄 Datei öffnen
explorer-action-open-manager = 🗁 Dateimanager öffnen
explorer-action-refresh-subtree = 🔄 Unterbaum aktualisieren
explorer-action-move-trash = ♻ In den Papierkorb verschieben
explorer-action-delete-permanently = 🗑 Dauerhaft löschen
explorer-action-refresh-directory = 🔄 Ordner aktualisieren

# Explorer Empty State
explorer-empty-state = Klicken Sie auf 'Neuer Scan', um die Speicherplatzbelegung zu analysieren.
choose-an-option = Option wählen
web-viewer = Web-Viewer
load-demo = 👁 Demo-Snapshot laden
placeholder-treemap = Das gescannte Dateisystem wird hier als Treemap visualisiert.
placeholder-plots = Das gescannte Dateisystem wird hier grafisch dargestellt.

# Treemap Zoom & Navigation
zoom-up = ⏶ Nach oben
zoom-reset = ❌ Zurücksetzen
zoom-to-dir = 🔍 In Treemap fokussieren
zoom-up-level = ⏶ Eine Ebene nach oben
zoom-empty-dir = Verzeichnis ist leer

# --- Extensions Panel ---
extensions-header = 📂 Erweiterungen
extensions-empty = Noch keine Statistiken erfasst.
extensions-hover-files = Dateien: { $count }

# --- Operations (Context Actions) ---
op-up-one-level = Eine Ebene nach oben
op-zoom-treemap = In Treemap fokussieren
op-refresh-entire-scan = Gesamten Scan aktualisieren
op-refresh-directory = Ordner aktualisieren
op-open-file = Datei öffnen
op-open-file-manager = Im Dateimanager öffnen
op-open-terminal = Terminal hier öffnen
op-copy-path = Pfad kopieren
op-copy-name = Name kopieren
op-move-trash = In den Papierkorb verschieben
op-permanently-delete = Dauerhaft löschen

# Toast Notifications
toast-already-root = Bereits auf der obersten Ebene
toast-navigated-up = Eine Ebene nach oben navigiert
toast-zoomed-treemap = Treemap auf Verzeichnis fokussiert
toast-refreshing-scan = Gesamter Scan wird aktualisiert...
toast-refreshing-dir = Ausgewählte Ordner werden aktualisiert...
toast-opened-file = Geöffnet: { $path }
toast-failed-open-file = Datei konnte nicht geöffnet werden: { $error }
toast-opened-manager = Im Dateimanager geöffnet: { $path }
toast-failed-open-manager = Dateimanager konnte nicht geöffnet werden: { $error }
toast-opened-terminal = Terminal geöffnet bei: { $path }
toast-failed-open-terminal = Terminal konnte nicht geöffnet werden: { $error }
toast-copied-paths = { $count ->
    [one] 1 Pfad in die Zwischenablage kopiert
   *[other] { $count } Pfade in die Zwischenablage kopiert
}
toast-copied-names = { $count ->
    [one] 1 Name in die Zwischenablage kopiert
   *[other] { $count } Namen in die Zwischenablage kopiert
}

# --- Modals ---
modal-remember-confirmation = Entscheidung für alle zukünftigen Dateien und Ordner merken
modal-process-multiple = Sie sind im Begriff, { $count } doppelte Dateien/Elemente zu verarbeiten:
modal-process-single = Sie sind im Begriff, den folgenden Pfad zu verarbeiten:
# Confirm Deletion/Trash/Link Modals
modal-delete-title = ⚠ WARNUNG: DAUERHAFTES LÖSCHEN
modal-delete-header = ⚠ Warnung: Dauerhaftes Löschen!
modal-delete-info = Gesamtgröße: { $size }
modal-delete-warning = Dies ist ein rekursiver Löschvorgang. Alle Dateien, Ordner und Unterverzeichnisse innerhalb der ausgewählten Pfade werden dauerhaft gelöscht und können nicht wiederhergestellt werden (der Papierkorb wird umgangen).
modal-delete-checkbox = Ich verstehe, dass Daten dauerhaft gelöscht werden und nicht wiederhergestellt werden können.
modal-delete-confirm = 🗑 Ja, dauerhaft löschen

modal-trash-title = ♻ IN DEN PAPIERKORB VERSCHIEBEN
modal-trash-header = ♻ In den Papierkorb verschieben
modal-trash-info = Gesamtgröße: { $size }
modal-trash-warning = Dies verschiebt die ausgewählten Pfade und all ihre Inhalte in den Papierkorb Ihres Systems, von wo sie später wiederhergestellt oder dauerhaft gelöscht werden können.
modal-trash-checkbox = Ich bestätige, dass ich dies in den Papierkorb verschieben möchte.
modal-trash-confirm = ♻ Ja, in den Papierkorb verschieben

modal-delete-duplicates-title = ⚠ WARNUNG VOR DAUERHAFTER DUPLIKATLÖSCHUNG
modal-delete-duplicates-header = ⚠ Warnung vor dauerhafter Duplikatslöschung!
modal-delete-duplicates-info = Freizugebender Speicherplatz insgesamt: { $size }
modal-delete-duplicates-warning = Alle ausgewählten Dateien werden dauerhaft gelöscht und können nicht wiederhergestellt werden (der Papierkorb wird umgangen).
modal-delete-duplicates-checkbox = Ich verstehe, dass Dateien dauerhaft gelöscht werden und nicht wiederhergestellt werden können.
modal-delete-duplicates-confirm = 🗑 Ja, Ausgewählte dauerhaft löschen

modal-trash-duplicates-title = ♻ DUPLIKATE IN DEN PAPIERKORB VERSCHIEBEN
modal-trash-duplicates-header = ♻ Duplikate in den Papierkorb verschieben
modal-trash-duplicates-info = Freizugebender Speicherplatz insgesamt: { $size }
modal-trash-duplicates-warning = Alle ausgewählten Dateien werden in den Papierkorb verschoben.
modal-trash-duplicates-checkbox = Ich bestätige, dass ich diese Dateien in den Papierkorb verschieben möchte.
modal-trash-duplicates-confirm = ♻ Ja, ausgewählte Dateien in den Papierkorb verschieben

modal-hardlink-duplicates-title = 🔗 DUPLIKATE DURCH HARTE VERKNÜPFUNGEN ERSETZEN
modal-hardlink-duplicates-header = 🔗 Duplikate durch harte Verknüpfungen ersetzen
modal-hardlink-duplicates-info = Zu verarbeitende Dateien insgesamt: { $count }. Kumulierte virtuelle Größe: { $size }
modal-hardlink-duplicates-warning = Dies löscht die ausgewählten Duplikate und ersetzt sie durch harte Verknüpfungen (Hardlinks) auf Dateisystemebene, die auf die verbleibende Originaldatei der jeweiligen Gruppe verweisen. Dadurch bleiben die Dateien visuell erhalten, während der physische Speicherplatz freigegeben wird.
modal-hardlink-duplicates-checkbox = Ich bestätige, dass ich die ausgewählten Dateien durch harte Verknüpfungen ersetzen möchte.
modal-hardlink-duplicates-confirm = 🔗 Ja, durch harte Verknüpfungen ersetzen

modal-softlink-duplicates-title = 🔗 DUPLIKATE DURCH SYMBOLISCHE VERKNÜPFUNGEN ERSETZEN
modal-softlink-duplicates-header = 🔗 Duplikate durch symbolische Verknüpfungen ersetzen
modal-softlink-duplicates-info = Zu verarbeitende Dateien insgesamt: { $count }. Kumulierte virtuelle Größe: { $size }
modal-softlink-duplicates-warning = Dies löscht die ausgewählten Duplikate und ersetzt sie durch symbolische Verknüpfungen (Softlinks) auf Dateisystemebene, die auf die verbleibende Originaldatei verweisen. Dadurch bleiben die Dateien visuell erhalten, während der physische Speicherplatz freigegeben wird.
modal-softlink-duplicates-checkbox = Ich bestätige, dass ich die ausgewählten Dateien durch symbolische Verknüpfungen ersetzen möchte.
modal-softlink-duplicates-confirm = 🔗 Ja, durch symbolische Verknüpfungen ersetzen

# Path Does Not Exist Modal
modal-path-not-exist-title = ❌ Pfad existiert nicht!
modal-path-not-exist-msg = Fehler: Der Pfad, den Sie löschen möchten, existiert nicht auf dem Datenträger.
modal-close-btn = Schließen
modal-details-label = Details: 
modal-cancel-btn = Abbrechen

# Elevation Recommended Modal
modal-elevation-title = ⚠ Administratorrechte empfohlen
modal-elevation-desc = eDirStat wird standardmäßig mit normalen Benutzerrechten ausgeführt. Windows setzt jedoch Administratorrechte voraus, um direkt auf den Datenträger zuzugreifen.
modal-elevation-mft-disabled = NTFS-MFT-Treiber unter Windows deaktiviert
modal-elevation-mft-desc = Ohne Administratorrechte kann der direkte MFT-Scanner nicht initialisiert werden. Die Dateianalyse greift auf den standardmäßigen Verzeichnisdurchlauf zurück, was den Scan bis zu 20 mal langsamer macht.
modal-elevation-relaunch-prompt = Möchten Sie die Anwendung jetzt mit Administratorrechten neu starten?
modal-elevation-continue-std = Als Standardbenutzer fortfahren
modal-elevation-relaunch-btn = 🛡 Als Administrator neu starten

# About Modal
modal-about-title = ℹ Über eDirStat
modal-about-author = Von: Cody Wyatt Neiman (xangelix) <neiman@cody.to>
modal-about-license-btn = 📜 Lizenz (MIT)
modal-about-desc1 = Ein hochperformantes Tool zur Analyse von Speicherplatz und Deduplizierung, geschrieben in Rust.
modal-about-desc2 = Bietet parallelen Verzeichnisdurchlauf mit Work-Stealing, komprimierte Schnappschüsse ohne Aufwand für Syntaxanalyse beim Laden sowie interaktive Baumdiagramme.
modal-about-desc3 = Die integrierte Deduplizierung führt eine mehrstufige Hashing-Pipeline aus, um Duplikate sicher zu finden und freizugebenden Speicherplatz unter Berücksichtigung bestehender Verknüpfungen zu ermitteln.
modal-about-licenses-btn = Quelloffene Lizenzen anzeigen
modal-about-version = v{ $version }

# How Deduplication Works Modal
modal-how-dedup-title = ℹ Funktionsweise der Deduplizierung
modal-how-dedup-desc1 = Anstatt die Bytes jeder Datei direkt miteinander vergleichen (was langsame, paarweise O(N²)-Scans erfordert), nutzt dieses System eine optimierte 7-stufige Pipeline zur sicheren und effizienten Identifizierung identischer Inhalte.
modal-how-dedup-pipeline-title = Die 7-stufige Pipeline:
modal-how-dedup-why-title = Warum reicht das aus?
modal-how-dedup-why-desc1 = Dank diesem mehrstufigen Filter wird sichergestellt, dass nur Dateien vollständig gelesen werden, welche überhaupt gleich sein können. Hierzu wird zunächst die Dateigröße verglichen, anschließend wird der Dateianfang und das Dateiende verglichen. Zum Schluss werden noch einige verteilte Stichproben aus den Dateien verglichen. Nur wenn dies alles identisch ist, wird zum Vergleichen der Dateien ein kryptografischer 256-Bit BLAKE3-Hash der gesamten Datei verwendet. Dieser Hash bietet ein vergleichbares Sicherheitsniveau zu gängigen sicheren Dateiübertragungsprotokollen. Auf einen paarweisen Binärvergleich von zwei Dateien wird daher verzichtet.

# How Deduplication Works Steps
modal-how-dedup-step1-title = 1. Aufteilung nach Größe
modal-how-dedup-step1-desc = Dateien werden nach ihrer genauen Größe gruppiert. Dateien mit einzigartiger Größe werden sofort aussortiert, um Festplattenzugriffe komplett zu vermeiden.
modal-how-dedup-step2-title = 2. Präfix-Hashing
modal-how-dedup-step2-desc = Die ersten 4KB der verbleibenden Kandidaten werden gehasht. Dadurch werden Dateien mit unterschiedlichen Headern oder Metadatenstrukturen schnell aussortiert.
modal-how-dedup-step3-title = 3. Mittelpunkt-Hashing
modal-how-dedup-step3-desc = Ein 4KB-Block aus der Mitte der verbleibenden Dateien wird gehasht, um interne strukturelle Unterschiede aufzudecken.
modal-how-dedup-step4-title = 4. Suffix-Hashing
modal-how-dedup-step4-desc = Die letzten 4KB werden gehasht. Dies ist hocheffektiv für Unterschiede im Endbereich oder bei abschließenden Metadaten.
modal-how-dedup-step5-title = 5. Mehrbereichs-Hashing
modal-how-dedup-step5-desc = Große Dateien (über 100MB) werden in regelmäßigen Abständen stichprobenartig über ihre gesamte Länge gehasht, um die Konsistenz zu prüfen, ohne die Datei komplett lesen zu müssen.
modal-how-dedup-step6-title = 6. Vollständiges BLAKE3-Hashing
modal-how-dedup-step6-desc = Für die verbleibenden Kandidaten wird ein vollständiger BLAKE3-Hash berechnet. Wegen der extrem großen Kollisionsresistenz dieses 256 Bit Hashes, bedeuten identische Hashes mit sehr hoher Wahrscheinlichkeit identische Inhalte. Dies erübrigt paarweise Binärvergleiche.
modal-how-dedup-step7-title = 7. Zeitstempel-Prüfung
modal-how-dedup-step7-desc = Unmittelbar vor der Anzeige oder Durchführung einer Deduplizierungsaktion prüft die Anwendung die Zeitstempel der Dateien, um sich gegen Änderungen abzusichern, die seit der Erstellung des Schnappschusses stattgefunden haben.

# Open Source Licenses Modal
modal-licenses-title = 📜 Quelloffene Lizenzen
modal-licenses-tab-app = eDirStat (MIT)
modal-licenses-tab-deps = Drittanbieter-Bibliotheken
modal-licenses-app-desc = eDirStat ist quelloffene Software, die unter der MIT-Lizenz vertrieben wird:
modal-licenses-desc = Folgende Drittanbieter-Bibliotheken werden in dieser Anwendung verwendet:
modal-licenses-copy-btn = 📋 Lizenz kopieren
modal-licenses-copy-all-btn = 📋 Lizenzen kopieren

# Processing Modal
modal-processing-title = ⏳ Verarbeitung...
modal-processing-deletion = Dateien und Verzeichnisse werden gelöscht...
modal-processing-trash = Dateien und Verzeichnisse werden in den Papierkorb verschoben...
modal-processing-hardlink = Duplikate werden durch Hardlinks ersetzt...
modal-processing-softlink = Duplikate werden durch Softlinks ersetzt...

# Explorer Column Headers
explorer-hdr-name = Name
explorer-hdr-percentage = Prozent
explorer-hdr-size = Größe
explorer-hdr-items = Elemente
explorer-hdr-files = Dateien
explorer-hdr-subdirs = Unterordner
explorer-hdr-created = Erstellt
explorer-hdr-modified = Geändert

# Update Checker
update-checking = Nach Aktualisierungen suchen...
update-available = Neue Version { $version } verfügbar!
update-up-to-date = Sie sind auf dem neuesten Stand
update-failed = Aktualisierungsprüfung fehlgeschlagen: { $error }

# Themes
theme = 🎨 Theme
theme-dark = Dunkel
theme-high-contrast = Hoher Kontrast
theme-light = Hell
theme-system = System

# New Scan Options Modal
modal-scan-options-title = Neue Scan-Optionen
modal-scan-options-header = Neuen Scan starten
modal-scan-options-path-label = Zu scannendes Verzeichnis:
modal-scan-options-paste-tooltip = Aus Zwischenablage einfügen
modal-scan-options-browse-tooltip = Ordner suchen...
modal-scan-options-scan-btn = Scannen
modal-scan-options-cancel-btn = Abbrechen
modal-scan-options-same-filesystem = Scan auf dasselbe Dateisystem/Volume beschränken
modal-scan-options-drives-header = 💽 Speicherlaufwerke & Volumes
modal-scan-options-refresh-tooltip = Speicherlaufwerke aktualisieren
modal-scan-options-root-system = Wurzelsystem
modal-scan-options-selected-badge = ✅ Ausgewählt
modal-scan-options-free-of = { $free } frei von { $total }
modal-scan-options-subtitle = Wählen Sie einen Datenträger, einen Schnellzugriff oder ein benutzerdefiniertes Verzeichnis zum Analysieren aus.
modal-scan-options-quick-access = 📍 Schnellzugriff
modal-scan-options-path-hint = /pfad/zum/scannen
modal-scan-options-hint = ℹ Wählen Sie oben ein Laufwerk aus oder geben Sie einen Verzeichnispfad ein.
modal-scan-options-sandbox-auth = 🔒 Sandbox-Zugriff erforderlich — Klicken Sie auf Scannen, um Zugriff zu gewähren
modal-scan-options-valid-dir = ✅ Gültiges Verzeichnis — Bereit zum Scannen
modal-scan-options-points-to-file = ⚠ Pfad verweist auf eine Datei — bitte wählen Sie einen Ordner aus.
modal-scan-options-dir-not-exist = ⚠ Verzeichnis existiert nicht im Dateisystem.
quick-loc-home = 🏠 Benutzer
quick-loc-documents = 📄 Dokumente
quick-loc-downloads = 📥 Downloads
quick-loc-desktop = 🖥 Schreibtisch
quick-loc-pictures = 🖼 Bilder
search-use-regex = Regulären Ausdruck verwenden (Regex)
search-match-case = Groß-/Kleinschreibung beachten
dedup-pref-dir-hint = z. B. /home/user/Archive

file-menu-close = Scan schließen
file-menu-quit = Beenden

badge-dataless-cloud = Cloud- / Dataless-Datei
badge-symlink = Symbolischer Link
badge-special-file = Spezialdatei (Pipe / Socket / Gerät)
badge-permission-denied = Zugriff verweigert

# Docker Disk Usage
vis-mode-docker = 📦 Docker
docker-desc = Speicherplatznutzung der lokalen Docker-Installation, direkt aus dem Datenverzeichnis auf der Festplatte gelesen — keine Daemon-Verbindung erforderlich.
docker-no-install = Auf diesem Rechner wurde keine Docker-Installation gefunden.
docker-env-roots-title = Datenverzeichnisse
docker-env-vmdisks-title = Virtuelle Maschinendatenträger
docker-env-socket = Daemon-Socket:
docker-env-roots-header = Datenstammverzeichnisse ({ $count })
docker-env-vmdisks-header = Virtuelle Maschinendisks ({ $count })
docker-kind-overlay2 = overlay2
docker-kind-containerd = containerd (Image-Speicher)
docker-kind-unknown = Unbekanntes Layout
docker-kind-overlay = overlay
docker-kind-aufs = aufs
docker-kind-btrfs = btrfs
docker-kind-zfs = zfs
docker-kind-devicemapper = devicemapper
docker-kind-vfs = vfs
docker-kind-windowsfilter = windowsfilter
docker-scope-system = systemweit
docker-scope-rootless = rootless
docker-readable = ✅ lesbar
docker-not-readable = ❌ nicht lesbar
docker-vm-size = { $apparent } logisch, { $allocated } auf der Festplatte belegt
docker-vm-note = 💽 Docker Desktop legt alle Daten innerhalb des obigen VM-Datenträgers ab; der Inhalt kann vom Host aus nicht zugeordnet werden.
docker-needs-elevated = ⚠ Ein Docker-Datenverzeichnis wurde gefunden, ist aber nicht lesbar. Starten Sie eDirStat mit erhöhten Rechten (sudo oder Administrator), um es zu analysieren.
docker-unsupported-layout = Dieser Docker-Datenstamm verwendet ein Speicherlayout, das eDirStat noch nicht analysieren kann (vorerst nur overlay2).
docker-analyze = ⚡ Analysieren
docker-analyzing = Docker-Datenverzeichnis wird analysiert...
docker-error = Docker-Inventur fehlgeschlagen: { $error }
docker-lbl-images = Image-Daten
docker-lbl-shared = Geteilte Layer
docker-lbl-unique = Einzigartige Image-Daten
docker-lbl-containers = Container-RW
docker-lbl-volumes = Volumes
docker-lbl-build-cache = Build Cache
docker-lbl-logs = Container-Protokolle
docker-lbl-reclaimable = Wiederfreigebbar
docker-section-images = Images
docker-section-containers = Container
docker-section-volumes = Volumes
docker-hdr-id = ID
docker-hdr-shared = Geteilt
docker-hdr-exclusive = Exklusiv
docker-hdr-layers = Layer
docker-hdr-rw = RW-Größe
docker-hdr-log = Protokollgröße
docker-no-entries = Keine Einträge.
docker-warnings-title = ⚠ Warnungen
docker-warn-large-log = Container '{ $container }' hat eine große Protokolldatei ({ $size }).
docker-warn-unref-layer = Layer { $cache_id } wird von keinem Image referenziert ({ $size } wiederfreigebbar).
docker-warn-root-unreadable = Datenverzeichnis { $path } ist nicht lesbar (erhöhte Rechte erforderlich).
docker-warn-metadata = Metadatenproblem: { $detail }
docker-warn-containerd-partial = Der containerd-Image-Store ist aktiv: Images und Layer können nicht von der Festplatte zugeordnet werden (Docker-API-Unterstützung ist geplant); diese Übersicht umfasst nur Container, Logs, Volumes und Build-Cache.
docker-source-disk = Festplattenscan
docker-source-partial = Festplattenscan (teilweise)
docker-source-api = Docker-API
docker-collected-at = Erstellt am { $time }
docker-refresh = 🔄 Aktualisieren
docker-no-snapshot-data = Dieser Snapshot enthält keine Docker-Daten.
docker-sandbox-unavailable = Die Live-Docker-Integration ist in der App-Store-Version von eDirStat nicht verfügbar, aber auf anderen Rechnern erstellte Snapshots können hier weiterhin angesehen werden.
docker-delete = Löschen
docker-delete-title = Docker-Image löschen
docker-delete-confirm = Das Docker-Image { $name } ({ $size }) über den Docker-Daemon löschen?
docker-delete-warning = Diese Aktion hebt die Markierung auf und löscht das Image über den Docker-Daemon. Der Vorgang kann nicht rückgängig gemacht werden; Schichten, die von anderen Images referenziert werden, bleiben erhalten.
docker-delete-force = Entfernung erzwingen
docker-delete-force-hover = Das Image entfernen, auch wenn Container es referenzieren (Force-Flag des Daemons).
docker-deleting = Image wird gelöscht...
docker-delete-done = Image { $name } gelöscht: { $untagged } Markierung(en) aufgehoben, { $deleted } Schicht(en) entfernt.
docker-delete-failed = Das Image konnte nicht gelöscht werden: { $error }
docker-delete-container = Container löschen
docker-delete-volume = Volume löschen
docker-delete-container-confirm = Den Container { $name } (beschreibbare Schicht { $size }, Protokoll { $extra }) über den Docker-Daemon löschen?
docker-delete-volume-confirm = Das Volume { $name } ({ $size }) über den Docker-Daemon löschen?
docker-delete-container-warning = Dies entfernt den Container dauerhaft über den Docker-Daemon. Der Vorgang kann nicht rückgängig gemacht werden.
docker-delete-volume-warning = Dies entfernt das Volume und seine Daten dauerhaft über den Docker-Daemon. Der Vorgang kann nicht rückgängig gemacht werden.
docker-delete-container-force-hover = Den Container entfernen, auch wenn er noch läuft (Force-Flag des Daemons).
docker-delete-volume-force-hover = Das Volume entfernen, auch wenn Container es verwenden (Force-Flag des Daemons).
docker-deleting-container = Container wird gelöscht...
docker-deleting-volume = Volume wird gelöscht...
docker-delete-container-done = Container { $name } gelöscht.
docker-delete-volume-done = Volume { $name } gelöscht.
docker-delete-container-failed = Der Container konnte nicht gelöscht werden: { $error }
docker-delete-volume-failed = Das Volume konnte nicht gelöscht werden: { $error }
docker-hdr-created = Erstellt
docker-hdr-refs = Verweise
docker-delete-title-multi = Docker-Images löschen
docker-delete-container-multi = Container löschen
docker-delete-volume-multi = Volumes löschen
docker-delete-confirm-multi = { $count } Images ({ $size } gesamt) über den Docker-Daemon löschen?
docker-delete-container-confirm-multi = { $count } Container (beschreibbare Schicht { $size }, Protokoll { $extra } gesamt) über den Docker-Daemon löschen?
docker-delete-volume-confirm-multi = { $count } Volumes ({ $size } gesamt) über den Docker-Daemon löschen?
docker-delete-more = ... und { $count } weitere
docker-delete-done-multi = { $count } Images gelöscht: { $untagged } Markierung(en) aufgehoben, { $deleted } Schicht(en) entfernt.
docker-delete-container-done-multi = { $count } Container gelöscht.
docker-delete-volume-done-multi = { $count } Volumes gelöscht.
docker-delete-partial-multi = { $succeeded } von { $count } gelöscht, { $failed } fehlgeschlagen. Erster Fehler: { $error }
badge-docker = Docker-Datenverzeichnis
badge-docker-vm = Docker-VM-Datenträger
badge-docker-area = Docker-{ $area }-Speicher
