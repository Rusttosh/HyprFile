Actúa como un arquitecto senior de software, desarrollador experto en Rust, Linux desktop, Wayland, Hyprland, gestores de archivos, UX keyboard-first y aplicaciones nativas modernas.

Quiero que diseñes y desarrolles una aplicación llamada provisionalmente HyprFiles, un gestor de archivos gráfico para Linux inspirado en la experiencia de Yazi, pero NO debe ser un simple wrapper encima de Yazi. La aplicación debe ser independiente, visual, estética, keyboard-first, rápida, confiable y profundamente integrada con entornos Wayland/Hyprland.

El objetivo es crear un gestor de archivos moderno para usuarios de tiling window managers, especialmente Hyprland, con una experiencia similar a Yazi en velocidad, navegación y previews, pero con una interfaz gráfica atractiva, configurable y coherente con temas visuales tipo ricing.

---

1. Visión del producto

Crea un gestor de archivos gráfico para Linux que combine:

- La velocidad y filosofía keyboard-first de Yazi.
- Una interfaz visual moderna, minimalista y altamente personalizable.
- Integración nativa con Wayland, Hyprland, XDG, portales, temas y aplicaciones predeterminadas.
- Operaciones de archivos seguras, asíncronas y con progreso visible.
- Previews avanzados de archivos.
- Soporte para themes, iconos, transparencias, bordes, blur cuando sea posible y layouts configurables.
- Configuración declarativa mediante archivos TOML, YAML o RON.
- Arquitectura modular que permita plugins y extensiones en el futuro.
- Diseño robusto: nunca sacrificar seguridad de datos por estética.

La app debe sentirse como una mezcla entre Yazi, un launcher moderno, un file picker avanzado y un gestor gráfico nativo para Hyprland.

---

2. Nombre provisional

Usa como nombre provisional:

HyprFiles

Pero diseña el proyecto para que el nombre pueda cambiarse fácilmente.

---

3. Principios de diseño

La aplicación debe seguir estos principios:

1. Keyboard-first: todo debe poder hacerse con el teclado.
2. Mouse-friendly: aunque el teclado sea prioritario, debe soportar mouse, drag & drop, selección múltiple y menús contextuales.
3. Visualmente elegante: interfaz moderna, limpia, themeable y adaptable a rices de Hyprland.
4. Rápida: navegación instantánea, carga progresiva, previews lazy y operaciones asíncronas.
5. Segura: operaciones destructivas protegidas, papelera por defecto, confirmaciones configurables, manejo de errores claro.
6. Modular: separar core, UI, integración Linux, previews, jobs y plugins.
7. Nativa Linux: usar estándares XDG, MIME, trash spec, portales y aplicaciones predeterminadas.
8. Wayland-first: priorizar Wayland/Hyprland, sin romper compatibilidad general con otros entornos Linux.
9. No bloquear la UI: ninguna operación pesada debe congelar la interfaz.
10. Configuración transparente: archivos de configuración legibles y documentados.

---

4. Stack tecnológico recomendado

Evalúa y justifica el stack final antes de implementar.

Preferencia inicial:

- Lenguaje principal: Rust
- Runtime asíncrono: Tokio
- UI: evaluar entre:
  - GTK4/libadwaita si se prioriza integración nativa Linux.
  - Slint si se prioriza estética custom y buena arquitectura.
  - Iced si se prioriza Rust puro y UI declarativa.
  - egui solo si encaja bien con el objetivo visual.
- File watching: "notify"
- Operaciones de papelera: "trash-rs" o implementación compatible con freedesktop trash spec.
- MIME/open with: integración con "xdg-open", MIME database y ".desktop".
- Configuración: "serde" + TOML.
- Logging: "tracing".
- Errores: "anyhow"/"thiserror".
- CLI interna opcional: "clap".
- Plugins futuros: Lua, Rhai o WASM, evaluar pros y contras.
- Integración Hyprland: IPC mediante sockets UNIX y/o comandos "hyprctl" cuando sea razonable.
- Portales: integración futura con "xdg-desktop-portal" y "xdg-desktop-portal-hyprland".

Antes de escribir código, decide cuál framework UI es más adecuado y explica por qué. Para el primer MVP, prioriza una base estable y mantenible sobre efectos visuales complejos.

---

5. Inspiración funcional de Yazi

La app debe inspirarse en Yazi en estos aspectos:

- Navegación rápida por teclado.
- Paneles de navegación.
- Preview lateral.
- Operaciones asíncronas.
- Cola de tareas.
- Feedback visual de progreso.
- Acciones configurables.
- Comandos rápidos.
- Filosofía minimalista y veloz.

