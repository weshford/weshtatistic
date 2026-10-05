# Menu Bar Dropdowns
file = Archivo
view = Ver
help = Ayuda

# Menu Bar Actions
new-scan = 📁 Nuevo análisis
save-snapshot = 💾 Guardar instantánea
load-snapshot = 📖 Cargar instantánea

# Menu Bar Status
idle = Inactivo

# View Menu Options
monospace-paths = 🅰 Rutas monoespaciadas
highlight-duplicates = ✨ Resaltar duplicados
treemap-borders = 🔳 Bordes del mapa de árbol
treemap-style =  Estilo del mapa de árbol
treemap-style-vertical = Gradiente vertical
treemap-style-offset-vertical = Gradiente vertical desplazado
treemap-style-diagonal = Gradiente diagonal
treemap-style-cushion = Sombreado de almohadilla
deletion-confirmation = 🗑 Confirmación de eliminación
trash-confirmation = ♻ Confirmación de papelera
time-format = 🕒 Formato de hora
language = 💬 Idioma
layout-mode = Modo de diseño:
classic-layout = Diseño clásico
windirstat-layout = Diseño de WinDirStat
vis-mode-treemap = 📊 Mapa de árbol
vis-mode-plots = 📈 Gráficos
select-plot-label = Seleccionar gráfico:
vis-mode-deduplicator = 👥 Buscador de archivos duplicados
search-filter-label = 🔍 Filtrar:

# Panel Toggles
toggle-left-panel = { $collapsed ->
    [true] ▶ Mostrar panel izquierdo (F9)
   *[false] ◀ Ocultar panel izquierdo (F9)
}

toggle-right-panel = { $collapsed ->
    [true] { $is_classic ->
        [true] ◀ Mostrar panel derecho (F11)
       *[false] ▶ Mostrar panel de extensiones (F11)
    }
   *[false] { $is_classic ->
        [true] ▶ Ocultar panel derecho (F11)
       *[false] ◀ Ocultar panel de extensiones (F11)
    }
}

collapse-all = ⏏ Contraer todo
about = ℹ Acerca de
web-not-available = Función no disponible en la versión web

# Status Indicators
scanning-disk = Analizando disco...
scan-complete = Análisis completado
scan-cancelled = Escaneo cancelado
path-label = Ruta: { $path }
worker-threads = ⚡ { $count } Hilos de trabajo
worker-threads-hover = El número de núcleos de CPU paralelos con robo de trabajo (work-stealing) asignados para el recorrido de directorios.

# Stats Panel (Bottom)
directories-count = 📁 Directorios: { $count }
files-count = 📄 Archivos: { $count }
total-size = 💾 Tamaño total: { $size }
elapsed-time = ⏱ Tiempo: { $time }
scan-speed = ⚡ Velocidad: { $speed }/s

# Selection Info
selection-path = Selección: { $path }
selection-items = Selección: { $count ->
    [one] 1 elemento
   *[other] { $count } elementos
}

# Plot Types
plot-size-distribution = 📊 Distribución de tamaño de archivo
plot-age-size = 🌌 Antigüedad de archivo vs. Tamaño de archivo
plot-dir-composition = 🍰 Composición de directorios
plot-extension-boxplot = 📦 Tamaños de archivo por extensión
plot-temporal-timeline = ⏱ Líneas de tiempo temporales vinculadas
plot-deduplicator-waste = 👥 Espacio duplicado desperdiciado por extensión

