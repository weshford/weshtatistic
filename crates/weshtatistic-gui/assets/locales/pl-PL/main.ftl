# Menu Bar Dropdowns
file = Plik
view = Widok
help = Pomoc

# Menu Bar Actions
new-scan = 📁 Nowy skan
save-snapshot = 💾 Zapisz migawkę
load-snapshot = 📖 Wczytaj migawkę

# Menu Bar Status
idle = Bezczynny

# View Menu Options
monospace-paths = 🅰 Ścieżki o stałej szerokości
highlight-duplicates = ✨ Wyróżnij duplikaty
treemap-borders = 🔳 Obramowania mapy drzewa
treemap-style =  Styl mapy drzewa
treemap-style-vertical = Gradient pionowy
treemap-style-offset-vertical = Przesunięty gradient pionowy
treemap-style-diagonal = Gradient ukośny
treemap-style-cushion = Cieniowanie poduszkowe
deletion-confirmation = 🗑 Potwierdzenie usuwania
trash-confirmation = ♻ Potwierdzenie przenoszenia do kosza
time-format = 🕒 Format czasu
language = 💬 Język
layout-mode = Tryb układu:
classic-layout = Klasyczny układ
windirstat-layout = Układ WinDirStat
vis-mode-treemap = 📊 Treemap
vis-mode-plots = 📈 Wykresy
select-plot-label = Wybierz wykres:
vis-mode-deduplicator = 👥 Wyszukiwanie duplikatów
search-filter-label = 🔍 Filtruj:

# Panel Toggles
toggle-left-panel = { $collapsed ->
    [true] ▶ Pokaż lewy panel (F9)
   *[false] ◀ Ukryj lewy panel (F9)
}

toggle-right-panel = { $collapsed ->
    [true] { $is_classic ->
        [true] ◀ Pokaż prawy panel (F11)
       *[false] ▶ Pokaż panel rozszerzeń (F11)
    }
   *[false] { $is_classic ->
        [true] ▶ Ukryj prawy panel (F11)
       *[false] ◀ Ukryj panel rozszerzeń (F11)
    }
}

collapse-all = ⏏ Zwiń wszystko
about = ℹ O programie weshtatistic
web-not-available = Funkcja niedostępna w wersji internetowej

# Status Indicators
scanning-disk = Skanowanie dysku...
scan-complete = Skanowanie ukończone
scan-cancelled = Skanowanie anulowane
path-label = Ścieżka: { $path }
worker-threads = ⚡ { $count } Wątki robocze
worker-threads-hover = Liczba równoległych rdzeni procesora (work-stealing) przypisanych do przeszukiwania katalogów.

# Stats Panel (Bottom)
directories-count = 📁 Katalogi: { $count }
files-count = 📄 Pliki: { $count }
total-size = 💾 Całkowity rozmiar: { $size }
elapsed-time = ⏱ Czas: { $time }
scan-speed = ⚡ Prędkość: { $speed }/s

# Selection Info
selection-path = Wybór: { $path }
selection-items = Wybór: { $count ->
    [one] 1 element
    [few] { $count } elementy
   *[other] { $count } elementów
}

# Plot Types
plot-size-distribution = 📊 Rozkład rozmiarów plików
plot-age-size = 🌌 Wiek plików vs. Rozmiar plików
plot-dir-composition = 🍰 Skład katalogów
plot-extension-boxplot = 📦 Rozmiary plików według rozszerzeń
plot-temporal-timeline = ⏱ Powiązane linie czasu
plot-deduplicator-waste = 👥 Przestrzeń marnowana przez duplikaty według rozszerzeń

