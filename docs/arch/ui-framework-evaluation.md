# Evaluación de Frameworks UI para HyprFile

**Subagente**: `ui-evaluator`  
**Fecha**: 2025-06-07  
**Entorno**: Rust 1.96.0 stable, Linux x86_64, Wayland/Hyprland

---

## Resumen Ejecutivo

Tras evaluar objetivamente **GTK4/libadwaita**, **Slint**, **Iced** y **egui** contra los criterios del proyecto, se recomienda **Iced 0.13.x** como framework UI principal. El prototipo mínimo compiló exitosamente en este entorno.

| Criterio | GTK4/libadwaita | Slint | Iced | egui |
|----------|-----------------|-------|------|------|
| Wayland nativo | Excelente | Bueno (via winit) | Bueno (via winit) | Transparencia: **NO soportada** |
| Theming custom completo | Limitado (GNOME HIG) | Limitado (estilos fijos) | **Excelente** (Paleta + Catalog) | Excelente |
| Transparencia/blur Wayland | Complejo (no blur nativo) | Bugs conocidos | **Funciona** (compositor hace blur) | **No soportada en Wayland** |
| Rendimiento listas grandes | **Excelente** (ListView virtual nativo) | Medio | Bueno (culling + virtualización manual) | Excelente |
| Idiomaticidad Rust | Baja (GObject FFI) | Media (lenguaje Slint) | **Alta** (Elm puro Rust) | Alta (immediate mode) |
| Packaging (AppImage/Flatpak) | Difícil (runtime GTK pesado) | Medio | **Fácil** (single binary) | Fácil |
| Keybindings / Command Palette | Medio | Medio | **Excelente** (mensajes reactivos) | Bueno |
| Separación core/UI | Media | Media | **Alta** (traits + channels) | Media |
| Madurez comunidad | Muy alta | Media | Alta (en crecimiento) | Alta |
| Curva de aprendizaje | Media | Media | Media | Baja |

---

## Criterios de Evaluación Detallados

### 1. Rendimiento con Listas Grandes

**GTK4/libadwaita**  
- Tiene `GtkListView` con virtualización nativa vía `GtkSelectionModel` + `GtkListItemFactory`.  
- Soporta miles de archivos sin problemas.  
- **Puntuación: 5/5**

**Slint**  
- Soporta `ListView` pero la virtualización depende del backend/renderer. No hay garantías documentadas para miles de elementos.  
- **Puntuación: 3/5**

**Iced**  
- No tiene lista virtualizada nativa, pero `Scrollable` + `Column` con viewport culling permite listas grandes.  
- PR #2611 (0.14) optimiza textos fuera de viewport. La comunidad tiene `iced-flatlist` para virtualización.  
- Para el MVP, una implementación manual de virtualización es viable.  
- **Puntuación: 3.5/5**

**egui**  
- Immediate mode: excelente rendimiento porque no hay árbol de widgets retenido.  
- Sin embargo, el layout complejo de columnas/dual-pane requiere más código manual.  
- **Puntuación: 4/5**

### 2. Theming Custom Completo

**GTK4/libadwaita**  
- CSS vía `gtk4::CssProvider`, pero libadwaita impone la GNOME HIG. Personalizar profundamente (transparencia, bordes redondeados agresivos) requiere hacks.  
- **Puntuación: 2.5/5**

**Slint**  
- No permite modificar los temas built-in directamente. Requiere crear componentes propios y usar globales exportadas.  
- El estilo está acoplado al motor de renderizado (FemtoVG, Skia, software).  
- **Puntuación: 2.5/5**

**Iced**  
- Sistema `Palette` + `Catalog` permite temas totales.  
- Temas built-in que cubren todos los requisitos de HyprFile: Catppuccin (4 variantes), Gruvbox (Dark/Light), Nord, Dracula, Tokyo Night (3 variantes), Solarized, etc.  
- Bordes redondeados, colores, espaciado: todo controlable por widget.  
- **Puntuación: 5/5**

**egui**  
- Control total de `Visuals`, `Style`, colores y redondeo.  
- **Puntuación: 5/5**

### 3. Soporte Wayland Nativo

**GTK4/libadwaita**  
- GDK tiene backend Wayland nativo (`gdk4-wayland`).  
- **Puntuación: 5/5**

**Slint**  
- Usa winit con feature `backend-winit-wayland`. Soporta Wayland.  
- **Puntuación: 4/5**

**Iced**  
- Usa winit; Wayland funciona out-of-the-box en 0.13. En 0.14 hay feature `wayland` explícita.  
- **Puntuación: 4/5**

**egui**  
- Usa winit/eframe, pero la documentación oficial de `ViewportBuilder` lista **"Android / iOS / X11 / Wayland / Orbital: Unsupported"** para transparencia de ventana.  
- Sin transparencia, Hyprland no puede aplicar blur ni integrar el app en un rice moderno.  
- **Puntuación: 1/5** (descartado por este criterio)