# --- Deduplicator Strings ---
dedup-desc = Busque y elimine de forma segura archivos idénticos byte por byte mediante hashes BLAKE3 criptográficamente seguros.
dedup-how-it-works = ℹ Cómo funciona
dedup-min-size = Tamaño mín. de archivo:
dedup-ignore-system = Ignorar archivos del sistema
dedup-ignore-hidden = Ignorar archivos ocultos
dedup-start-scan = ⚡ Iniciar análisis de duplicados
dedup-scan-first = Analice primero un directorio.
dedup-cancelled-msg = El análisis fue cancelado. Inicie un nuevo análisis para buscar duplicados.
dedup-analyzing = Analizando archivos...
dedup-no-duplicates = No se encontraron grupos de duplicados. Intente reducir el tamaño mínimo de archivo o analizar otra carpeta.
no-permission = Sin permiso
hardlink-badge = Enlace físico
dedup-select-items = 🎯 Seleccionar elementos...
dedup-select-all-but-oldest = 🎯 Todos excepto el más antiguo
dedup-select-all-but-newest = 🎯 Todos excepto el más reciente
dedup-select-all-but-shortest = 🎯 Todos excepto la ruta más corta
dedup-select-all-but-rootmost = 🎯 Todos excepto el más cercano a la raíz
dedup-select-all-but-longest = 🎯 Todos excepto la ruta más larga
dedup-pref-dir-pattern = Patrón de directorio preferido:
dedup-select-all-but-pref = 🎯 Todos excepto el directorio preferido
dedup-clear-selection = ❌ Limpiar selección
dedup-link-menu = 🔗 Enlazar... ({ $count } archivos)
dedup-link-menu-disabled = 🔗 Enlazar... (0 archivos)
dedup-link-hardlinks = 🔗 Reemplazar seleccionados por enlaces físicos
dedup-link-softlinks = 🔗 Reemplazar seleccionados por enlaces simbólicos
dedup-remove-menu = 🗑 Eliminar... ({ $count } archivos, { $size })
dedup-remove-menu-disabled = 🗑 Eliminar... (0 archivos)
dedup-remove-trash = ♻ Mover seleccionados a la papelera
dedup-remove-delete = 🗑 Eliminar seleccionados permanentemente
dedup-warning-title = ⚠ ADVERTENCIA DE PÉRDIDA DE DATOS
dedup-warning-desc = { $count ->
    [one] Eliminando todas las versiones de 1 archivo
   *[other] Eliminando todas las versiones de { $count } archivos
}
dedup-warning-no-original = No quedará ninguna copia original:
dedup-warning-details = Ha marcado tanto el archivo original como todas las copias duplicadas de los archivos enumerados a continuación. Su eliminación provocará probablemente una pérdida permanente de datos:
dedup-cancel-hover = Haga clic para cancelar el análisis
scan-cancel-hover = Haga clic para cancelar el escaneo
dedup-current-label = Actual
dedup-phase1-size = Fase 1/7: Agrupando todos los archivos analizados por tamaño...
dedup-phase1-filter = Fase 1/7: Filtrando exclusiones en candidatos duplicados...
dedup-phase2-prefix = Fase 2/7: Calculando el hash de los prefijos de archivo (primeros 4KB)...
dedup-phase3-midpoint = Fase 3/7: Calculando el hash del punto medio de los archivos...
dedup-phase4-suffix = Fase 4/7: Calculando el hash de los sufijos de archivo...
dedup-phase5-multirange = Fase 5/7: Calculando el hash de rango múltiple para archivos grandes...
dedup-phase6-full = Fase 6/7: Calculando el hash BLAKE3 completo de los candidatos restantes...
dedup-phase7-validation = Fase 7/7: Validación final de la marca de tiempo...
dedup-phase-finished = ¡Finalizado en { $duration }! Se encontraron { $count } grupos de duplicados. Espacio potencial a recuperar: { $space }
dedup-scan-cancelled-with-error = El análisis fue cancelado: { $error }

# Deduplicator Table Headers
dedup-hdr-checkbox = [     ]
dedup-hdr-filename = Nombre de archivo
dedup-hdr-directory = Directorio padre
dedup-hdr-size = Tamaño
dedup-hdr-reclaimable = Recuperable
dedup-hdr-created = Creado
dedup-hdr-modified = Modificado
dedup-copies-selected = ({ $count ->
    [one] 1 copia seleccionada
   *[other] { $count } copias seleccionadas
})