# --- Deduplicator Strings ---
dedup-desc = Wyszukuj i bezpiecznie usuwaj identyczne pliki (bajt po bajcie) przy użyciu bezpiecznych kryptograficznie skrótów BLAKE3.
dedup-how-it-works = ℹ Jak to działa
dedup-min-size = Minimalny rozmiar pliku:
dedup-ignore-system = Ignoruj pliki systemowe
dedup-ignore-hidden = Ignoruj pliki ukryte
dedup-start-scan = ⚡ Uruchom skanowanie duplikatów
dedup-scan-first = Najpierw zeskanuj katalog.
dedup-cancelled-msg = Skanowanie zostało anulowane. Uruchom nowe skanowanie, aby znaleźć duplikaty.
dedup-analyzing = Analizowanie plików...
dedup-no-duplicates = Nie znaleziono żadnych duplikatów. Spróbuj zmniejszyć minimalny rozmiar pliku lub zeskanować inny folder.
no-permission = Brak uprawnień
hardlink-badge = Hardlink
dedup-select-items = 🎯 Wybierz elementy...
dedup-select-all-but-oldest = 🎯 Wszystkie oprócz najstarszego
dedup-select-all-but-newest = 🎯 Wszystkie oprócz najnowszego
dedup-select-all-but-shortest = 🎯 Wszystkie oprócz najkrótszej ścieżki
dedup-select-all-but-rootmost = 🎯 Wszystkie oprócz najbliższego głównego katalogu
dedup-select-all-but-longest = 🎯 Wszystkie oprócz najdłuższej ścieżki
dedup-pref-dir-pattern = Preferowany wzorzec katalogu:
dedup-select-all-but-pref = 🎯 Wszystkie oprócz preferowanego katalogu
dedup-clear-selection = ❌ Wyczyść wybór
dedup-link-menu = 🔗 Połącz... ({ $count } plików)
dedup-link-menu-disabled = 🔗 Połącz... (0 plików)
dedup-link-hardlinks = 🔗 Zastąp wybrane twardymi dowiązaniami (hardlinks)
dedup-link-softlinks = 🔗 Zastąp wybrane dowiązaniami symbolicznymi (softlinks)
dedup-remove-menu = 🗑 Usuń... ({ $count } plików, { $size })
dedup-remove-menu-disabled = 🗑 Usuń... (0 plików)
dedup-remove-trash = ♻ Przenieś wybrane do kosza
dedup-remove-delete = 🗑 Usuń wybrane trwale
dedup-warning-title = ⚠ OSTRZEŻENIE O UTRACIE DANYCH
dedup-warning-desc = { $count ->
    [one] Usuwanie wszystkich wersji 1 pliku
   *[other] Usuwanie wszystkich wersji { $count } plików
}
dedup-warning-no-original = Żadna oryginalna kopia nie pozostanie:
dedup-warning-details = Zaznaczono oryginał oraz wszystkie kopie duplikatów dla poniższych plików. Ich usunięcie doprowadzi do trwałej utraty danych:
dedup-cancel-hover = Kliknij, aby anulować skanowanie
scan-cancel-hover = Kliknij, aby anulować skanowanie
dedup-current-label = Bieżący
dedup-phase1-size = Faza 1/7: Grupowanie wszystkich plików według rozmiaru...
dedup-phase1-filter = Faza 1/7: Filtrowanie wykluczeń z kandydatów na duplikaty...
dedup-phase2-prefix = Faza 2/7: Haszowanie początków plików (pierwsze 4KB)...
dedup-phase3-midpoint = Faza 3/7: Haszowanie środków plików...
dedup-phase4-suffix = Faza 4/7: Haszowanie końców plików...
dedup-phase5-multirange = Faza 5/7: Haszowanie wielozakresowe dużych plików...
dedup-phase6-full = Faza 6/7: Pełne haszowanie BLAKE3 pozostałych kandydatów...
dedup-phase7-validation = Faza 7/7: Ostateczna weryfikacja znaczników czasu...
dedup-phase-finished = Ukończono w czasie: { $duration }! Znaleziono { $count } grup duplikatów. Potencjalne odzyskanie miejsca: { $space }
dedup-scan-cancelled-with-error = Skanowanie zostało anulowane: { $error }

# Deduplicator Table Headers
dedup-hdr-checkbox = [     ]
dedup-hdr-filename = Nazwa pliku
dedup-hdr-directory = Katalog nadrzędny
dedup-hdr-size = Rozmiar
dedup-hdr-reclaimable = Do odzyskania
dedup-hdr-created = Utworzony
dedup-hdr-modified = Zmodyfikowany
dedup-copies-selected = ({ $count ->
    [one] Zaznaczono 1 kopię
    [few] Zaznaczono { $count } kopie
   *[other] Zaznaczono { $count } kopii
})

# --- Explorer Details Panel ---
explorer-details-header = ℹ Szczegóły
explorer-deselect-hover = Odznacz elementy
explorer-deselect-single-hover = Odznacz element
explorer-selected-items-count = { $count ->
    [one] Zaznaczono 1 element
    [few] Zaznaczono { $count } elementy
   *[other] Zaznaczono { $count } elementów
}
explorer-total-size = Całkowity rozmiar: { $size }
explorer-files = Pliki: { $count }
explorer-directories = Katalogi: { $count }
explorer-actions-title = Akcje
explorer-actions-operations = Operacje:
explorer-action-refresh-hover = Odśwież wszystkie zaznaczone poddrzewa katalogów
explorer-grid-type = Typ:
explorer-grid-size = Rozmiar:
explorer-grid-bytes = Bajty:
explorer-grid-items = Elementy:
explorer-grid-files = Pliki:
explorer-grid-subdirs = Podkatalogi:
explorer-grid-user = Użytkownik:
explorer-grid-group = Grupa:
explorer-grid-permissions = Uprawnienia:
explorer-grid-path = Pełna ścieżka:

