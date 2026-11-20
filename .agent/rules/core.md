---
trigger: always_on
description: Core language, stack, git, verification, and documentation rules for the faktury project.
---

# Core Project Rules

1. **Jazyk a pojmenování**
   - S uživatelem komunikuj česky.
   - Veškerý kód (Rust, Slint), názvy souborů, identifikátory, struktury, funkce, proměnné a komentáře piš **anglicky**.
   - Pouze uživatelské texty v UI (`ui/`), chybové/stavové hlášky pro uživatele a výstup PDF šablon jsou **česky**.

2. **Architektura a závislosti**
   - Zachovej čistě synchronní architekturu bez async runtime (nepřidávej Tokio).
   - Udržuj kompatibilitu s křížovou kompilací pro Windows (`x86_64-pc-windows-gnu`).
   - **Zákaz přidávat nové C/C++ závislosti do `Cargo.toml`:** používej pouze čistý Rust / `rustls` (jedinou povolenou stávající výjimkou je `rusqlite` s feature `["bundled"]`).

3. **Git a workflow**
   - **Nikdy necommituj ani nepushuj přímo do `main`** bez výslovného souhlasu uživatele.

4. **Ověření a Circuit Breaker**
   - Po změnách v kódu vždy ověř build a testy (`cargo check`, `cargo test`).
   - **Circuit breaker:** pokud `cargo check` nebo `cargo test` selže **3× po sobě na stejné chybě**, okamžitě zastav úpravy a nahlas uživateli přesnou chybu.

5. **Report po změně kódu**
   - Po každé změně kódu vypiš stručný **Code Walkthrough** (max. 5 bodů: tok dat, klíčové soubory a funkce k přečtení, přijaté kompromisy).
   - Přidej sekci **Documentation Pointers** s přesnými názvy modulů, traitů nebo kapitol v oficiální dokumentaci použitých knihoven.

6. **Synchronizace dokumentace**
   - Při každé změně architektury, modulů nebo datového modelu aktualizuj [ARCHITECTURE.md](../../ARCHITECTURE.md).
   - Při změně uživatelského rozhraní šablon (`invoice_data.json`) nebo `config.toml` aktualizuj také [README.md](../../README.md).
