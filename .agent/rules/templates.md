---
trigger: glob
globs: "templates/**/*.typ, src/pdf.rs"
description: TemplateData/invoice_data.json contract synchronization across all Typst templates and PDF compilation tests.
---

# Typst Templates & PDF Rules

1. **Konzistence datového kontraktu**
   - Každá změna v `TemplateData` (`src/models.rs`) nebo ve struktuře `invoice_data.json` se musí promítnout do **všech** vestavěných `.typ` šablon v `templates/` (včetně případných sdílených `.typ` modulů a logiky jejich seedování v `src/config.rs`).
   - Při změně datového kontraktu aktualizuj tabulku polí v [ARCHITECTURE.md](../../ARCHITECTURE.md) a uživatelskou referenci v [README.md](../../README.md).

2. **Povinné ověření kompilace PDF**
   - Po úpravě `src/pdf.rs`, `TemplateData` nebo jakékoli `.typ` šablony spusť `cargo test` a ověř, že všechny vestavěné šablony projdou testem kompilace PDF bez chyb.
