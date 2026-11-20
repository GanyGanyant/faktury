# Architektura projektu Faktury

Tento dokument popisuje aktuální architekturu, datové toky, schémata a klíčové invarianty desktopové aplikace **Faktury**.

---

## 1. Technologický stack

- **Jazyk a běhový model:** Rust (edice `2024`, viz [Cargo.toml](Cargo.toml)), čistě synchronní architektura bez asynchronního runtime (nepoužívá Tokio).
- **GUI:** `slint` `1.9` + `slint-build` `1.9` ([build.rs](build.rs) kompiluje [ui/main.slint](ui/main.slint), do aplikace vloženo přes `slint::include_modules!()`).
- **Databáze:** `rusqlite` `0.32` s feature `["bundled"]` (SQLite soubor `faktury.db`).
- **Konfigurace a systémové cesty:** `directories` `5` (`ProjectDirs::from("", "", "faktury")`, `UserDirs::new()`), `toml` `0.8` (`config.toml`).
- **HTTP a JSON:** `ureq` `2.12` s feature `["json"]` (výchozí TLS přes `rustls`), `serde` `1` (`derive`), `serde_json` `1`.
- **Datum a čas:** `chrono` `0.4` (`Local`, `NaiveDate`, `Datelike`, `Duration`).
- **PDF engine a prohlížeč:**
  - Primárně externí CLI `typst compile <template_path> <output_path>`.
  - Při chybě nebo nedostupnosti externího CLI fallback na in-process kompilátor `typst-as-lib` `0.17` (features `["packages", "ureq"]`) + `typst-pdf` `0.15` s přibalenými fonty *Libertinus Sans* (Regular, Bold, Italic z `fonts/`).
  - Otevření výsledného PDF v systémovém prohlížeči přes crate `open` `5`.
- **Release & CI:**
  - Release profil v [Cargo.toml](Cargo.toml): `strip = true`, `lto = true`, `codegen-units = 1`, `opt-level = "z"`, `panic = "abort"`.
  - Release build na Windows skrývá konzolové okno (`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` v [src/main.rs](src/main.rs)).
  - CI workflow [.github/workflows/ci.yml](.github/workflows/ci.yml) spouštěné na `push` a `pull_request` do větve `main` ověřuje `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` a `cargo test`.
  - Release workflow [.github/workflows/release.yml](.github/workflows/release.yml) spouštěné na tazích `v*` a ručně (`workflow_dispatch`) sestavuje binárky pro Linux x64, macOS ARM64 (`aarch64-apple-darwin`) a Windows x64 a publikuje GitHub Release.

---

## 2. Mapa modulů

