---
name: rust-workspace-setup
description: Configura un workspace Cargo multi-crate con estructura inicial, dependencias comunes, y scaffolding de crates para proyectos Rust complejos.
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

Configura un workspace Cargo inicial para un proyecto Rust multi-crate.

Pasos:
1. Verificar que `cargo` y `rustc` estén instalados. Si no, reportarlo.
2. Crear `Cargo.toml` workspace raíz con `resolver = "3"` y lista de crates miembros.
3. Crear la estructura de directorios sugerida:
   - `crates/<crate-name>/Cargo.toml` y `src/lib.rs` para cada crate.
   - `crates/<crate-name>/src/` submódulos según corresponda.
4. Agregar dependencias comunes en workspace.dependencies si aplica: tokio, serde, anyhow, thiserror, tracing, tracing-subscriber.
5. Ejecutar `cargo check` para validar que compila.
6. Si falla, reportar el error claramente.

Restricciones:
- No escribas código de lógica de negocio; solo scaffolding, `pub mod`, y dependencias.
- Usa Rust Edition 2024 si está disponible; si no, 2021.