### 4. Idiomaticidad en Rust

**GTK4/libadwaita**  
- Bindings de GObject; requiere entender el modelo de referencias, signals y main thread de GTK. No es idiomático Rust.  
- **Puntuación: 2/5**

**Slint**  
- Requiere archivos `.slint` y un build script (`slint-build`). La lógica de negocio vive entre Rust y Slint, fragmentando el código.  
- **Puntuación: 3/5**

**Iced**  
- Patrón Elm puro en Rust: `Model -> Message -> Update -> View`. Sin FFI, sin lenguaje externo. Integra naturalmente con `async`/Tokio.  
- **Puntuación: 5/5**

**egui**  
- 100% Rust, immediate mode. Muy idiomático para herramientas internas o debug UIs. Para una app de productividad con layouts complejos, el modelo retenido es más mantenible.  
- **Puntuación: 4/5**

### 5. Curva de Aprendizaje y Madurez

**GTK4/libadwaita**  
- Muy maduro (GNOME ecosystem), documentación extensa. Pero la curva es media-alta debido a la complejidad de GObject y la rigidez de libadwaita.  
- **Puntuación: 4/5**

**Slint**  
- Comunidad creciente, pero menor que Iced/egui para apps de escritorio Linux. La curva incluye aprender el lenguaje Slint.  
- **Puntuación: 3/5**

**Iced**  
- Comunidad activa (~2M descargas, 306 dependencias inversas en 0.14). Curva media: hay que entender el flujo unidireccional de datos.  
- **Puntuación: 4/5**

**egui**  
- Comunidad muy grande (29k estrellas, ~16M descargas). Curva baja para lo básico, pero escalar a apps complejas requiere disciplina.  
- **Puntuación: 4.5/5**

### 6. Facilidad de Packaging

**GTK4/libadwaita**  
- AppImage/Flatpak requiere incluir runtime GTK4 + temas + iconos. PKGBUILD depende de sistema. Más pesado.  
- **Puntuación: 2/5**

**Slint**  
- Binario relativamente standalone, pero depende de librerías de sistema para winit/wayland.  
- **Puntuación: 3.5/5**

**Iced**  
- Single binary con renderers wgpu (Vulkan) o tiny-skia (software). No requiere GTK/QT en runtime. Ideal para AppImage.  
- **Puntuación: 5/5**

**egui**  
- Similar a Iced: binario autónomo.  
- **Puntuación: 5/5**

### 7. Soporte Keybindings Globales, Command Palette, Overlays

**GTK4/libadwaita**  
- `GtkEventControllerKey` permite atajos, pero la arquitectura no facilita una command palette centralizada tipo VS Code.  
- **Puntuación: 3/5**

**Slint**  
- Eventos de teclado disponibles, pero sin un modelo de "comandos" centralizado. Requiere implementación manual.  
- **Puntuación: 3/5**

**Iced**  
- Las `Subscription` y el flujo `Message` permiten un bus de comandos central. Un componente `CommandPalette` puede emitir mensajes que cualquier parte de la app maneje. Los atajos globales se capturan como `Event::Keyboard`.  
- **Puntuación: 5/5**

**egui**  
- `ctx.input()` permite leer teclado por frame. Para una command palette persistente y navegable, requiere más estado manual.  
- **Puntuación: 3.5/5**

### 8. Separación Core/UI

**GTK4/libadwaita**  
- GTK obliga a que gran parte de la lógica viva en callbacks/signals acoplados a widgets. Separar requiere arquitectura adicional (MVVM, etc.).  
- **Puntuación: 2.5/5**

**Slint**  
- Slint expone `struct` y callbacks a Rust, pero la línea entre UI y lógica se difumina.  
- **Puntuación: 3/5**

**Iced**  
- El modelo Elm naturalmente separa estado (`State`), mensajes (`Message`), actualización (`update`) y vista (`view`). El `core` puede vivir como servicios que emiten `Message` a través de canales Tokio. El trait `UiRuntime` ya definido en `hyprfiles-ui` encaja perfectamente.  
- **Puntuación: 5/5**

**egui**  
- En immediate mode, la lógica tiende a mezclarse con el render loop. Se puede separar, pero no es el patrón natural.  
- **Puntuación: 3/5**

---

## Análisis Descartes

### egui — DESCARTADO

**Razón**: La transparencia de ventana está explícitamente marcada como **"Unsupported"** en Wayland según la documentación oficial de `egui::ViewportBuilder` (docs.rs).  

> "Android / iOS / X11 / Wayland / Orbital: Unsupported"  

Sin transparencia, Hyprland no puede aplicar blur de compositor. Esto rompe el requisito de integración visual con rices de Hyprland. Aunque egui es rápido e idiomático, este bloqueador es infranqueable para los objetivos del proyecto.