# Explorer Type Names
type-symlink = Dowiązanie symboliczne
type-directory = Katalog
type-file = Plik

# Explorer Actions
explorer-action-copy-path = 📋 Kopiuj ścieżkę
explorer-action-open-file = 📄 Otwórz plik
explorer-action-open-manager = 🗁 Otwórz menedżer plików
explorer-action-refresh-subtree = 🔄 Odśwież poddrzewo
explorer-action-move-trash = ♻ Przenieś do kosza
explorer-action-delete-permanently = 🗑 Usuń trwale
explorer-action-refresh-directory = 🔄 Odśwież katalog

# Explorer Empty State
explorer-empty-state = Kliknij 'Nowy skan', aby zbadać zużycie dysku.
choose-an-option = Wybierz opcję
web-viewer = Przeglądarka Web
load-demo = 👁 Załaduj przykładową migawkę demo
placeholder-treemap = Zeskanowany system plików zostanie tutaj przedstawiony w postaci mapy drzewa. (treemap).
placeholder-plots = Zeskanowany system plików zostanie tutaj przedstawiony na wykresie.

# Treemap Zoom & Navigation
zoom-up = ⏶ W górę
zoom-reset = ❌ Resetuj
zoom-to-dir = 🔍 Skup w Treemap
zoom-up-level = ⏶ Poziom w górę
zoom-empty-dir = Katalog jest pusty

# --- Extensions Panel ---
extensions-header = 📂 Rozszerzenia
extensions-empty = Nie zebrano jeszcze statystyk.
extensions-hover-files = Pliki: { $count }

# --- Operations (Context Actions) ---
op-up-one-level = Przejdź poziom wyżej
op-zoom-treemap = Skup w Treemap
op-refresh-entire-scan = Odśwież całe skanowanie
op-refresh-directory = Odśwież katalog
op-open-file = Otwórz plik
op-open-file-manager = Otwórz w menedżerze plików
op-open-terminal = Otwórz terminal tutaj
op-copy-path = Kopiuj ścieżkę
op-copy-name = Kopiuj nazwę
op-move-trash = Przenieś do kosza
op-permanently-delete = Usuń trwale

# Toast Notifications
toast-already-root = Jesteś już na najwyższym poziomie
toast-navigated-up = Przejście o poziom wyżej powiodło się
toast-zoomed-treemap = Treemap skupiony na katalogu
toast-refreshing-scan = Odświeżanie całego skanowania...
toast-refreshing-dir = Odświeżanie zaznaczonych katalogów...
toast-opened-file = Otwarto: { $path }
toast-failed-open-file = Nie udało się otworzyć pliku: { $error }
toast-opened-manager = Otwarto w menedżerze plików: { $path }
toast-failed-open-manager = Nie udało się otworzyć menedżera plików: { $error }
toast-opened-terminal = Otwarto terminal w: { $path }
toast-failed-open-terminal = Nie udało się otworzyć terminala: { $error }
toast-copied-paths = { $count ->
    [one] Skopiowano 1 ścieżkę do schowka
    [few] Skopiowano { $count } ścieżki do schowka
   *[other] Skopiowano { $count } ścieżek do schowka
}
toast-copied-names = { $count ->
    [one] Skopiowano 1 nazwę do schowka
    [few] Skopiowano { $count } nazwy do schowka
   *[other] Skopiowano { $count } nazw do schowka
}

# --- Modals ---
modal-remember-confirmation = Zapamiętaj potwierdzenie dla wszystkich przyszłych plików i katalogów
modal-process-multiple = Zamierzasz przetworzyć { $count } zduplikowanych plików/elementów:
modal-process-single = Zamierzasz przetworzyć następującą ścieżkę:
# Confirm Deletion/Trash/Link Modals
modal-delete-title = ⚠ OSTRZEŻENIE O TRWAŁYM USUWANIU
modal-delete-header = ⚠ Ostrzeżenie o trwałym usuwaniu!
modal-delete-info = Całkowity rozmiar: { $size }
modal-delete-warning = Jest to usuwanie rekurencyjne. Wszystkie pliki, foldery i podkatalogi pod wybranymi ścieżkami zostaną trwale usunięte i nie będzie można ich odzyskać (z pominięciem kosza).
modal-delete-checkbox = Rozumiem, że pliki zostaną trwale usunięte i nie będzie można ich odzyskać.
modal-delete-confirm = 🗑 Tak, usuń trwale

