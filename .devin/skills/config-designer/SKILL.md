---
name: config-designer
description: Diseña e implementa sistemas de configuración declarativa en Rust usando serde + TOML/YAML/RON, con recarga en caliente, validación, y valores por defecto robustos.
triggers:
  - user
  - model
allowed-tools:
  - read
  - write
  - edit
  - exec
  - glob
---

Diseña un sistema de configuración declarativa para una aplicación Rust.

Pasos:
1. Analizar qué secciones de config necesita la app (general, ui, keybindings, preview, behavior, etc.).
2. Definir structs Rust con `serde::Deserialize` y valores por defecto con `#[serde(default)]`.
3. Elegir formato: TOML por defecto, YAML/RON opcional.
4. Implementar carga desde XDG config dir (`dirs::config_dir()` o `xdg`).
5. Implementar recarga en caliente con `notify` si aplica.
6. Validar config al cargar (rangos, paths existentes, etc.).
7. Proveer ejemplos de config en `examples/` o `docs/`.

Restricciones:
- Nunca hardcodear rutas del usuario; usar XDG dirs.
- Documentar cada campo con comentarios en los ejemplos TOML.
- Manejar errores de parseo amigablemente, indicando línea y campo problemático.