Pero NO debe depender del binario de Yazi como backend obligatorio. Puede estudiar su filosofía y conceptos, pero debe implementar su propio core.

Si se reutiliza código o ideas de proyectos open source, revisar licencias y atribución correctamente. No copiar código sin respetar licencia.

---

6. Arquitectura deseada

Diseña el proyecto con una separación clara de capas:

hyprfiles/
  crates/
    hyprfiles-core/
      fs/
      jobs/
      preview/
      search/
      mime/
      trash/
      config/
      theme/
      events/
    hyprfiles-ui/
      app/
      layout/
      widgets/
      panels/
      command_palette/
      dialogs/
      icons/
      animations/
    hyprfiles-integrations/
      hyprland/
      xdg/
      portals/
      terminal/
      desktop_entries/
    hyprfiles-plugins/
      runtime/
      api/
    hyprfiles-cli/
      main.rs

La arquitectura debe permitir que el core pueda probarse sin UI.

---

7. Módulos principales

Implementa o diseña estos módulos:

7.1 Core de filesystem

Debe soportar:

- Listar directorios.
- Ordenar por nombre, tipo, tamaño, fecha, extensión.
- Mostrar archivos ocultos.
- Filtrar archivos.
- Leer metadatos.
- Detectar symlinks.
- Detectar permisos.
- Detectar tipo de archivo.
- Manejar errores de permisos.
- Evitar seguir symlinks peligrosos sin control.
- Soportar rutas locales inicialmente.
- Preparar arquitectura para rutas remotas o montajes en el futuro.

7.2 Job queue

Todas las operaciones peligrosas o pesadas deben pasar por una cola de tareas.

Debe soportar:

- Copiar.
- Mover.
- Renombrar.
- Enviar a papelera.
- Borrar permanentemente, solo con confirmación explícita.
- Crear carpeta.
- Crear archivo.
- Duplicar archivo.
- Comprimir.
- Extraer.
- Calcular tamaño de carpeta.
- Cancelar operación.
- Pausar/reanudar si es viable.
- Mostrar progreso.
- Mostrar errores parciales.
- Reintentar operaciones fallidas.
- Manejar conflictos de nombres.
- Sobrescribir, renombrar automáticamente, saltar o cancelar.

La UI no debe ejecutar operaciones destructivas directamente.

7.3 Preview engine

Diseña previews extensibles para:

- Texto plano.
- Código fuente con syntax highlighting.
- Markdown.
- JSON/YAML/TOML.
- Imágenes.
- PDF.
- Video.
- Audio metadata.
- Archivos comprimidos.
- Carpetas.
- Binarios.
- Symlinks.
- Permisos y metadata.

El sistema debe ser lazy, cacheable y cancelable. Si el usuario cambia de archivo rápidamente, cancelar previews anteriores.

7.4 Thumbnails

Diseña caché de thumbnails:

- Imágenes.
- Videos.
- PDF primera página.
- Carpetas opcionalmente.
- Invalidación por modificación del archivo.
- Límite de tamaño.
- Limpieza automática.
- Ubicación compatible con estándares XDG cuando sea posible.

7.5 Búsqueda

Implementa búsqueda por:

- Nombre.
- Extensión.
- Tipo.
- Contenido usando ripgrep si está instalado.
- Fuzzy search.
- Filtros rápidos.
- Integración futura con fd/ripgrep.

La búsqueda debe no bloquear la UI.

7.6 MIME y abrir con

Debe soportar:

- Abrir archivo con aplicación predeterminada.
- Abrir con aplicación específica.
- Leer ".desktop" entries.
- Usar "xdg-open" como fallback.
- Mostrar diálogo “abrir con”.
- Copiar ruta.
- Copiar nombre.
- Copiar URI file://.
- Abrir terminal en ruta actual.
- Abrir editor configurado.

7.7 Papelera

Por defecto, borrar debe enviar a papelera, no eliminar permanentemente.

Debe soportar:

- Mover a papelera.
- Restaurar desde papelera en el futuro.
- Borrar permanentemente solo con confirmación.
- Compatible con freedesktop trash spec.
- Mensajes claros sobre operaciones destructivas.

---

8. UI/UX deseada

La interfaz debe ser moderna, rápida y estética.

8.1 Layout principal

Debe tener layouts configurables:

1. Yazi-like
   
   - Columna padre.
   - Columna actual.
   - Preview lateral.

2. Dual pane
   
   - Panel izquierdo.
   - Panel derecho.
   - Transferencias entre paneles.

3. Grid mode
   
   - Vista de iconos/thumbnails.

4. List mode
   
   - Vista detallada con columnas.