| Cesta | Role |
| :--- | :--- |
| [build.rs](build.rs) | Build skript kompilující `ui/main.slint` pomocí `slint_build::compile`. |
| [src/main.rs](src/main.rs) | Vstupní bod aplikace, inicializace cest/konfigurace/DB, držení aplikačního stavu (`Rc<RefCell<...>>`, `VecModel`), registrace 18 Slint callbacků, logika kalendáře a přepočtů v UI. |
| [src/models.rs](src/models.rs) | Doménové struktury (`Supplier`, `Customer`, `LineItem`, `Invoice`), JSON kontrakt pro Typst (`TemplateData`, `TemplateSupplier`, `TemplateCustomer`, `TemplateLineItem`), `TemplateData::from_parts` a výpočetní/formátovací funkce (`round2`, `format_amount`, `compute_totals`). |
| [src/config.rs](src/config.rs) | Rozlišení standardních OS cest (`AppPaths`), čtení/zápis `config.toml` (`AppConfig`), jednorázová migrace `./faktury.db` a tabulky `supplier`, seedování vestavěných `.typ` šablon (`BUILTIN_KLASICKA`, `BUILTIN_MODERNI`) a výpis dostupných šablon. |
| [src/db.rs](src/db.rs) | Inicializace SQLite schématu (`invoices`, `line_items`, `customers`), CRUD nad odběrateli a fakturami, generování pořadových čísel faktur a variabilních symbolů pro daný rok. |
| [src/ares.rs](src/ares.rs) | Synchronní HTTP klient pro veřejné REST API registru ARES (`AresResult`, `fetch`) a parsování obchodního jména, DIČ a sídla. |
| [src/pdf.rs](src/pdf.rs) | Zápis `invoice_data.json`, kompilace PDF (`typst` CLI s fallbackem na vestavěný `TypstEngine` + `typst_pdf::pdf`), odvození výstupní cesty PDF (`pdf_output_path`) a otevření souboru (`open::that`). |
| [ui/main.slint](ui/main.slint) | Kompletní definice Slint GUI (`AppWindow`, pomocné komponenty `Card`, `PrimaryButton`, `SecondaryButton`, `IconButton`, `CalendarPopup`, `DayCell`, datové struktury `LineItemUI`, `InvoiceRow`, `CalendarDay`, `CalendarWeek` a 3 záložky). |
| [templates/faktura.typ](templates/faktura.typ) | Výchozí klasická Typst šablona (při seedování zapisována do uživatelské složky jako `klasicka.typ`). |
| [templates/moderni.typ](templates/moderni.typ) | Moderní Typst šablona s barevným akcentem (zapisována jako `moderni.typ`). |
| `fonts/` | Písma `LibertinusSans-Regular.otf`, `LibertinusSans-Bold.otf`, `LibertinusSans-Italic.otf` vkládaná do binárky přes `include_bytes!`. |
| [.github/workflows/ci.yml](.github/workflows/ci.yml) | CI pipeline pro `push`/`pull_request` do `main` (formátování, Clippy, unit testy). |
| [.github/workflows/release.yml](.github/workflows/release.yml) | Multi-platformní CI pipeline pro sestavení a publikaci GitHub Release. |

---

## 3. Umístění souborů v OS (`AppPaths`)

Řídí modul [src/config.rs](src/config.rs) pomocí `directories::ProjectDirs::from("", "", "faktury")` a `directories::UserDirs::new()`:

- **`config_dir`**: konfigurační adresář OS (`~/.config/faktury` na Linuxu; fallback `.config/faktury`). Obsahuje:
  - `config.toml` (`AppPaths::config_file()`)
  - `templates/` (`AppPaths::templates_dir()`) – uživatelské i seedované `.typ` šablony a dočasný `invoice_data.json`
- **`data_dir`**: datový adresář OS (`~/.local/share/faktury` na Linuxu; fallback `.local/share/faktury`). Obsahuje:
  - `faktury.db` (`AppPaths::db_path()`)
- **`default_output_dir`**: adresář `Documents/Faktury` uživatele (fallback `data_dir/invoices`). Použije se jako výchozí hodnota `AppConfig::output_dir`, pokud není v `config.toml` nastavena jiná cesta.

### Jednorázová migrace starších souborů při startu

1. **`config::migrate_legacy_db_if_needed`**: pokud cílový `paths.db_path()` ještě neexistuje a v aktuálním pracovním adresáři existuje `./faktury.db`, zkopíruje ji do `paths.db_path()`.
2. **`config::load_or_init`**: pokud `config.toml` neexistuje, pokusí se otevřít `./faktury.db` nebo `paths.db_path()`, přečíst záznam `WHERE id = 1` z legacy tabulky `supplier` (`load_legacy_supplier`) a uložit hodnoty do nově vytvořeného `config.toml`.

---

## 4. Tok dat

