# ADR-002: Framework UI — Iced 0.13.x

## Estado
Aceptado

## Contexto
HyprFile requiere un framework UI que:
- Funcione nativamente en Wayland (entorno objetivo: Hyprland).
- Permita theming profundo (Catppuccin, Gruvbox, Nord, Tokyo Night, Dracula), transparencia y bordes redondeados.
- Sea keyboard-first, soportando keybindings globales y command palette.
- Mantenga una separación limpia entre core y UI mediante traits/channels.
- Facilite el packaging como AppImage, Flatpak y PKGBUILD.
- Sea idiomatico en Rust y mantenga una curva de aprendizaje razonable para el equipo.

Se evaluaron cuatro opciones reales: GTK4/libadwaita, Slint, Iced y egui.

## Opciones Consideradas

### 1. GTK4/libadwaita (v0.11.3 / libadwaita 0.9.1)
- **Ventajas**: Wayland nativo, `GtkListView` con virtualización nativa, madurez extrema.
- **Desventajas**: libadwaita impone GNOME HIG (conflicto con rice personalizable); bindings GObject no son idiomáticos Rust; packaging pesado (runtime GTK); main-thread restrictions dificultan Tokio.
- **Veredicto**: Descartado por rigidez temática y peso de runtime.

### 2. Slint (v1.15.1 / 1.16.1)
- **Ventajas**: Declarativo, soporta backend-winit-wayland, buen rendimiento general.
- **Desventajas**: Theming limitado (requiere crear widgets propios desde cero); bugs documentados en transparencia Wayland (#6020, #6145); lenguaje Slint reduce idiomaticidad Rust; comunidad de desktop Linux más pequeña.
- **Veredicto**: Descartado por limitaciones de theming y problemas de transparencia.

### 3. egui (v0.34.2)
- **Ventajas**: Inmediato, 100% Rust, rendimiento excelente, theming total.
- **Desventajas**: **Transparencia de ventana en Wayland explícitamente no soportada** (`ViewportBuilder` docs: "Android / iOS / X11 / Wayland / Orbital: Unsupported"). Sin transparencia, Hyprland no puede aplicar blur. Immediate mode complica layouts complejos (dual-pane, columnas Yazi-like).
- **Veredicto**: Descartado por bloqueador de transparencia Wayland.

### 4. Iced (v0.13.1)
- **Ventajas**:
  - Temas built-in que cubren todos los requerimientos (Catppuccin 4 variantes, Gruvbox, Nord, Dracula, Tokyo Night).
  - Sistema `Palette` + `Catalog` para theming total y personalizado.
  - Wayland funcional via winit; transparencia confirmada en 0.13.1 (issue #2727).
  - Single binary, packaging sencillo (AppImage/Flatpak/PKGBUILD).
  - Patrón Elm puro Rust: separación natural entre core y UI.
  - Integración Tokio via feature `tokio`.
  - Comunidad activa (~2M descargas).
- **Desventajas**:
  - Sin virtualización nativa de listas (requiere solución manual o `iced-flatlist` para miles de elementos).
  - La personalización de chrome de ventana en Wayland es limitada (compositor gestiona decoraciones).

## Decisión

Se adopta **Iced 0.13.x** como framework UI de HyprFile.

Dependencia concreta en `crates/hyprfiles-ui/Cargo.toml`:

```toml
iced = { version = "0.13", features = ["tokio"] }
```

MSRV: 1.80 (coincide con el workspace actual).

## Justificación

1. **Theming**: Iced es el único framework evaluado que ofrece temas built-in para *todas* las paletas requeridas por el proyecto sin salir del ecosistema Rust.
2. **Wayland + Transparencia**: egui fue eliminado por no soportar transparencia en Wayland. Slint tiene bugs abiertos. GTK4 es funcional pero pesado. Iced 0.13.1 funciona y permite que el compositor (Hyprland) aplique blur sobre ventanas transparentes.
3. **Idiomaticidad**: El patrón Elm (`State -> Message -> Update -> View`) es 100% Rust y se alinea con el trait `UiRuntime` y el bus de eventos del core.
4. **Packaging**: No requiere runtime GTK/QT. Ideal para distribución como AppImage.
5. **Keyboard-first**: Las `Subscription` y el flujo `Message` facilitan una command palette centralizada y keybindings globales sin callbacks circulares.

## Consecuencias

- **Positivas**:
  - Desarrollo UI en Rust puro, sin FFI ni lenguajes externos.
  - Facilidad para cambiar temas en runtime.
  - Binario autónomo simplifica distribución.
  - Arquitectura desacoplada core/UI lista para escalar.

- **Negativas / Riesgos**:
  - Listas grandes (miles de archivos) requerirán implementación manual de virtualización en el MVP. Mitigación: empezar con `Scrollable` + culling; escalar a `iced-flatlist` o componente propio virtualizado en fase 2.
  - El redondeo de ventana y blur dependen del compositor Hyprland, no de Iced. Esto es aceptable porque es el modelo estándar en Wayland.

## Notas Técnicas

- Renderer por defecto: `wgpu` (usa Vulkan en Linux). Fallback software: `tiny-skia`.
- Feature `tokio`: permite que `Commands` y `Subscriptions` usen el runtime Tokio del workspace.
- Ruta de migración a 0.14: cuando el workspace actualice MSRV a 1.88+, la migración es directa y aporta mejoras de viewport culling y flags `wayland`/`x11` explícitas.

## Validación

Se construyó un prototipo mínimo compilable en `/tmp/iced-proto` usando `iced = "0.13"`. El build completó exitosamente en el entorno actual (Rust 1.96.0, Linux x86_64), confirmando que la dependencia resuelve y compila correctamente.
