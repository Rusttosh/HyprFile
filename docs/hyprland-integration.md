# Hyprland Integration for HyprFile

## Overview

`hyprfiles-integrations` provides system-level integrations for HyprFile, a file manager designed primarily for Hyprland users but compatible with any Linux desktop environment (Sway, River, GNOME, KDE, X11).

All Hyprland-specific functionality is **opt-in / auto-detected** and never fails if Hyprland is not present.

---

## 1. XDG Compliance (`src/xdg/`)

### Decisiones técnicas

- **Uso de `dirs` crate + env vars**: Priorizamos las variables de entorno (`XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, `XDG_DATA_HOME`) y caemos en los defaults de `dirs` cuando no están definidas. Esto asegura compatibilidad con cualquier sesión Linux, no solo Hyprland.
- **Rutas de la aplicación**:
  - Config: `~/.config/hyprfiles/`
  - Cache: `~/.cache/hyprfiles/`
  - Data: `~/.local/share/hyprfiles/`
- **MIME database**: Se escanean las rutas estándar (`/usr/share/mime`, `~/.local/share/mime`, y las derivadas de `XDG_DATA_DIRS`). Implementamos un parser XML ligero para extraer `mime-type` y `comment` sin depender de crates XML pesados.
- **xdg-open**: Se invoca vía `tokio::process::Command` para no bloquear el runtime async.
- **Icon themes**: Se escanean directorios estándar (`/usr/share/icons`, `/usr/local/share/icons`, `~/.local/share/icons`) buscando `index.theme`. La resolución de iconos es básica (busca `.svg`, `.png`, `.xpm` en subdirectorios `apps/`).

---

## 2. Desktop Entries (`src/desktop_entries/`)

### Decisiones técnicas

- **Parser manual**: `.desktop` files son archivos INI simples. Implementamos un parser propio en lugar de añadir una dependencia de `ini`/`configparser`, reduciendo el árbol de dependencias.
- **Filtrado**: Se omiten entradas con `NoDisplay=true` o `Hidden=true`.
- **MIME matching**: Las aplicaciones se buscan por `MimeType` declarado. Soporta matching exacto (case-insensitive).
- **Exec field codes**: Reemplazamos `%f`, `%F`, `%u`, `%U` por el path del archivo. Otros field codes se eliminan. Usamos un mini-split de comillas básicas para separar el comando.
- **MIME guessing**: Dado que `mime_guess` no está en el workspace, mantenemos un mapa de extensiones comunes a MIME types. Fallback a `xdg-mime query filetype` si no hay match.

---

## 3. Hyprland Integration (`src/hyprland/`)

### Detección segura

- `detect_hyprland()` verifica:
  1. Existencia de `HYPRLAND_INSTANCE_SIGNATURE` en env.
  2. Disponibilidad de `hyprctl` en `$PATH`.
- Si ninguna condición se cumple, todas las funciones retornan error descriptivo sin panic.

### Funciones implementadas

| Función | Descripción |
|---------|-------------|
| `get_active_monitor()` | Ejecuta `hyprctl monitors -j` y parsea JSON vía `serde`. |
| `get_active_workspace()` | Ejecuta `hyprctl workspaces -j` y parsea el primer workspace. |
| `send_floating_hint()` | Genera strings `windowrulev2` para `hyprland.conf` (ej: float + size). |
| `picker_mode_config()` | Configuración sugerida para modo picker (float, center, borderless). |
| `read_system_colors()` | Lee colores generados por **pywal** (`~/.cache/wal/colors.json`) o **matugen** (`~/.config/matugen/colors.json`). |

### Ejemplos de keybinds (`hyprland.conf`)

```ini
# Abrir HyprFile en directorio home
bind = SUPER, E, exec, hyprfiles

# Modo picker (ventana flotante compacta)
bind = SUPER SHIFT, E, exec, hyprfiles --picker

# Reglas recomendadas para picker (pegar en hyprland.conf)
windowrulev2 = float,class:hyprfiles
windowrulev2 = size 900 650,class:hyprfiles
windowrulev2 = center,class:hyprfiles
windowrulev2 = borderless,class:hyprfiles
```

---

## 4. Terminal Integration (`src/terminal/`)

### Decisiones técnicas

- **Detección priorizada**:
  1. Variable `HYPRFILES_TERMINAL`.
  2. Variable `TERM` si contiene un terminal conocido.
  3. Primer terminal conocido en `$PATH`: `foot`, `alacritty`, `kitty`, `wezterm`, `gnome-terminal`, `konsole`, `xterm`.
- **Apertura en directorio**: Cada terminal recibe su flag nativa de working directory (`--working-directory`, `--directory`, `--workdir`, etc.).
- **Graceful fallback**: Si no se encuentra ninguno, se usa `xterm`.

---

## 5. Portals (`src/portals/`)

### Estado actual

- Estructura placeholder lista para futura integración D-Bus.
- `detect_xdg_desktop_portal_hyprland()` busca el binario o el systemd service file en rutas típicas (`/usr/lib`, `/usr/libexec`, `/usr/local`).
- Funciones placeholder: `open_file_chooser()`, `take_screenshot()`.

### Futuro

La integración completa de portales requerirá hablar con `org.freedesktop.portal.Desktop` usando `zbus` o `dbus-rs`. Esto queda fuera del scope MVP pero la API está preparada.

---

## 6. Trash System Integration (`src/trash.rs`)

### Validación XDG

- El crate `trash` (v5) sigue la especificación freedesktop:
  - Archivos trashed: `$XDG_DATA_HOME/Trash/files/`
  - Metadatos: `$XDG_DATA_HOME/Trash/info/{name}.trashinfo`
- Nuestra integración valida que esos directorios existan y puede leer los `.trashinfo` para listar items con su ruta original y fecha.

### Comportamiento en diferentes filesystems

| Escenario | Comportamiento |
|-----------|----------------|
| Mismo filesystem | `trash` crate usa `rename()` (instantáneo). |
| Cross-device (ej: montaje externo) | Fallback a copy + delete; más lento pero seguro. |
| Top-level directories (`.Trash`) | El crate maneja automáticamente la creación de `.Trash-{uid}` en dispositivos externos si aplica. |

---

## 7. Testing

- **28 tests** en `hyprfiles-integrations`, todos pasan.
- Mock de variables de entorno para probar detección sin Hyprland instalado.
- Archivos temporales `.desktop` y `.trashinfo` para probar parsers sin depender del host.
- Tests de XDG dirs que funcionan en contenedores mínimos (sin `/usr/share/mime`).

---

## 8. Reglas críticas seguidas

- **Nunca se asume Hyprland**: `detect_hyprland()` es la puerta de entrada; todas las funciones devuelven `Err` descriptivo si no está.
- **Logging con `tracing`**: Todos los módulos usan `trace`, `debug`, `info`, `warn` según severidad.
- **Async-first**: Operaciones I/O bloqueantes (trash, xdg-open, terminal, hyprctl) usan `tokio::process` o `tokio::task::spawn_blocking`.