# --- Explorer Details Panel ---
explorer-details-header = ℹ Detalles
explorer-deselect-hover = Deseleccionar elementos
explorer-deselect-single-hover = Deseleccionar elemento
explorer-selected-items-count = { $count ->
    [one] 1 elemento seleccionado
   *[other] { $count } elementos seleccionados
}
explorer-total-size = Tamaño total: { $size }
explorer-files = Archivos: { $count }
explorer-directories = Directorios: { $count }
explorer-actions-title = Acciones
explorer-actions-operations = Operaciones:
explorer-action-refresh-hover = Actualizar todos los subárboles de directorios seleccionados
explorer-grid-type = Tipo:
explorer-grid-size = Tamaño:
explorer-grid-bytes = Bytes:
explorer-grid-items = Elementos:
explorer-grid-files = Archivos:
explorer-grid-subdirs = Subdirectorios:
explorer-grid-user = Usuario:
explorer-grid-group = Grupo:
explorer-grid-permissions = Permisos:
explorer-grid-path = Ruta completa:

# Explorer Type Names
type-symlink = Enlace simbólico
type-directory = Directorio
type-file = Archivo

# Explorer Actions
explorer-action-copy-path = 📋 Copiar ruta
explorer-action-open-file = 📄 Abrir archivo
explorer-action-open-manager = 🗁 Abrir gestor
explorer-action-refresh-subtree = 🔄 Actualizar subárbol
explorer-action-move-trash = ♻ Mover a la papelera
explorer-action-delete-permanently = 🗑 Eliminar permanentemente
explorer-action-refresh-directory = 🔄 Actualizar directorio

# Explorer Empty State
explorer-empty-state = Haga clic en «Nuevo análisis» para explorar el uso del disco.
choose-an-option = Elija una opción
web-viewer = Visor Web
load-demo = 👁 Cargar instantánea de demostración
placeholder-treemap = El sistema de archivos analizado se mostrará aquí como un mapa de árbol.
placeholder-plots = El sistema de archivos analizado se graficará aquí.

# Treemap Zoom & Navigation
zoom-up = ⏶ Subir
zoom-reset = ❌ Restablecer
zoom-to-dir = 🔍 Enfocar en Treemap
zoom-up-level = ⏶ Subir un nivel
zoom-empty-dir = El directorio está vacío

# --- Extensions Panel ---
extensions-header = 📂 Extensiones
extensions-empty = Aún no se han recopilado estadísticas.
extensions-hover-files = Archivos: { $count }

# --- Operations (Context Actions) ---
op-up-one-level = Subir un nivel
op-zoom-treemap = Enfocar en Treemap
op-refresh-entire-scan = Actualizar análisis completo
op-refresh-directory = Actualizar directorio
op-open-file = Abrir archivo
op-open-file-manager = Abrir en el gestor de archivos
op-open-terminal = Abrir terminal aquí
op-copy-path = Copiar ruta
op-copy-name = Copiar nombre
op-move-trash = Mover a la papelera
op-permanently-delete = Eliminar permanentemente

# Toast Notifications
toast-already-root = Ya se encuentra en el nivel raíz
toast-navigated-up = Se ha subido un nivel
toast-zoomed-treemap = Treemap enfocado en el directorio
toast-refreshing-scan = Actualizando análisis completo...
toast-refreshing-dir = Actualizando directorios seleccionados...
toast-opened-file = Abierto: { $path }
toast-failed-open-file = No se pudo abrir el archivo: { $error }
toast-opened-manager = Abierto en el gestor de archivos: { $path }
toast-failed-open-manager = No se pudo abrir en el gestor de archivos: { $error }
toast-opened-terminal = Terminal abierta en: { $path }
toast-failed-open-terminal = No se pudo abrir la terminal: { $error }
toast-copied-paths = { $count ->
    [one] Se ha copiado 1 ruta al portapapeles
   *[other] Se han copiado { $count } rutas al portapapeles
}
toast-copied-names = { $count ->
    [one] Se ha copiado 1 nombre al portapapeles
   *[other] Se han copiado { $count } nombres al portapapeles
}