```text
Start aplikace:
  resolve_paths() -> ensure_dirs() -> migrate_legacy_db_if_needed()
    -> load_or_init(config.toml) -> ensure_default_templates() -> list_templates()
    -> open_db(faktury.db) -> db::init()
    -> AppWindow::new() -> naplnění UI + history_model + saved_customers + kalendář
    -> window.run()

Interakce z UI (Slint callbacky v src/main.rs):
  [Načíst z ARES] -> ares::fetch(ico) -> GET ares.gov.cz -> UI cust-*
  [Uložit nastavení] -> aktualizace config_state -> config::save_config(config.toml)
  [Uložit a generovat PDF] -> open_db() -> (volitelně db::save_customer)
    -> collect_items() + compute_totals()
    -> db::save_invoice / db::update_invoice
    -> TemplateData::from_parts() -> <templates_dir>/invoice_data.json
    -> pdf::compile() [typst CLI -> fallback TypstEngine + typst_pdf::pdf]
    -> db::set_invoice_pdf_path() -> pdf::open()
    -> obnovení historie, posun čísla/VS, reset formuláře
```

### Hlavní toky podrobně

1. **Start aplikace ([src/main.rs](src/main.rs)):**
   - Inicializuje cesty (`AppPaths`), provede případnou migraci DB a konfigurace, do `templates_dir` zapíše chybějící vestavěné šablony (`klasicka.typ`, `moderni.typ`) a načte seznam `*.typ` souborů.
   - Otevře SQLite databázi a zavolá `db::init`.
   - Spočítá výchozí číslo faktury (`next_invoice_number`) a variabilní symbol (`next_variable_symbol`) pro aktuální kalendářní rok, datum vystavení (dnešek) a datum splatnosti (`today + default_due_days`).
   - Vytvoří `AppWindow`, naplní `VecModel` pro položky, historii faktur, seznam uložených odběratelů a mřížku kalendáře (6×7 dnů) a spustí `window.run()`.
2. **Načtení odběratele z ARES (`fetch-ares`):**
   - `ares::fetch` odešle `GET https://ares.gov.cz/ekonomicke-subjekty-v-be/rest/ekonomicke-subjekty/{ico}` s hlavičkou `Accept: application/json`.
   - Z odpovědi vytáhne `obchodniJmeno`, `ico`, `dic` a složí `sidlo` (`nazevUlice` + `cisloDomovni`/`cisloOrientacni`, formátované PSČ `NNN NN`, `nazevObce`).
   - Výsledek naplní vlastnosti `cust-*` v `AppWindow`; při chybě nastaví `ares-status` a `ares-status-is-error = true`.
3. **Uložení faktury a generování PDF (`save-invoice`):**
   - Otevře DB, pokud odběratel má neprázdné IČO, provede upsert do `customers` a aktualizuje rozbalovací seznam v UI.
   - Posbírá položky z `items_model`, spočítá součty přes `models::compute_totals` a podle `editing_invoice_id` zavolá buď `db::save_invoice`, nebo `db::update_invoice`.
   - Sestaví `TemplateData::from_parts(&invoice, &supplier, &customer, &items)`, zapíše JSON do `<templates_dir>/invoice_data.json` a přes `pdf::compile` vygeneruje PDF na cestu `pdf::pdf_output_path(&output_dir, &invoice.number)`.
   - Po úspěšné kompilaci uloží cestu PDF přes `db::set_invoice_pdf_path`, otevře soubor (`pdf::open`), obnoví historii, posune číslo faktury a VS a vyčistí formulář.
4. **Operace nad historií:**
   - `edit-invoice(id)`: zavolá `db::load_invoice_detail`, nastaví `editing_invoice_id = Some(id)`, naplní formulář včetně položek, přepočte součty, zapne `is-editing = true` a přepne `active-tab` na `0`.
   - `delete-invoice(id)`: zavolá `db::delete_invoice`, pokud faktura měla uložený `pdf_path`, smaže soubor z disku (`std::fs::remove_file`), případně resetuje formulář, pokud byla mazaná faktura právě v editaci, a obnoví historii.
   - `regenerate-pdf(id)`: načte detail přes `db::load_invoice_detail`, zkombinuje jej s aktuálním `supplier` z `config_state` a znovu spustí `pdf::compile` + `pdf::open`.

---

## 5. Aktuální SQLite schéma

Definováno v `db::init` ([src/db.rs](src/db.rs)):