5. Compact floating mode
   
   - Ideal para abrir como ventana flotante en Hyprland.

8.2 Componentes visuales

Debe incluir:

- Barra superior opcional.
- Breadcrumb de ruta.
- Panel de archivos.
- Preview lateral.
- Command palette.
- Barra de estado.
- Indicador de tareas en curso.
- Modal de confirmación.
- Menú contextual.
- Selector “open with”.
- Overlay de búsqueda.
- Toast notifications.
- Panel de detalles.
- Vista de permisos.
- Vista de metadata.

8.3 Estética

Debe soportar:

- Bordes redondeados.
- Transparencia configurable.
- Blur si el compositor lo permite.
- Padding configurable.
- Gaps.
- Iconos Nerd Font o icon theme.
- Colores por tema.
- Modo oscuro.
- Modo claro opcional.
- Temas tipo Catppuccin, Gruvbox, Tokyo Night, Nord, Dracula.
- Integración futura con pywal/matugen.
- CSS o sistema de estilos configurable, según el framework UI.

La app debe poder verse bien en un rice de Hyprland.

8.4 Interacción keyboard-first

Debe soportar keybindings configurables.

Keybindings base sugeridos:

j / Down        mover abajo
k / Up          mover arriba
h / Left        ir al directorio padre
l / Enter       abrir archivo o entrar carpeta
gg              ir al inicio
G               ir al final
Space           seleccionar archivo
v               modo selección
/               buscar
:               command palette
yy              copiar ruta o archivo
dd              enviar a papelera
p               pegar
r               renombrar
n               nuevo archivo
N               nueva carpeta
.               mostrar ocultos
Tab             cambiar panel
q               cerrar
Ctrl+r          refrescar
Ctrl+h          alternar ocultos
Ctrl+c          cancelar tarea actual

Debe permitir mapear acciones en config.

8.5 Command palette

Implementar una paleta tipo comandos:

:open
:open-with
:copy
:move
:rename
:trash
:delete-permanently
:mkdir
:touch
:compress
:extract
:chmod
:copy-path
:copy-name
:copy-uri
:toggle-hidden
:search
:bulk-rename
:open-terminal
:reload-config
:theme
:quit

La command palette debe ser extensible por plugins en el futuro.

---

9. Integración con Hyprland

Diseñar integración específica con Hyprland, pero sin hacer que la app dependa exclusivamente de Hyprland.

Funciones deseadas:

- Detectar si se está ejecutando en Hyprland.
- Leer información mediante Hyprland IPC o "hyprctl".
- Detectar monitor activo.
- Detectar workspace actual.
- Opcional: abrir en modo flotante recomendado.
- Opcional: enviar hints al usuario para reglas en "hyprland.conf".
- Integración con colores de theme.
- Integración futura con "hyprland.conf", pywal o matugen.
- Modo “picker flotante” para abrir rápido desde un keybind.
- Opción para cerrar automáticamente al seleccionar archivo en modo picker.
- Soporte para lanzar desde keybind de Hyprland.

Ejemplo de uso esperado:

bind = SUPER, E, exec, hyprfiles
bind = SUPER SHIFT, E, exec, hyprfiles --picker

La app debe funcionar también en otros entornos Wayland/X11 con menor integración.

---

10. Integración Linux/XDG

Debe respetar estándares Linux:

- XDG config directory.
- XDG cache directory.
- XDG data directory.
- MIME database.
- ".desktop" files.
- "xdg-open".
- Papelera freedesktop.
- Portales cuando sea necesario.
- File chooser integration en el futuro.
- Icon themes.
- Variables de entorno comunes.

Ubicaciones sugeridas:

~/.config/hyprfiles/config.toml
~/.config/hyprfiles/keymaps.toml
~/.config/hyprfiles/theme.toml
~/.cache/hyprfiles/
~/.local/share/hyprfiles/

---

11. Configuración

Diseña configuración declarativa.

Ejemplo:

[general]
show_hidden = false
confirm_delete = true
default_layout = "yazi"
open_dirs_on_single_click = false
terminal = "foot"
editor = "nvim"

[ui]
theme = "catppuccin-mocha"
transparency = 0.92
blur = true
border_radius = 12
padding = 8
show_breadcrumb = true
show_status_bar = true
icon_theme = "Papirus-Dark"
nerd_font_icons = true

[preview]
enabled = true
max_file_size_mb = 10
image_preview = true
pdf_preview = true
video_thumbnails = true
syntax_highlighting = true

[hyprland]
enabled = true
detect_active_monitor = true
picker_mode_floating = true
sync_with_pywal = false
sync_with_matugen = false