# --- Modals ---
modal-remember-confirmation = Recordar confirmación para todos los archivos y directorios futuros
modal-process-multiple = Está a punto de procesar { $count } archivos/elementos duplicados:
modal-process-single = Está a punto de procesar la siguiente ruta:
# Confirm Deletion/Trash/Link Modals
modal-delete-title = ⚠ ADVERTENCIA DE ELIMINACIÓN PERMANENTE
modal-delete-header = ⚠ ¡Advertencia de eliminación permanente!
modal-delete-info = Tamaño total: { $size }
modal-delete-warning = Esta es una eliminación recursiva. Todos los archivos, carpetas y subdirectorios bajo las rutas seleccionadas se eliminarán permanentemente y no se podrán recuperar (omitiendo la papelera de reciclaje).
modal-delete-checkbox = Entiendo que los archivos se eliminarán permanentemente y no se podrán recuperar.
modal-delete-confirm = 🗑 Sí, eliminar permanentemente

modal-trash-title = ♻ MOVER A LA PAPELERA
modal-trash-header = ♻ Mover a la papelera
modal-trash-info = Tamaño total: { $size }
modal-trash-warning = Esto moverá las rutas seleccionadas y todo su contenido a la papelera de reciclaje del sistema, de donde podrán ser recuperados o eliminados permanentemente más tarde.
modal-trash-checkbox = Confirmo que deseo mover esto a la papelera.
modal-trash-confirm = ♻ Sí, mover a la papelera

modal-delete-duplicates-title = ⚠ ADVERTENCIA DE DEDUPLICACIÓN PERMANENTE
modal-delete-duplicates-header = ⚠ ¡Advertencia de eliminación permanente de duplicados!
modal-delete-duplicates-info = Espacio total a recuperar: { $size }
modal-delete-duplicates-warning = Todos los archivos seleccionados se eliminarán permanentemente y no se podrán recuperar (omitiendo la papelera de reciclaje).
modal-delete-duplicates-checkbox = Entiendo que los archivos se eliminarán permanentemente y no se podrán recuperar.
modal-delete-duplicates-confirm = 🗑 Sí, eliminar seleccionados permanentemente

modal-trash-duplicates-title = ♻ MOVER DUPLICADOS A LA PAPELERA
modal-trash-duplicates-header = ♻ Mover duplicados a la papelera
modal-trash-duplicates-info = Espacio total a recuperar: { $size }
modal-trash-duplicates-warning = Todos los archivos seleccionados se moverán a la papelera de reciclaje.
modal-trash-duplicates-checkbox = Confirmo que deseo mover estos archivos a la papelera.
modal-trash-duplicates-confirm = ♻ Sí, mover seleccionados a la papelera

modal-hardlink-duplicates-title = 🔗 REEMPLAZAR DUPLICADOS CON ENLACES FÍSICOS
modal-hardlink-duplicates-header = 🔗 Reemplazar duplicados con enlaces físicos
modal-hardlink-duplicates-info = Total de archivos a procesar: { $count }. Tamaño virtual acumulado: { $size }
modal-hardlink-duplicates-warning = Esto eliminará los archivos duplicados seleccionados y los reemplazará con enlaces físicos a nivel de sistema de archivos que apuntan al archivo original restante en cada grupo. Esto conserva los archivos visualmente mientras libera almacenamiento físico real.
modal-hardlink-duplicates-checkbox = Confirmo que deseo reemplazar los archivos seleccionados con enlaces físicos.
modal-hardlink-duplicates-confirm = 🔗 Sí, reemplazar con enlaces físicos

modal-softlink-duplicates-title = 🔗 REPLACE DUPLICATES WITH SOFTLINKS
modal-softlink-duplicates-header = 🔗 Reemplazar duplicados con enlaces simbólicos
modal-softlink-duplicates-info = Total de archivos a procesar: { $count }. Tamaño virtual acumulado: { $size }
modal-softlink-duplicates-warning = Esto eliminará los archivos duplicados seleccionados y los reemplazará con enlaces simbólicos a nivel de sistema de archivos que apuntan al archivo original restante en cada grupo. Esto conserva los archivos visualmente mientras libera almacenamiento físico real.
modal-softlink-duplicates-checkbox = Confirmo que deseo reemplazar los archivos seleccionados con enlaces simbólicos.
modal-softlink-duplicates-confirm = 🔗 Sí, reemplazar con enlaces simbólicos