```sql
CREATE TABLE IF NOT EXISTS invoices (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    number          TEXT NOT NULL,
    variable_symbol TEXT NOT NULL,
    issue_date      TEXT NOT NULL,
    due_date        TEXT NOT NULL,
    payment_method  TEXT NOT NULL DEFAULT 'Převodem',
    note            TEXT NOT NULL DEFAULT '',
    total_excl_vat  REAL NOT NULL DEFAULT 0,
    total_vat       REAL NOT NULL DEFAULT 0,
    total_incl_vat  REAL NOT NULL DEFAULT 0,
    customer_name   TEXT NOT NULL DEFAULT '',
    customer_reg_id TEXT NOT NULL DEFAULT '',
    customer_vat_id TEXT NOT NULL DEFAULT '',
    customer_street TEXT NOT NULL DEFAULT '',
    customer_zip    TEXT NOT NULL DEFAULT '',
    customer_city   TEXT NOT NULL DEFAULT '',
    pdf_path        TEXT
);

CREATE TABLE IF NOT EXISTS line_items (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id  INTEGER NOT NULL REFERENCES invoices(id),
    description TEXT NOT NULL DEFAULT '',
    quantity    REAL NOT NULL DEFAULT 1,
    unit        TEXT NOT NULL DEFAULT 'ks',
    unit_price  REAL NOT NULL DEFAULT 0,
    vat_rate    INTEGER NOT NULL DEFAULT 21
);

CREATE TABLE IF NOT EXISTS customers (
    ico    TEXT PRIMARY KEY,
    name   TEXT NOT NULL DEFAULT '',
    dic    TEXT NOT NULL DEFAULT '',
    street TEXT NOT NULL DEFAULT '',
    zip    TEXT NOT NULL DEFAULT '',
    city   TEXT NOT NULL DEFAULT ''
);
```

*Poznámka:* Starší databázová tabulka `supplier` se v `db::init` nevytváří; modul [src/config.rs](src/config.rs) ji pouze jednorázově čte při prvotní migraci do `config.toml`.

---

## 6. Datový kontrakt `invoice_data.json`

Serializuje se z `TemplateData` ([src/models.rs](src/models.rs)) pomocí `serde_json::to_vec_pretty` do `<templates_dir>/invoice_data.json` a předává se vybrané `.typ` šabloně (pro CLI na disku i přes `with_static_file_resolver` pro vestavěný `TypstEngine`):

| JSON klíč | Typ | Zdroj v Rustu (`TemplateData::from_parts`) | Popis |
| :--- | :--- | :--- | :--- |
| `invoice_number` | `string` | `Invoice.number` | Evidenční číslo faktury (např. `"0001/2026"`) |
| `variable_symbol` | `string` | `Invoice.variable_symbol` | Variabilní symbol bez lomítka (např. `"00012026"`) |
| `issue_date` | `string` | `Invoice.issue_date` | Datum vystavení (`"DD.MM.YYYY"`) |
| `due_date` | `string` | `Invoice.due_date` | Datum splatnosti (`"DD.MM.YYYY"`) |
| `payment_method` | `string` | `Invoice.payment_method` | Forma úhrady (např. `"Převodem"`) |
| `note` | `string` | `Invoice.note` | Úvodní text / víceřádková poznámka |
| `total_without_vat` | `f64` | `round2(Invoice.total_excl_vat)` | Celkový základ daně bez DPH |
| `total_vat` | `f64` | `round2(Invoice.total_vat)` | Celková částka DPH |
| `total_with_vat` | `f64` | `round2(Invoice.total_incl_vat)` | Celková částka k úhradě včetně DPH |
| `supplier.name` | `string` | `Supplier.name` | Název dodavatele |
| `supplier.street` | `string` | `Supplier.street` | Ulice a číslo dodavatele |
| `supplier.zip` | `string` | `Supplier.zip` | PSČ dodavatele |
| `supplier.city` | `string` | `Supplier.city` | Město dodavatele |
| `supplier.ico` | `string` | `Supplier.reg_id` | IČO dodavatele |
| `supplier.dic` | `string` | `Supplier.vat_id` | DIČ dodavatele |
| `supplier.bank_account` | `string` | `Supplier.account_cz` | Tuzemské číslo účtu |
| `supplier.iban` | `string` | `Supplier.iban` | IBAN pro převod a QR platbu |
| `customer.name` | `string` | `Customer.name` | Název odběratele |
| `customer.street` | `string` | `Customer.street` | Ulice odběratele |
| `customer.zip` | `string` | `Customer.zip` | PSČ odběratele |
| `customer.city` | `string` | `Customer.city` | Město odběratele |
| `customer.ico` | `string` | `Customer.reg_id` | IČO odběratele |
| `customer.dic` | `string` | `Customer.vat_id` | DIČ odběratele |
| `items` | `array` | `Vec<TemplateLineItem>` | Položky faktury |
| `items[].description` | `string` | `LineItem.description` | Popis položky |
| `items[].quantity` | `f64` | `round2(LineItem.quantity)` | Množství |
| `items[].unit` | `string` | `LineItem.unit` | Měrná jednotka |
| `items[].unit_price` | `f64` | `round2(LineItem.unit_price)` | Jednotková cena bez DPH |
| `items[].vat_rate` | `u8` | `LineItem.vat_rate` | Sazba DPH v % (`0`, `12`, `21`) |
| `items[].total` | `f64` | `round2(LineItem.total_with_vat())` | Cena řádku včetně DPH |