[behavior]
trash_by_default = true
permanent_delete_requires_phrase = true
case_sensitive_sort = false
directories_first = true

[keybindings]
quit = ["q"]
open = ["Enter", "l"]
parent = ["h"]
down = ["j", "Down"]
up = ["k", "Up"]
search = ["/"]
command_palette = [":"]
toggle_hidden = ["."]
trash = ["dd"]
rename = ["r"]
new_file = ["n"]
new_folder = ["N"]

La app debe poder recargar configuración sin reiniciarse cuando sea razonable.

---

12. Seguridad y protección de datos

Este punto es crítico.

Implementa medidas como:

- Borrar manda a papelera por defecto.
- Borrado permanente requiere confirmación fuerte.
- Operaciones destructivas deben mostrar resumen.
- No sobrescribir sin preguntar.
- En conflictos, ofrecer:
  - reemplazar;
  - renombrar;
  - saltar;
  - cancelar.
- Mostrar errores claramente.
- Mantener log de operaciones recientes.
- Evitar seguir symlinks peligrosos sin aviso.
- No ejecutar scripts/plugins sin permisos explícitos.
- Separar operaciones de UI y filesystem.
- Tests para operaciones de archivos en directorios temporales.
- Nunca bloquear la UI durante operaciones largas.
- Cancelación segura de operaciones.
- Manejo de permisos insuficientes.
- Manejo de discos desmontados durante una operación.
- Manejo de rutas inexistentes.
- Manejo de archivos modificados durante operación.

---

13. Plugins y extensibilidad

Diseña una arquitectura futura de plugins, aunque no se implemente completa en el MVP.

Los plugins deberían poder:

- Agregar comandos.
- Agregar previews.
- Agregar acciones contextuales.
- Agregar integraciones.
- Leer selección actual.
- Leer ruta actual.
- Mostrar notificaciones.
- Pedir confirmación al usuario.
- Ejecutar comandos externos con permisos controlados.

Evaluar runtimes:

- Lua: familiar para usuarios de Yazi/Neovim.
- Rhai: integración Rust simple.
- WASM: aislamiento más fuerte.
- Scripts externos: simple pero menos seguro.

Para MVP, basta con diseñar la interfaz de plugins y quizá permitir acciones externas simples configuradas por TOML.

---

14. CLI

La app debe tener flags útiles:

hyprfiles
hyprfiles /ruta/inicial
hyprfiles --picker
hyprfiles --dual-pane
hyprfiles --config /ruta/config.toml
hyprfiles --theme catppuccin-mocha
hyprfiles --select /ruta/archivo
hyprfiles --debug
hyprfiles --no-hyprland

Modo picker:

- Abre ventana compacta.
- Permite elegir archivo o carpeta.
- Imprime resultado por stdout o lo copia al portapapeles según flag.
- Opcionalmente cierra al seleccionar.

Ejemplo:

hyprfiles --picker --stdout

---

15. Roadmap de desarrollo

Divide el proyecto en fases.

Fase 0: Diseño técnico

Entregar:

- Decisión de stack.
- Estructura de proyecto.
- Modelo de datos.
- Arquitectura de eventos.
- Diseño de job queue.
- Diseño de UI.
- Formato de config.
- Riesgos técnicos.

Fase 1: MVP funcional

Implementar:

- App abre una ventana.
- Lista directorios.
- Navegación con teclado.
- Entrar/salir de carpetas.
- Mostrar archivos ocultos.
- Vista tipo columnas.
- Preview básico de texto e imagen.
- Abrir archivo con app predeterminada.
- Crear carpeta.
- Renombrar.
- Enviar a papelera.
- Config básica.
- Keybindings básicos.

Fase 2: Operaciones robustas

Implementar:

- Copiar.
- Mover.
- Pegar.
- Progreso.
- Cancelación.
- Manejo de conflictos.
- Cola de tareas.
- Toasts.
- Logs.

Fase 3: Estética y temas

Implementar:

- Theme engine.
- Transparencia.
- Bordes.
- Gaps.
- Iconos.
- Temas predefinidos.
- Modo compacto.
- Personalización visual.

Fase 4: Integración Hyprland

Implementar:

- Detección Hyprland.
- Modo picker.
- IPC básico.
- Detección monitor/workspace.
- Recomendaciones de reglas Hyprland.
- Integración con pywal/matugen si es viable.

Fase 5: Previews avanzados

Implementar:

- PDF.
- Video thumbnails.
- Markdown render.
- Syntax highlighting.
- Archivos comprimidos.
- Metadata avanzada.
- Caché de thumbnails.

