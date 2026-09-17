# ADR-001: Stack y Arquitectura Inicial

## Estado
Propuesto

## Contexto
HyprFile es un gestor de archivos gráfico para Linux, keyboard-first, integrado con Wayland/Hyprland, inspirado en Yazi pero independiente.

## Decisiones Pendientes
1. **Framework UI**: GTK4/libadwaita, Slint, Iced o egui. El subagente `ui-evaluator` debe decidir.
2. **Plugin runtime**: Lua, Rhai, WASM o scripts externos. Post-MVP.
3. **Sistema de eventos**: channels de Tokio, broadcast, o actor model.

## Decisiones Tomadas
- **Lenguaje**: Rust (estable, async, seguro, nativo Linux).
- **Runtime async**: Tokio.
- **Configuración**: TOML + serde.
- **Logging**: tracing + tracing-subscriber.
- **Errores**: anyhow (app boundary) + thiserror (library boundary).
- **Workspace**: multi-crate con crates separadas por capa (core, ui, integrations, plugins, cli).
- **Patrón de arquitectura**: core desacoplado de UI mediante traits y event bus.
- **Testing**: tests unitarios en cada crate con `tempfile`.
- **CI**: GitHub Actions (pendiente de implementar por `qa-tester`).

## Consecuencias
- Alta testabilidad del core.
- Flexibilidad para cambiar el framework UI en el futuro.
- Complejidad inicial mayor por el multi-crate, pero mejor mantenibilidad a largo plazo.

## Notas
- El subagente `rust-architect` debe refinar este ADR con decisiones definitivas.