modal-trash-title = ♻ PRZENIEŚ DO KOSZA
modal-trash-header = ♻ Przenieś do kosza
modal-trash-info = Całkowity rozmiar: { $size }
modal-trash-warning = Spowoduje to przeniesienie wybranych ścieżek oraz ich zawartości do systemowego kosza, skąd mogą być później przywrócone lub trwale usunięte.
modal-trash-checkbox = Potwierdzam chęć przeniesienia tego elementu do kosza.
modal-trash-confirm = ♻ Tak, przenieś do kosza

modal-delete-duplicates-title = ⚠ OSTRZEŻENIE O TRWAŁYM USUWANIU DUPLIKATÓW
modal-delete-duplicates-header = ⚠ Ostrzeżenie o trwałym usuwaniu duplikatów!
modal-delete-duplicates-info = Całkowita przestrzeń do odzyskania: { $size }
modal-delete-duplicates-warning = Wszystkie wybrane pliki zostaną trwale usunięte i nie będzie można ich odzyskać (z pominięciem kosza).
modal-delete-duplicates-checkbox = Rozumiem, że pliki zostaną trwale usunięte i nie będzie można ich odzyskać.
modal-delete-duplicates-confirm = 🗑 Tak, usuń wybrane trwale

modal-trash-duplicates-title = ♻ PRZENIEŚ DUPLIKATY DO KOSZA
modal-trash-duplicates-header = ♻ Przenieś duplikaty do kosza
modal-trash-duplicates-info = Całkowita przestrzeń do odzyskania: { $size }
modal-trash-duplicates-warning = Wszystkie wybrane pliki zostaną przeniesione do systemowego kosza.
modal-trash-duplicates-checkbox = Potwierdzam chęć przeniesienia tych plików do kosza.
modal-trash-duplicates-confirm = ♻ Tak, przenieś wybrane do kosza

modal-hardlink-duplicates-title = 🔗 ZASTĄP DUPLIKATY TWARDYMI DOWIĄZANIAMI
modal-hardlink-duplicates-header = 🔗 Zastąp duplikaty twardymi dowiązaniami
modal-hardlink-duplicates-info = Liczba plików do przetworzenia: { $count }. Skumulowany rozmiar wirtualny: { $size }
modal-hardlink-duplicates-warning = Spowoduje to usunięcie zaznaczonych zduplikowanych plików i zastąpienie ich twardymi dowiązaniami na poziomie systemu plików, wskazującymi na pozostały oryginalny plik z każdej grupy. Pozwala to na wizualne zachowanie plików przy jednoczesnym zwolnieniu rzeczywistego fizycznego miejsca na dysku.
modal-hardlink-duplicates-checkbox = Potwierdzam chęć zastąpienia wybranych plików twardymi dowiązaniami.
modal-hardlink-duplicates-confirm = 🔗 Tak, zastąp twardymi dowiązaniami

modal-softlink-duplicates-title = 🔗 ZASTĄP DUPLIKATY DOWIĄZANIAMI SYMBOLICZNYMI
modal-softlink-duplicates-header = 🔗 Zastąp duplikaty dowiązaniami symbolicznymi
modal-softlink-duplicates-info = Liczba plików do przetworzenia: { $count }. Skumulowany rozmiar wirtualny: { $size }
modal-softlink-duplicates-warning = Spowoduje to usunięcie zaznaczonych zduplikowanych plików i zastąpienie ich dowiązaniami symbolicznymi (softlinks) na poziomie systemu plików, wskazującymi na pozostały oryginalny plik z każdej grupy. Pozwala to na wizualne zachowanie plików przy jednoczesnym zwolnieniu rzeczywistego fizycznego miejsca na dysku.
modal-softlink-duplicates-checkbox = Potwierdzam chęć zastąpienia wybranych plików dowiązaniami symbolicznymi.
modal-softlink-duplicates-confirm = 🔗 Tak, zastąp dowiązaniami symbolicznymi

# Path Does Not Exist Modal
modal-path-not-exist-title = ❌ Ścieżka nie istnieje!
modal-path-not-exist-msg = Błąd: Ścieżka, którą próbujesz usunąć, nie istnieje na dysku.
modal-close-btn = Zamknij
modal-details-label = Szczegóły: 
modal-cancel-btn = Anuluj