# Path Does Not Exist Modal
modal-path-not-exist-title = ❌ ¡La ruta no existe!
modal-path-not-exist-msg = Error: La ruta que intenta eliminar no existe en el disco.
modal-close-btn = Cerrar
modal-details-label = Detalles: 
modal-cancel-btn = Cancelar

# Elevation Recommended Modal
modal-elevation-title = ⚠ Elevación recomendada
modal-elevation-desc = eDirStat se ejecuta con privilegios de usuario estándar de forma predeterminada. Sin embargo, Windows restringe estrictamente el acceso directo al identificador físico del disco a las cuentas de administrador.
modal-elevation-mft-disabled = Controlador NTFS MFT de Windows desactivado
modal-elevation-mft-desc = Sin privilegios administrativos, el analizador directo de la MFT del disco no se puede inicializar. El análisis de archivos utilizará el controlador de recorrido estándar alternativo, lo que reducirá el rendimiento del análisis hasta 20 veces.
modal-elevation-relaunch-prompt = ¿Desea reiniciar la aplicación con privilegios de administrador ahora?
modal-elevation-continue-std = Continuar como usuario estándar
modal-elevation-relaunch-btn = 🛡 Reiniciar como administrador

# About Modal
modal-about-title = ℹ Acerca de eDirStat
modal-about-author = Por: Cody Wyatt Neiman (xangelix) <neiman@cody.to>
modal-about-license-btn = 📜 Licencia (MIT)
modal-about-desc1 = Una herramienta de análisis de espacio en disco y deduplicación de alto rendimiento escrita en Rust.
modal-about-desc2 = Incluye recorrido de directorios paralelo con robo de trabajo (work-stealing), capturas comprimidas con deserialización de diseño de copia cero (zero-parsing) e imágenes de mapa de árbol (treemaps) interactivas.
modal-about-desc3 = El deduplicador integrado ejecuta una canalización de hash criptográfico de varias etapas para aislar de forma segura los grupos de duplicados, calcular el espacio recuperable y respetar los enlaces físicos a nivel de sistema.
modal-about-licenses-btn = Ver licencias de código abierto
modal-about-version = v{ $version }

# How Deduplication Works Modal
modal-how-dedup-title = ℹ Cómo funciona la deduplicación
modal-how-dedup-desc1 = En lugar de comparar directamente los bytes de cada archivo (lo que requiere exploraciones lentas O(N²) en parejas), este sistema utiliza una canalización optimizada de 7 etapas para identificar contenido idéntico de manera segura y eficiente.
modal-how-dedup-pipeline-title = La canalización de 7 etapas:
modal-how-dedup-why-title = ¿Por qué es suficiente?
modal-how-dedup-why-desc1 = Este filtro de varias etapas garantiza que solo se lean en su totalidad los archivos con idéntico tamaño, prefijo, punto medio, sufijo y muestras de bloques distribuidos. Por último, comparar un hash criptográfico BLAKE3 de 256 bits ofrece un perfil de seguridad al nivel de los protocolos de transferencia seguros de la industria, eliminando la necesidad de comparaciones lentas byte a byte.