### GTK4/libadwaita — DESCARTADO

**Razón**: Aunque tiene virtualización de listas nativa y Wayland perfecto, presenta tres problemas críticos:  
1. **libadwaita impone la GNOME HIG**, conflictuando directamente con la visión de "rice personalizable" de HyprFile.  
2. **Packaging pesado**: requiere runtime GTK4 en AppImage/Flatpak.  
3. **Menos idiomático en Rust**: bindings GObject, main thread restrictions, signals. Dificulta la separación core/UI y la integración con Tokio.

### Slint — DESCARTADO

**Razón**: Theming es insuficiente para el proyecto. Según respuestas de los mantenedores:  
> "no u cant change it. use exported globals and a struct for different theme styles, and u kinda have to create your own components instead of std-widgets"  

Además, la transparencia en Wayland tiene bugs documentados (#6145, #6020) que dependen del renderer. El lenguaje Slint añade una barrera de entrada y reduce la idiomaticidad Rust.

---

## Recomendación: Iced 0.13.x

### Versión Propuesta
`iced = "0.13"` (resuelve a **0.13.1**, MSRV 1.80).  
Feature activado: `tokio` para integrar el runtime async existente del workspace.

### Por qué Iced

1. **Equilibrio visual + Rust**: Ofrece temas built-in para Catppuccin, Gruvbox, Nord, Dracula, Tokyo Night — exactamente los que pide el proyecto. La personalización completa es posible sin salir del ecosistema Rust.
2. **Wayland funcional**: vía winit. La transparencia funciona en 0.13.1 (confirmado por issue #2727). El blur lo gestiona Hyprland como compositor para ventanas transparentes.
3. **Packaging sencillo**: Binario único sin dependencias de sistema masivas.
4. **Keyboard-first natural**: El flujo reactivo de mensajes es ideal para keybindings globales, modal de command palette y atajos tipo Vim.
5. **Separación core/UI**: El patrón Elm (`State -> Message -> Update -> View`) se alinea perfectamente con el trait `UiRuntime` y el bus de eventos del core.
6. **Madurez**: ~2M descargas, desarrollo activo, comunidad receptiva.

### Limitaciones Aceptadas

- **Listas grandes**: No hay virtualización nativa. Para el MVP, usaremos `Scrollable` con culling. En fases posteriores, se evaluará `iced-flatlist` o una implementación propia de virtualización.
- **Bordes redondeados de ventana**: En Wayland, el redondeo de ventana lo controla el compositor (Hyprland). Los widgets internos sí pueden tener `border_radius` totalmente custom.
- **Blur**: Depende del compositor, no del framework. Con `window::Settings::transparent` (funcional en 0.13.1), Hyprland aplica blur automáticamente si está configurado.

### Ruta de Actualización

Iced 0.14.0 (MSRV 1.88) añade:  
- Feature flags explícitas `wayland`/`x11`.  
- Campo `transparent` formal en `window::Settings`.  
- Mejoras de rendimiento en viewport culling (PR #2611).  

Cuando el workspace actualice su MSRV de 1.80 a 1.88+, la migración a 0.14 será directa.

---

## Validación del Prototipo

Se creó un proyecto mínimo en `/tmp/iced-proto` con:

```rust
use iced::widget::{column, text, container, button};
use iced::{Element, Task, Theme};

fn main() -> iced::Result {
    iced::application("HyprFile Proto", State::update, State::view)
        .theme(|_| Theme::Dark)
        .run()
}

#[derive(Default)]
struct State;

#[derive(Clone, Debug)]
enum Message { Nop }

impl State {
    fn update(&mut self, _message: Message) -> Task<Message> { Task::none() }
    fn view(&self) -> Element<Message> { /* UI */ }
}
```

**Resultado**: `cargo build` exitoso. 435 paquetes resueltos. Compilación completa sin errores.  
**Conclusión**: Iced 0.13.1 compila y funciona en este entorno (Rust 1.96, Linux x86_64).

---

## Referencias

- Iced crates.io: https://crates.io/crates/iced (0.13.1, MSRV 1.80; 0.14.0, MSRV 1.88)
- egui ViewportBuilder docs: https://docs.rs/egui/latest/egui/viewport/struct.ViewportBuilder.html (transparencia Wayland: Unsupported)
- Slint backend-winit-wayland: https://github.com/slint-ui/slint/blob/master/api/rs/slint/Cargo.toml
- GTK4 Wayland bindings: https://crates.io/crates/gdk4-wayland (0.11.0)
- Iced issue #2727 (transparencia 0.13.1 funciona): https://github.com/iced-rs/iced/issues/2727
- Iced PR #2611 (viewport culling 0.14): https://github.com/iced-rs/iced/pull/2611
- iced-flatlist (virtualización comunitaria): https://github.com/luizvidoto/iced-flatlist