# Elevation Recommended Modal
modal-elevation-title = ⚠ Zalecane podniesienie uprawnień
modal-elevation-desc = Program weshtatistic jest domyślnie uruchamiany ze standardowymi uprawnieniami użytkownika. Jednak system Windows ściśle ogranicza bezpośredni dostęp do fizycznego uchwytu dysku dla kont administratora.
modal-elevation-mft-disabled = Sterownik NTFS MFT systemu Windows jest wyłączony
modal-elevation-mft-desc = Bez uprawnień administratora nie można zainicjować bezpośredniego skanera MFT. Analiza plików skorzysta z alternatywnego, standardowego przeszukiwania katalogów, co obniża wydajność skanowania nawet 20-krotnie.
modal-elevation-relaunch-prompt = Czy chcesz teraz uruchomić aplikację z uprawnieniami administratora?
modal-elevation-continue-std = Kontynuuj jako standardowy użytkownik
modal-elevation-relaunch-btn = 🛡 Uruchom jako administrator

# About Modal
modal-about-title = ℹ O programie weshtatistic
modal-about-author = Autor: weshford
modal-about-license-btn = 📜 Licencja (MIT)
modal-about-desc1 = Wydajne narzędzie do analizy przestrzeni dyskowej i deduplikacji napisane w języku Rust.
modal-about-desc2 = Posiada równoległe przeszukiwanie katalogów metodą work-stealing, kompresowane migawki z deserializacją układu bez konieczności parsowania oraz interaktywne i płynne mapy drzewa.
modal-about-desc3 = Zintegrowany moduł deduplikacji uruchamia wieloetapowy proces kryptograficznego haszowania w celu bezpiecznego wyodrębnienia grup duplikatów, obliczenia przestrzeni do odzyskania oraz uwzględnienia systemowych twardych dowiązań.
modal-about-licenses-btn = Wyświetl licencje Open Source
modal-about-version = v{ $version }

# How Deduplication Works Modal
modal-how-dedup-title = ℹ Jak działa deduplikacja
modal-how-dedup-desc1 = Zamiast bezpośredniego porównywania bajtów każdego pliku (co wymagałoby powolnego, parzystego skanowania O(N²)), system ten wykorzystuje zoptymalizowany, 7-etapowy proces do bezpiecznej i wydajnej identyfikacji identycznej zawartości.
modal-how-dedup-pipeline-title = 7-etapowy proces:
modal-how-dedup-why-title = Dlaczego to wystarcza?
modal-how-dedup-why-desc1 = Ten wieloetapowy filtr gwarantuje, że w całości zostaną odczytane tylko te pliki, które posiadają identyczny rozmiar, początek, środek, koniec i próbki bloków. Porównanie 256-bitowego skrótu kryptograficznego BLAKE3 zapewnia poziom bezpieczeństwa zgodny z branżowymi protokołami bezpiecznego transferu danych, eliminując potrzebę powolnego porównywania bajt po bajcie.

# How Deduplication Works Steps
modal-how-dedup-step1-title = 1. Podział według rozmiaru
modal-how-dedup-step1-desc = Pliki są grupowane według ich dokładnego rozmiaru w bajtach. Każdy plik o unikalnym rozmiarze jest natychmiast odrzucany, co całkowicie omija operacje wejścia/wyjścia na dysku.
modal-how-dedup-step2-title = 2. Haszowanie początku (prefix)
modal-how-dedup-step2-desc = Pierwsze 4KB pozostałych kandydatów jest haszowane. Pozwala to na szybkie odfiltrowanie plików o różnych nagłówkach lub formatach metadanych.
modal-how-dedup-step3-title = 3. Haszowanie środka (midpoint)
modal-how-dedup-step3-desc = Środkowy blok 4KB pozostałych plików jest haszowany, co ujawnia wewnętrzne różnice strukturalne.
modal-how-dedup-step4-title = 4. Haszowanie końca (suffix)
modal-how-dedup-step4-desc = Ostatnie 4KB danych jest haszowane. Jest to bardzo skuteczne przy identyfikowaniu różnic w końcowej zawartości pliku lub metadanych.
modal-how-dedup-step5-title = 5. Haszowanie wielozakresowe
modal-how-dedup-step5-desc = Duże pliki (powyżej 100MB) podlegają okresowemu próbkowaniu bloków na całej ich długości w celu sprawdzenia spójności zawartości bez odczytywania całego pliku.
modal-how-dedup-step6-title = 6. Pełny hasz BLAKE3
modal-how-dedup-step6-desc = Dla pozostałych kandydatów obliczany jest pełny hasz kryptograficzny BLAKE3. Ze względu na wysoką odporność na kolizje przestrzeni 256-bitowej, pasujące skróty wskazują na astronomiczne prawdopodobieństwo, że pliki są identyczne, stanowiąc wysoce wiarygodny dowód tożsamości bez konieczności porównań parzystych.
modal-how-dedup-step7-title = 7. Weryfikacja znaczników czasu
modal-how-dedup-step7-desc = Tuż przed wyświetleniem lub wykonaniem jakiejkolwiek akcji deduplikacji program weryfikuje znaczniki czasu plików na dysku, aby zabezpieczyć się przed zmianami, które zaszły od momentu wygenerowania migawki.