Fase 6: Plugins

Implementar:

- API inicial.
- Comandos externos.
- Acciones custom.
- Documentación para plugins.
- Ejemplos.

Fase 7: Packaging

Implementar:

- Build release.
- AppImage.
- Flatpak.
- Arch PKGBUILD.
- Nix flake opcional.
- ".desktop" file.
- Iconos.
- Documentación de instalación.

---

16. Testing

Incluir tests desde el inicio.

Tests mínimos:

- Listado de directorios.
- Ordenamiento.
- Filtros.
- Renombrado.
- Crear carpeta.
- Enviar a papelera usando directorios temporales.
- Copiar archivos pequeños.
- Copiar carpetas.
- Manejo de conflictos.
- Cancelación de jobs.
- Parser de config.
- Parser de keybindings.
- Preview de texto.
- Detección MIME.
- No bloquear UI durante jobs.

Usar directorios temporales para evitar tocar archivos reales del usuario.

---

17. Observabilidad y debugging

Implementar:

- Logging con niveles.
- Flag "--debug".
- Logs en archivo opcional.
- Panel de errores reciente.
- Mensajes amigables.
- Backtraces en modo debug.
- Métricas simples de tareas:
  - archivos procesados;
  - bytes copiados;
  - duración;
  - errores;
  - tareas canceladas.

---

18. Performance

Optimizar para:

- Directorios con miles de archivos.
- Previews cancelables.
- Carga progresiva.
- Cache de thumbnails.
- No bloquear la UI.
- Evitar lecturas innecesarias.
- Debounce en file watcher.
- Virtualized list/grid si la UI lo permite.
- Lazy metadata loading.
- Lazy icon loading.

---

19. Accesibilidad

Aunque la app esté orientada a usuarios avanzados, debe considerar:

- Navegación completa por teclado.
- Contraste suficiente.
- Tamaño de fuente configurable.
- Indicadores claros de selección/foco.
- No depender únicamente del color para estados críticos.
- Confirmaciones textuales para operaciones destructivas.

---

20. Documentación

Crear documentación para:

- Instalación.
- Uso básico.
- Keybindings.
- Configuración.
- Themes.
- Integración con Hyprland.
- Modo picker.
- Operaciones de archivos.
- Seguridad.
- Desarrollo.
- Roadmap.
- Contribución.
- Licencia.

Incluir ejemplos de configuración.

---

21. Entregables que debes producir

Primero entrega un documento técnico con:

1. Resumen del proyecto.
2. Decisión del stack.
3. Arquitectura.
4. Estructura de carpetas.
5. Modelo de datos principal.
6. Sistema de eventos.
7. Diseño de job queue.
8. Diseño de previews.
9. Diseño de configuración.
10. Diseño de themes.
11. Integración Hyprland.
12. Riesgos y mitigaciones.
13. Roadmap por fases.

Después genera el código inicial del proyecto con:

1. Workspace Rust.
2. Crates separados si aplica.
3. App mínima que abra ventana.
4. Listado real de archivos.
5. Navegación básica.
6. Config inicial.
7. Keybindings básicos.
8. Preview simple de texto.
9. Operación segura de enviar a papelera.
10. Logging básico.
11. Tests iniciales.

No generes solo pseudocódigo. Produce una base real y compilable. Si alguna parte depende del framework UI elegido, usa APIs reales y actuales. Si hay incertidumbre sobre una librería, explícala y propón alternativa.

---

22. Reglas de implementación

Sigue estas reglas:

- Prioriza seguridad de datos.
- No implementes borrado permanente en el primer MVP salvo que esté fuertemente protegido.
- No bloquees el hilo principal de UI.
- No mezcles lógica de filesystem con widgets de UI.
- No hardcodees rutas del usuario.
- Usa XDG dirs.
- Usa errores tipados donde tenga sentido.
- Escribe código legible y mantenible.
- Añade comentarios solo donde aporten claridad.
- Incluye tests.
- Documenta decisiones técnicas.
- Mantén la app usable aunque Hyprland no esté presente.
- Diseña para Wayland-first, pero no rompas compatibilidad general.
- Evita dependencias innecesarias.
- Antes de implementar algo grande, explica el tradeoff.

---

23. Resultado esperado

El resultado final debe ser una base sólida para un gestor de archivos gráfico moderno llamado HyprFiles:

- Rápido.
- Bonito.
- Keyboard-first.
- Inspirado en Yazi.
- Nativo para Linux.
- Integrado con Hyprland.
- Configurable.
- Seguro.
- Modular.
- Extensible.

Comienza por el diseño técnico y luego crea el código inicial compilable del MVP.