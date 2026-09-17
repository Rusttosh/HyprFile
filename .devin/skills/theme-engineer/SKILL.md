---
name: theme-engineer
description: Diseña e implementa un engine de temas visual para aplicaciones Rust, con soporte para modos claro/oscuro, paletas predefinidas, transparencia, bordes, y sincronización con pywal/matugen.
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

Diseña un sistema de temas visuales para una aplicación Linux desktop en Rust.

Pasos:
1. Definir estructura de tema: colores base, superficies, acentos, semánticos (error, warning, info).
2. Soportar modos: dark, light, auto (detectar sistema).
3. Crear temas predefinidos: Catppuccin Mocha/Latte, Gruvbox, Tokyo Night, Nord, Dracula.
4. Definir propiedades de UI configurables: transparencia, blur, border_radius, padding, gaps, icon_theme, nerd_font_icons.
5. Implementar carga desde archivo TOML de tema con fallback a tema por defecto.
6. Preparar hook futuro para sincronización con pywal/matugen (leer archivo de colores generado).
7. Documentar formato de tema y crear ejemplos.

Restricciones:
- Separar definición de tema de la implementación de renderizado; el tema es solo datos.
- Usar serde para serialización/deserialización.
- Validar valores (transparencia 0.0-1.0, border_radius >= 0).