# Open Source Licenses Modal
modal-licenses-title = 📜 Licencje Open Source
modal-licenses-tab-app = weshtatistic (MIT)
modal-licenses-tab-deps = Biblioteki stron trzecich
modal-licenses-app-desc = weshtatistic to oprogramowanie open source dystrybuowane na licencji MIT:
modal-licenses-desc = W aplikacji używane są następujące biblioteki i pakiety (crates) stron trzecich:
modal-licenses-copy-btn = 📋 Kopiuj licencję
modal-licenses-copy-all-btn = 📋 Kopiuj licencje

# Processing Modal
modal-processing-title = ⏳ Przetwarzanie...
modal-processing-deletion = Usuwanie plików i katalogów...
modal-processing-trash = Przenoszenie plików i katalogów do kosza...
modal-processing-hardlink = Zastępowanie duplikatów twardymi dowiązaniami...
modal-processing-softlink = Zastępowanie duplikatów symbolicznymi dowiązaniami...

# Explorer Column Headers
explorer-hdr-name = Nazwa
explorer-hdr-percentage = Procent
explorer-hdr-size = Rozmiar
explorer-hdr-items = Elementy
explorer-hdr-files = Pliki
explorer-hdr-subdirs = Podkatal.
explorer-hdr-created = Utworzony
explorer-hdr-modified = Zmodyfikowany

# Update Checker
update-checking = Sprawdzanie aktualizacji...
update-available = Nowa wersja { $version } jest dostępna!
update-up-to-date = Masz najnowszą wersję
update-failed = Błąd sprawdzania aktualizacji: { $error }

# Themes
theme = 🎨 Motyw
theme-dark = Ciemny
theme-high-contrast = Wysoki kontrast
theme-light = Jasny
theme-system = Systemowy

# New Scan Options Modal
modal-scan-options-title = Opcje nowego skanowania
modal-scan-options-header = Uruchom nowe skanowanie
modal-scan-options-path-label = Ścieżka katalogu do skanowania:
modal-scan-options-paste-tooltip = Wklej ze schowka
modal-scan-options-browse-tooltip = Przeglądaj folder...
modal-scan-options-scan-btn = Skanuj
modal-scan-options-cancel-btn = Anuluj
modal-scan-options-same-filesystem = Ogranicz skanowanie do tego samego systemu plików/wolumenu
modal-scan-options-drives-header = 💽 Dyski i wolumeny pamięci
modal-scan-options-refresh-tooltip = Odśwież dyski pamięci
modal-scan-options-root-system = System główny
modal-scan-options-selected-badge = ✅ Wybrane
modal-scan-options-free-of = { $free } wolne z { $total }
modal-scan-options-subtitle = Wybierz wolumin pamięci, szybką lokalizację lub własny katalog do analizy.
modal-scan-options-quick-access = 📍 Skróty szybkiego dostępu
modal-scan-options-path-hint = /sciezka/do/skanowania
modal-scan-options-hint = ℹ Wybierz dysk powyżej lub wprowadź ścieżkę do katalogu.
modal-scan-options-sandbox-auth = 🔒 Wymagany dostęp do piaskownicy — kliknij Skanuj, aby przyznać dostęp
modal-scan-options-valid-dir = ✅ Prawidłowy katalog — gotowy do skanowania
modal-scan-options-points-to-file = ⚠ Ścieżka wskazuje na plik — wybierz folder.
modal-scan-options-dir-not-exist = ⚠ Katalog nie istnieje w systemie plików.
quick-loc-home = 🏠 Katalog domowy
quick-loc-documents = 📄 Dokumenty
quick-loc-downloads = 📥 Pobrane
quick-loc-desktop = 🖥 Pulpit
quick-loc-pictures = 🖼 Obrazy
search-use-regex = Użyj wyrażeń regularnych (Regex)
search-match-case = Uwzględniaj wielkość liter
dedup-pref-dir-hint = np. /home/user/Archive

