# Resumen del Proyecto HyprFile

## Propósito
HyprFile (nombre provisional HyprFiles) es un gestor de archivos gráfico moderno para Linux, diseñado para usuarios de tiling window managers —especialmente Hyprland— que buscan velocidad, control por teclado, estética personalizable y operaciones seguras.

## Objetivos Clave
1. **Keyboard-first**: toda operación accesible por teclado, con keybindings tipo Vim/Yazi.
2. **Visualmente elegante**: temas configurables, transparencia, blur, bordes redondeados, integración con rices.
3. **Rápido**: navegación instantánea, previews lazy, operaciones async, listas virtuales.
4. **Seguro**: papelera por defecto, confirmaciones configurables, manejo robusto de errores.
5. **Nativo Linux**: XDG, MIME, .desktop, portales, freedesktop trash spec.
6. **Modular**: core independiente de la UI, preparado para plugins futuros.

## Stack Tecnológico (Propuesto)
- **Lenguaje**: Rust
- **Async**: Tokio
- **UI**: Por decidir (GTK4/libadwaita, Slint, Iced, egui)
- **Config**: serde + TOML
- **Logging**: tracing
- **Errores**: anyhow + thiserror
- **CLI**: clap

## Arquitectura
Workspace multi-crate:
- `hyprfiles-core`: filesystem, jobs, preview, search, MIME, trash, config, theme, events
- `hyprfiles-ui`: app, layout, widgets, panels, command palette, dialogs, icons
- `hyprfiles-integrations`: hyprland, xdg, portals, terminal, desktop_entries
- `hyprfiles-plugins`: runtime y API para extensiones futuras
- `hyprfiles-cli`: punto de entrada, flags, picker mode

## Roadmap (Fases)
1. **Fase 0**: Diseño técnico y decisión de stack UI.
2. **Fase 1**: MVP funcional (ventana, listado, navegación, preview básico, trash, config).
3. **Fase 2**: Operaciones robustas (copy, move, progreso, cola de tareas).
4. **Fase 3**: Estética y temas.
5. **Fase 4**: Integración Hyprland.
6. **Fase 5**: Previews avanzados.
7. **Fase 6**: Plugins.
8. **Fase 7**: Packaging (AppImage, Flatpak, PKGBUILD, Nix).

## Subagentes Definidos
| Nombre | Rol | Responsabilidades |
|--------|-----|-------------------|
| `rust-architect` | Arquitecto Rust | Workspace, modelos de datos, eventos, ADRs |
| `ui-evaluator` | Especialista UI | Evaluación de frameworks UI, prototipo mínimo |
| `core-engineer` | Ingeniero del Core | Filesystem, jobs, preview, MIME, trash, search |
| `hyprland-integrator` | Integrador Hyprland | Detección, IPC, modos, XDG, portales |
| `qa-tester` | QA y Seguridad | Tests, CI, observabilidad, documentación de seguridad |

## Skills Definidas
| Nombre | Propósito |
|--------|-----------|
| `rust-workspace-setup` | Scaffolding de workspace Cargo multi-crate |
| `config-designer` | Diseño de configuración declarativa TOML |
| `theme-engineer` | Diseño de engine de temas visuales |

## Estado Actual
- Workspace Rust creado y compilando.
- Scaffolding de crates y módulos listo.
- Traits y structs base del core definidos.
- Subagentes y skills configurados en `.devin/`.
- Documentación inicial en `docs/arch/`.

## Siguiente Paso
Ejecutar el subagente `ui-evaluator` para decidir el framework UI, y luego `rust-architect` para refinar la arquitectura con la decisión tomada.
