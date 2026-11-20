---
trigger: glob
globs: "src/db.rs, src/models.rs"
description: Backwards-compatible SQLite migrations, transactions, and migration unit tests.
---

# Database & Models Rules

1. **Zpětně kompatibilní migrace**
   - Každá změna schématu SQLite musí být zpětně kompatibilní a **nikdy nesmí vyžadovat smazání existující databáze `faktury.db`**.
   - Migrace prováděj inkrementálně a idempotentně (např. podle `PRAGMA user_version`), se zachováním všech stávajících dat.

2. **Transakce a integrita dat**
   - Všechny vícekrokové databázové zápisy (např. uložení, úprava nebo smazání faktury včetně `line_items`, případně přestavba tabulek při migraci) musí běžet uvnitř SQLite transakce (`conn.transaction()` / `BEGIN ... COMMIT`).
   - Při otevírání databáze zapínej a respektuj cizí klíče (`PRAGMA foreign_keys = ON`).

3. **Testování migrací a modelů**
   - Ke každé změně schématu nebo migračnímu kroku napiš unit test v in-memory SQLite (`Connection::open_in_memory()`), který ověří jak čistou inicializaci nové DB, tak migraci ze starší verze schématu naplněné testovacími daty.