# How Deduplication Works Steps
modal-how-dedup-step1-title = 1. Particionado por tamaño
modal-how-dedup-step1-desc = Los archivos se agrupan por su tamaño exacto en bytes. Cualquier archivo con un tamaño único se descarta de inmediato, evitando por completo las operaciones de E/S del disco.
modal-how-dedup-step2-title = 2. Hash de prefijo
modal-how-dedup-step2-desc = Se calcula el hash de los primeros 4KB de los candidatos restantes. Esto descarta rápidamente archivos con diferentes cabeceras o formatos de metadatos.
modal-how-dedup-step3-title = 3. Hash del punto medio
modal-how-dedup-step3-desc = Se calcula el hash de un bloque de 4KB en el centro de los archivos restantes, detectando diferencias estructurales internas.
modal-how-dedup-step4-title = 4. Hash de sufijo
modal-how-dedup-step4-desc = Se calcula el hash de los últimos 4KB de datos. Esto es muy eficaz para identificar diferencias en los contenidos finales o metadatos.
modal-how-dedup-step5-title = 5. Hash de rango múltiple
modal-how-dedup-step5-desc = Los archivos grandes (más de 100MB) se someten a un muestreo periódico de bloques a lo largo de toda su longitud para verificar la coherencia del contenido sin leer todo el archivo.
modal-how-dedup-step6-title = 6. Hash BLAKE3 completo
modal-how-dedup-step6-desc = Para los candidatos restantes, se calcula un hash criptográfico BLAKE3 completo. Debido a la alta resistencia a colisiones de un espacio de 256 bits, la coincidencia de hashes indica una improbabilidad astronómica de que los archivos difieran, lo que proporciona una prueba de identidad muy fiable sin necesidad de comparaciones por pares.
modal-how-dedup-step7-title = 7. Validación de marca de tiempo
modal-how-dedup-step7-desc = Justo antes de mostrar o ejecutar cualquier acción de deduplicación, la aplicación verifica las marcas de tiempo de los archivos en el disco para protegerse contra los cambios ocurridos desde la generación de la instantánea.

# Open Source Licenses Modal
modal-licenses-title = 📜 Licencias de código abierto
modal-licenses-tab-app = eDirStat (MIT)
modal-licenses-tab-deps = Bibliotecas de terceros
modal-licenses-app-desc = eDirStat es software de código abierto distribuido bajo la Licencia MIT:
modal-licenses-desc = En esta aplicación se utilizan las siguientes bibliotecas y crates de terceros:
modal-licenses-copy-btn = 📋 Copiar licencia
modal-licenses-copy-all-btn = 📋 Copiar licencias

# Processing Modal
modal-processing-title = ⏳ Procesando...
modal-processing-deletion = Eliminando archivos y directorios...
modal-processing-trash = Moviendo archivos y directorios a la papelera...
modal-processing-hardlink = Reemplazando duplicados con enlaces duros...
modal-processing-softlink = Reemplazando duplicados con enlaces simbólicos...

# Explorer Column Headers
explorer-hdr-name = Nombre
explorer-hdr-percentage = Porcentaje
explorer-hdr-size = Tamaño
explorer-hdr-items = Elementos
explorer-hdr-files = Archivos
explorer-hdr-subdirs = Subdirs
explorer-hdr-created = Creado
explorer-hdr-modified = Modificado

# Update Checker
update-checking = Buscando actualizaciones...
update-available = ¡Nueva versión { $version } disponible!
update-up-to-date = Ya está actualizado
update-failed = Error al buscar actualizaciones: { $error }

# Themes
theme = 🎨 Tema
theme-dark = Oscuro
theme-high-contrast = Alto contraste
theme-light = Claro
theme-system = Sistema

# New Scan Options Modal
modal-scan-options-title = Opciones de nuevo análisis
modal-scan-options-header = Iniciar nuevo análisis
modal-scan-options-path-label = Ruta del directorio a analizar:
modal-scan-options-paste-tooltip = Pegar desde el portapapeles
modal-scan-options-browse-tooltip = Buscar carpeta...
modal-scan-options-scan-btn = Analizar
modal-scan-options-cancel-btn = Cancelar
modal-scan-options-same-filesystem = Limitar el análisis al mismo sistema de archivos/volumen
modal-scan-options-drives-header = 💽 Unidades de almacenamiento y volúmenes
modal-scan-options-refresh-tooltip = Actualizar unidades de almacenamiento
modal-scan-options-root-system = Sistema raíz
modal-scan-options-selected-badge = ✅ Seleccionado
modal-scan-options-free-of = { $free } libres de { $total }
modal-scan-options-subtitle = Seleccione un volumen de almacenamiento, una ubicación rápida o un directorio personalizado para analizar.
modal-scan-options-quick-access = 📍 Accesos rápidos
modal-scan-options-path-hint = /ruta/para/escanear
modal-scan-options-hint = ℹ Seleccione una unidad arriba o ingrese una ruta de directorio.
modal-scan-options-sandbox-auth = 🔒 Se requiere acceso Sandbox — Haga clic en Escanear para conceder acceso
modal-scan-options-valid-dir = ✅ Directorio válido — Listo para escanear
modal-scan-options-points-to-file = ⚠ La ruta apunta a un archivo — por favor seleccione una carpeta.
modal-scan-options-dir-not-exist = ⚠ El directorio no existe en el sistema de archivos.
quick-loc-home = 🏠 Inicio
quick-loc-documents = 📄 Documentos
quick-loc-downloads = 📥 Descargas
quick-loc-desktop = 🖥 Escritorio
quick-loc-pictures = 🖼 Imágenes
search-use-regex = Usar expresión regular (Regex)
search-match-case = Coincidir mayúsculas y minúsculas
dedup-pref-dir-hint = ej. /home/user/Archive