file-menu-close = Zamknij skanowanie
file-menu-quit = Zakończ

badge-dataless-cloud = Plik w chmurze / bez danych lokalnych
badge-symlink = Dowiązanie symboliczne
badge-special-file = Plik specjalny (Potok / Gniazdo / Urządzenie)
badge-permission-denied = Odmowa dostępu

# Docker Disk Usage
vis-mode-docker = 📦 Docker
docker-desc = Wykorzystanie dysku przez lokalną instalację Dockera, odczytane bezpośrednio z katalogu danych na dysku — bez połączenia z demonem.
docker-no-install = Nie wykryto instalacji Dockera na tej maszynie.
docker-env-roots-title = Katalogi danych
docker-env-vmdisks-title = Dyski maszyny wirtualnej
docker-env-socket = Gniazdo demona:
docker-env-roots-header = Korzenie danych ({ $count })
docker-env-vmdisks-header = Dyski maszyn wirtualnych ({ $count })
docker-kind-overlay2 = overlay2
docker-kind-containerd = containerd (magazyn obrazów)
docker-kind-unknown = nieznany układ
docker-kind-overlay = overlay
docker-kind-aufs = aufs
docker-kind-btrfs = btrfs
docker-kind-zfs = zfs
docker-kind-devicemapper = devicemapper
docker-kind-vfs = vfs
docker-kind-windowsfilter = windowsfilter
docker-scope-system = systemowy
docker-scope-rootless = rootless
docker-readable = ✅ odczytywalny
docker-not-readable = ❌ nieodczytywalny
docker-vm-size = { $apparent } pozornie, { $allocated } przydzielone na dysku
docker-vm-note = 💽 Docker Desktop przechowuje wszystkie dane na powyższym dysku VM; zawartości nie można przypisać z hosta.
docker-needs-elevated = ⚠ Znaleziono katalog danych Dockera, ale nie jest on odczytywalny. Uruchom weshtatistic z podwyższonymi uprawnieniami (sudo lub administrator), aby go przeanalizować.
docker-unsupported-layout = Ten katalog danych Dockera używa układu pamięci, którego weshtatistic nie może jeszcze analizować (na razie tylko overlay2).
docker-analyze = ⚡ Analizuj
docker-analyzing = Analizowanie katalogu danych Dockera...
docker-error = Inwentaryzacja Dockera nie powiodła się: { $error }
docker-lbl-images = Dane obrazów
docker-lbl-shared = Współdzielone warstwy
docker-lbl-unique = Unikalne dane obrazów
docker-lbl-containers = RW kontenerów
docker-lbl-volumes = Volumes
docker-lbl-build-cache = Build Cache
docker-lbl-logs = Dzienniki kontenerów
docker-lbl-reclaimable = Do odzyskania
docker-section-images = Obrazy
docker-section-containers = Kontenery
docker-section-volumes = Wolumeny
docker-hdr-id = ID
docker-hdr-shared = Współdzielone
docker-hdr-exclusive = Wyłączne
docker-hdr-layers = Warstwy
docker-hdr-rw = Rozmiar RW
docker-hdr-log = Rozmiar dziennika
docker-no-entries = Brak wpisów.
docker-warnings-title = ⚠ Ostrzeżenia
docker-warn-large-log = Kontener '{ $container }' ma duży plik dziennika ({ $size }).
docker-warn-unref-layer = Warstwa { $cache_id } nie jest używana przez żaden obraz ({ $size } do odzyskania).
docker-warn-root-unreadable = Katalog danych { $path } jest nieodczytywalny (wymagane uprawnienia administratora).
docker-warn-metadata = Problem z metadanymi: { $detail }
docker-warn-containerd-partial = Używany jest magazyn obrazów containerd: obrazów i warstw nie można przypisać z dysku (planowana obsługa API Dockera); ten przegląd obejmuje tylko kontenery, dzienniki, wolumeny i pamięć podręczną kompilacji.
docker-source-disk = skan dysku
docker-source-partial = skan dysku (częściowy)
docker-source-api = API Dockera
docker-collected-at = Zebrano o { $time }
docker-refresh = 🔄 Odśwież
docker-no-snapshot-data = Ta migawka nie zawiera danych Dockera.
docker-sandbox-unavailable = Integracja Docker na żywo nie jest dostępna w wersji weshtatistic z App Store, ale migawki zebrane na innych komputerach można nadal tutaj przeglądać.
docker-delete = Usuń
docker-delete-title = Usuń obraz Dockera
docker-delete-confirm = Usunąć obraz Dockera { $name } ({ $size }) przez demona Dockera?
docker-delete-warning = Ta operacja zdejmuje tagi i usuwa obraz przez demona Dockera. Nie można jej cofnąć; warstwy wciąż używane przez inne obrazy zostaną zachowane.
docker-delete-force = Wymuś usunięcie
docker-delete-force-hover = Usuń obraz, nawet jeśli odwołują się do niego kontenery (flaga force demona).
docker-deleting = Usuwanie obrazu...
docker-delete-done = Obraz { $name } usunięty: usunięto { $untagged } tagów, usunięto { $deleted } warstw.
docker-delete-failed = Nie udało się usunąć obrazu: { $error }
docker-delete-container = Usuń kontener
docker-delete-volume = Usuń wolumen
docker-delete-container-confirm = Usunąć kontener { $name } (warstwa zapisu { $size }, dziennik { $extra }) przez demona Dockera?
docker-delete-volume-confirm = Usunąć wolumen { $name } ({ $size }) przez demona Dockera?
docker-delete-container-warning = Ta operacja trwale usuwa kontener przez demona Dockera. Nie można jej cofnąć.
docker-delete-volume-warning = Ta operacja trwale usuwa wolumen i jego dane przez demona Dockera. Nie można jej cofnąć.
docker-delete-container-force-hover = Usuń kontener, nawet jeśli jest uruchomiony (flaga force demona).
docker-delete-volume-force-hover = Usuń wolumen, nawet jeśli używają go kontenery (flaga force demona).
docker-deleting-container = Usuwanie kontenera...
docker-deleting-volume = Usuwanie wolumenu...
docker-delete-container-done = Kontener { $name } usunięty.
docker-delete-volume-done = Wolumen { $name } usunięty.
docker-delete-container-failed = Nie udało się usunąć kontenera: { $error }
docker-delete-volume-failed = Nie udało się usunąć wolumenu: { $error }
docker-hdr-created = Utworzono
docker-hdr-refs = Odnośniki
docker-delete-title-multi = Usuń obrazy Dockera
docker-delete-container-multi = Usuń kontenery
docker-delete-volume-multi = Usuń wolumeny
docker-delete-confirm-multi = Usunąć { $count } obrazów ({ $size } łącznie) przez demona Dockera?
docker-delete-container-confirm-multi = Usunąć { $count } kontenerów (warstwa zapisu { $size }, dziennik { $extra } łącznie) przez demona Dockera?
docker-delete-volume-confirm-multi = Usunąć { $count } wolumenów ({ $size } łącznie) przez demona Dockera?
docker-delete-more = ... i { $count } kolejnych
docker-delete-done-multi = Usunięto { $count } obrazów: usunięto { $untagged } tagów, usunięto { $deleted } warstw.
docker-delete-container-done-multi = Usunięto { $count } kontenerów.
docker-delete-volume-done-multi = Usunięto { $count } wolumenów.
docker-delete-partial-multi = Usunięto { $succeeded } z { $count }, { $failed } nie powiodło się. Pierwszy błąd: { $error }
badge-docker = Katalog danych Dockera
badge-docker-vm = Wirtualny dysk Dockera
badge-docker-area = Magazyn Dockera: { $area }

# Cleanup
vis-mode-cleanup = 🗑 Cleanup
cleanup-desc = Well-known reclaimable directories found in this scan: build artifacts and caches you can move to trash, plus system caches best cleaned by their own tools.
cleanup-summary = { $count } locations can free up { $size }
cleanup-regenerable-total = { $size } directly reclaimable
cleanup-tool-total = { $size } best cleaned by system tools
cleanup-tier-regenerable = Regenerable
cleanup-tier-tool = System tool
cleanup-empty = No well-known cleanup targets in this scan.
cleanup-analyzing = Analyzing scan for cleanup targets...
cleanup-run-command = Run in a terminal:
cleanup-copy = Copy
cleanup-copied = Command copied to clipboard.
cleanup-age-days = { $days } days
cleanup-age-fresh = Recent
cleanup-toast = 🗑 { $size } of reclaimable space found — open the Cleanup tab to review.
cleanup-hdr-tier = Tier
cleanup-hdr-title = What
cleanup-hdr-path = Location
cleanup-hdr-age = Age
cleanup-hdr-action = Action