---

## 7. Klíčové invarianty

- **Synchronní model:** V celém projektu není žádný async runtime; UI smyčka běží na hlavním vlákně přes Slint.
- **Rozdělení perzistence:** Údaje dodavatele, výchozí splatnost (`default_due_days`, výchozí `14`), aktivní šablona (`template`, výchozí `"klasicka.typ"`) a výstupní složka (`output_dir`) jsou uloženy v `config.toml` (se `serde` aliasy `reg_id`, `vat_id`, `account_cz` pro zpětnou kompatibilitu); faktury, položky a adresář odběratelů jsou v SQLite databázi `faktury.db`.
- **Ochrana uživatelských šablon:** `config::ensure_default_templates` zapisuje vestavěné šablony (`templates/faktura.typ` → `klasicka.typ`, `templates/moderni.typ` → `moderni.typ`) do `templates_dir` pouze tehdy, pokud daný soubor na disku ještě neexistuje.
- **Číslování dokladů za kalendářní rok:** `db::next_invoice_number` a `db::next_variable_symbol` vyhledají v `invoices` nejvyšší číselný prefix před `/` u čísel končících `/{year}` a vytvoří následující 4místné pořadí `NNNN/YYYY` a `NNNNYYYY`.
- **Snapshot odběratele na faktuře:** Údaje odběratele se při uložení faktury kopírují přímo do sloupců `customer_*` tabulky `invoices` (nezávisle na záznamu v adresáři `customers`, jehož primárním klíčem je `ico`).
- **Formátování dat a částek:**
  - Data jsou reprezentována jako řetězce `"DD.MM.YYYY"` (UI parser `parse_date` přijímá i `"D.M.YYYY"`).
  - Částky v JSON jsou zaokrouhlené na 2 desetinná místa (`models::round2`) jako `f64`; v UI jsou formátovány funkcí `models::format_amount` s desetinnou čárkou a v PDF šablonách funkcí `fmt(n)` s českým oddělením tisíců nezlomitelnou mezerou a desetinnou čárkou.
  - QR platba SPD 1.0 se v obou vestavěných šablonách (`faktura.typ`, `moderni.typ`) vykreslí přes balíček `@preview/tiaoma:0.3.0`, pokud je `data.supplier.iban` neprázdné.
- **Pojmenování výstupního PDF:** `pdf::pdf_output_path` nahrazuje v čísle faktury `/` za `_` a vrací `<output_dir>/faktura_<safe_name>.pdf` (bez zdvojení prefixu `faktura_`).