file-menu-close = Cerrar análisis
file-menu-quit = Salir

badge-dataless-cloud = Archivo en la nube / sin datos locales
badge-symlink = Enlace simbólico
badge-special-file = Archivo especial (Tubería / Socket / Dispositivo)
badge-permission-denied = Acceso denegado

# Docker Disk Usage
vis-mode-docker = 📦 Docker
docker-desc = Uso de disco de la instalación local de Docker, leído directamente desde el directorio de datos en disco; no requiere conexión al daemon.
docker-no-install = No se detectó ninguna instalación de Docker en esta máquina.
docker-env-roots-title = Directorios de datos
docker-env-vmdisks-title = Discos de máquina virtual
docker-env-socket = Socket del daemon:
docker-env-roots-header = Raíces de datos ({ $count })
docker-env-vmdisks-header = Discos de máquinas virtuales ({ $count })
docker-kind-overlay2 = overlay2
docker-kind-containerd = containerd (almacén de imágenes)
docker-kind-unknown = diseño desconocido
docker-kind-overlay = overlay
docker-kind-aufs = aufs
docker-kind-btrfs = btrfs
docker-kind-zfs = zfs
docker-kind-devicemapper = devicemapper
docker-kind-vfs = vfs
docker-kind-windowsfilter = windowsfilter
docker-scope-system = sistema
docker-scope-rootless = rootless
docker-readable = ✅ legible
docker-not-readable = ❌ no legible
docker-vm-size = { $apparent } aparentes, { $allocated } asignados en disco
docker-vm-note = 💽 Docker Desktop guarda todos sus datos dentro del disco de la VM anterior; el contenido no puede atribuirse desde el host.
docker-needs-elevated = ⚠ Se encontró un directorio de datos de Docker pero no es legible. Ejecute eDirStat con privilegios elevados (sudo o administrador) para analizarlo.
docker-unsupported-layout = Esta raíz de datos de Docker usa un formato de almacenamiento que eDirStat aún no puede analizar (solo overlay2 por ahora).
docker-analyze = ⚡ Analizar
docker-analyzing = Analizando el directorio de datos de Docker...
docker-error = Error al inventariar Docker: { $error }
docker-lbl-images = Datos de imágenes
docker-lbl-shared = Capas compartidas
docker-lbl-unique = Datos de imágenes únicos
docker-lbl-containers = RW de contenedores
docker-lbl-volumes = Volumes
docker-lbl-build-cache = Build Cache
docker-lbl-logs = Registros de contenedores
docker-lbl-reclaimable = Recuperable
docker-section-images = Imágenes
docker-section-containers = Contenedores
docker-section-volumes = Volúmenes
docker-hdr-id = ID
docker-hdr-shared = Compartido
docker-hdr-exclusive = Exclusivo
docker-hdr-layers = Capas
docker-hdr-rw = Tamaño RW
docker-hdr-log = Tamaño de registro
docker-no-entries = Sin entradas.
docker-warnings-title = ⚠ Advertencias
docker-warn-large-log = El contenedor '{ $container }' tiene un archivo de registro grande ({ $size }).
docker-warn-unref-layer = La capa { $cache_id } no es referenciada por ninguna imagen ({ $size } recuperables).
docker-warn-root-unreadable = El directorio de datos { $path } no es legible (se requieren privilegios elevados).
docker-warn-metadata = Problema de metadatos: { $detail }
docker-warn-containerd-partial = El almacén de imágenes containerd está en uso: las imágenes y capas no se pueden atribuir desde el disco (soporte de la API de Docker planeado); este inventario solo cubre contenedores, registros, volúmenes y caché de compilación.
docker-source-disk = análisis del disco
docker-source-partial = análisis del disco (parcial)
docker-source-api = API de Docker
docker-collected-at = Recopilado el { $time }
docker-refresh = 🔄 Actualizar
docker-no-snapshot-data = Esta instantánea no contiene datos de Docker.
docker-sandbox-unavailable = La integración de Docker en vivo no está disponible en la versión de eDirStat de la App Store, pero las instantáneas recopiladas en otras máquinas pueden revisarse aquí.
docker-delete = Eliminar
docker-delete-title = Eliminar imagen de Docker
docker-delete-confirm = ¿Eliminar la imagen de Docker { $name } ({ $size }) a través del daemon de Docker?
docker-delete-warning = Esta acción elimina la etiqueta y borra la imagen a través del daemon de Docker. La operación no se puede deshacer; las capas que aún usan otras imágenes se conservan.
docker-delete-force = Forzar eliminación
docker-delete-force-hover = Quitar la imagen aunque haya contenedores que la usen (opción force del daemon).
docker-deleting = Eliminando imagen...
docker-delete-done = Imagen { $name } eliminada: { $untagged } etiqueta(s) eliminada(s), { $deleted } capa(s) borrada(s).
docker-delete-failed = No se pudo eliminar la imagen: { $error }
docker-delete-container = Eliminar contenedor
docker-delete-volume = Eliminar volumen
docker-delete-container-confirm = ¿Eliminar el contenedor { $name } (capa escribible { $size }, registro { $extra }) a través del daemon de Docker?
docker-delete-volume-confirm = ¿Eliminar el volumen { $name } ({ $size }) a través del daemon de Docker?
docker-delete-container-warning = Esto elimina el contenedor permanentemente a través del daemon de Docker. La operación no se puede deshacer.
docker-delete-volume-warning = Esto elimina el volumen y sus datos permanentemente a través del daemon de Docker. La operación no se puede deshacer.
docker-delete-container-force-hover = Quitar el contenedor aunque siga en ejecución (opción force del daemon).
docker-delete-volume-force-hover = Quitar el volumen aunque haya contenedores que lo usen (opción force del daemon).
docker-deleting-container = Eliminando contenedor...
docker-deleting-volume = Eliminando volumen...
docker-delete-container-done = Contenedor { $name } eliminado.
docker-delete-volume-done = Volumen { $name } eliminado.
docker-delete-container-failed = No se pudo eliminar el contenedor: { $error }
docker-delete-volume-failed = No se pudo eliminar el volumen: { $error }
docker-hdr-created = Creado
docker-hdr-refs = Refs
docker-delete-title-multi = Eliminar imágenes de Docker
docker-delete-container-multi = Eliminar contenedores
docker-delete-volume-multi = Eliminar volúmenes
docker-delete-confirm-multi = ¿Eliminar { $count } imágenes ({ $size } en total) a través del daemon de Docker?
docker-delete-container-confirm-multi = ¿Eliminar { $count } contenedores (capa escribible { $size }, registro { $extra } en total) a través del daemon de Docker?
docker-delete-volume-confirm-multi = ¿Eliminar { $count } volúmenes ({ $size } en total) a través del daemon de Docker?
docker-delete-more = ... y { $count } más
docker-delete-done-multi = { $count } imágenes eliminadas: { $untagged } etiqueta(s) eliminada(s), { $deleted } capa(s) borrada(s).
docker-delete-container-done-multi = { $count } contenedores eliminados.
docker-delete-volume-done-multi = { $count } volúmenes eliminados.
docker-delete-partial-multi = { $succeeded } de { $count } eliminados, { $failed } fallaron. Primer error: { $error }
badge-docker = Directorio de datos de Docker
badge-docker-vm = Disco virtual de Docker
badge-docker-area = Almacenamiento Docker { $area }

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
