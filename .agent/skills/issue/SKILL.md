---
name: issue
description: Implement a GitHub issue end-to-end on a dedicated branch, verify with cargo test, update docs if needed, and open a Pull Request with Closes #<id>. Triggered via /issue <id>.
---

# Implementace GitHub Issue (`/issue <id>`)

Tento skill slouží k kompletnímu zpracování konkrétního GitHub Issue od načtení zadání až po otevření Pull Requestu.

## Postup

1. **Načti zadání Issue**
   - Spusť `gh issue view <id>` a přečti celý popis, komentáře i labely.
   - Pokud je v zadání nejasnost nebo nejednoznačné rozhodnutí, doptáš se uživatele před zahájením úprav.

2. **Připrav samostatnou větev**
   - Zkontroluj `git status` (čistý working tree) a ujisti se, že vycházíš z aktuální větve `main`.
   - **Nikdy necommituj ani nepushuj přímo do `main`.**
   - Vytvoř větev pojmenovanou podle typu a čísla issue:
     - `feat/<id>-<slug>` (nová funkce / `enhancement`),
     - `fix/<id>-<slug>` (oprava chyby / `bug`),
     - `refactor/<id>-<slug>` (refaktoring nebo `tech-debt`).

3. **Implementace a ověření**
   - Implementuj změny v souladu se všemi pravidly v `.agent/rules/` (`core.md` a kontextová pravidla `db.md`, `ui.md`, `templates.md`).
   - Doplň nebo aktualizuj relevantní unit testy.
   - Ověř změny příkazy:
     - `cargo test`
     - `cargo clippy --all-targets`
   - Dodržuj circuit breaker: pokud `cargo check` nebo `cargo test` selže **3× po sobě na stejné chybě**, zastav úpravy a nahlas přesnou chybu uživateli.

4. **Aktualizace dokumentace**
   - Pokud se změnila architektura, moduly, DB schéma, konfigurace nebo datový kontrakt `invoice_data.json`, aktualizuj [ARCHITECTURE.md](../../../ARCHITECTURE.md) (a případně [README.md](../../../README.md)).

5. **Commit, push a Pull Request**
   - Zkontroluj `git diff` a `git status`.
   - Commitni změny s výstižnou anglickou commit zprávou na nově vytvořenou větev.
   - Pushni větev na `origin` (`git push -u origin <branch>`).
   - Otevři Pull Request přes `gh pr create`, v jehož těle bude uveden odkaz `Closes #<id>` a stručný popis změn i ověření.
   - V závěrečné odpovědi uživateli uveď odkaz na PR, **Code Walkthrough** (max. 5 bodů) a **Documentation Pointers** dle `.agent/rules/core.md`.
