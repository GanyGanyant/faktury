# Faktury – Nativní desktopová fakturační aplikace

Rychlá, moderní a samostatná desktopová aplikace pro vystavování, správu a tisk faktur v českém podnikatelském prostředí. Aplikace je napsána v jazyce **Rust**, grafické rozhraní běží na **Slint** a sazba tiskově dokonalých PDF dokumentů probíhá přes moderní sázecí systém **Typst**.

---

## Hlavní funkce

- **Automatické načítání údajů z ARES:** Zadejte IČO odběratele a jedním kliknutím načtěte název firmy, DIČ, ulici, město i PSČ přímo ze státního registru ARES.
- **Platba QR kódem (český standard SPD 1.0):** Automatické generování QR platby pro okamžité zaplacení ve všech českých mobilních bankovnictvích podle IBANu, částky a variabilního symbolu.
- **Integrovaný Typst PDF engine:** Špičková typografie bez nutnosti instalovat externí nástroje. Aplikace obsahuje zabudovaný kompilátor Typst s přibalenými fonty *Libertinus Sans* a automatickým fallbackem.
- **Výběr a tvorba vlastních šablon faktur:** Podpora přepínání šablon (`klasicka.typ`, `moderni.typ`) i přidávání vlastních šablon v syntaxi Typst.
- **SQLite databáze & historie vystavených faktur:** Bezpečné lokální úložiště `faktury.db`. Faktury lze zpětně procházet, filtrovat, upravovat, mazat nebo okamžitě znovu exportovat do PDF.
- **Adresář odběratelů:** Možnost ukládat často používané firmy a rychle je vybírat z rozbalovacího menu.
- **Interaktivní kalendář:** Výběr data vystavení i splatnosti přes přehledný kalendář s automatickým dopočtem splatnosti podle nastavení.
- **Živý přepočet součtů:** Okamžitý dynamický výpočet základu daně, částek DPH (21 %, 12 %, 0 %) i celkové částky k úhradě při úpravě položek.
- **Čistá TOML konfigurace:** Nastavení dodavatele, výchozí splatnosti, aktivní šablony i výstupní složky jsou přehledně uloženy v souboru `config.toml`.

---

## Umístění souborů v systému

Aplikace striktně respektuje standardní cesty operačních systémů pro konfiguraci a data:

| Položka | Linux | Windows | macOS |
| :--- | :--- | :--- | :--- |
| **Konfigurace (`config.toml`)** | `~/.config/faktury/config.toml` | `%APPDATA%\faktury\config\config.toml` | `~/Library/Application Support/faktury/config.toml` |
| **Složka šablon (`templates/`)** | `~/.config/faktury/templates/` | `%APPDATA%\faktury\config\templates\` | `~/Library/Application Support/faktury/templates/` |
| **Databáze (`faktury.db`)** | `~/.local/share/faktury/faktury.db` | `%APPDATA%\faktury\data\faktury.db` | `~/Library/Application Support/faktury/faktury.db` |
| **Výchozí složka pro PDF** | `~/Dokumenty/Faktury/` | `%USERPROFILE%\Documents\Faktury\` | `~/Documents/Faktury/` |

> **Poznámka:** Pokud konfigurace `config.toml` při prvním spuštění neexistuje, aplikace automaticky vytvoří výchozí soubor a případně zmigruje starší nastavení dodavatele z existující databáze.

---

## Tvorba vlastních šablon faktur (Typst)

Aplikace při každém generování PDF zapíše data aktuální faktury do souboru `invoice_data.json` v adresáři šablon a předá je vybranému souboru `*.typ`.

### Jak přidat novou šablonu:
1. Otevřete systémovou složku šablon (např. `~/.config/faktury/templates/` na Linuxu).
2. Vytvořte nový soubor s příponou `.typ` (např. `firemni.typ`), nejlépe kopií existující `klasicka.typ` nebo `moderni.typ`.
3. V aplikaci na záložce **Nastavení dodavatele** vyberte novou šablonu v rozbalovací nabídce **Šablona faktury** a uložte nastavení.

### Reference datových polí (`invoice_data.json`)

V Typst šabloně se data načítají pomocí:
```typst
#let data = json("invoice_data.json")
```

Následující tabulka obsahuje kompletní přehled všech dostupných polí:

| Pole v JSON | Datový typ | Popis a příklad |
| :--- | :--- | :--- |
| `invoice_number` | `string` | Číslo faktury (např. `"0001/2026"`) |
| `variable_symbol` | `string` | Variabilní symbol pro platbu bez lomítek (např. `"00012026"`) |
| `issue_date` | `string` | Datum vystavení (např. `"14.11.2026"`) |
| `due_date` | `string` | Datum splatnosti (např. `"28.11.2026"`) |
| `payment_method` | `string` | Forma úhrady (např. `"Převodem"`, `"Hotově"`) |
| `note` | `string` | Úvodní text dokladu nebo poznámka |
| `total_without_vat` | `float` | Celkový základ daně bez DPH v Kč (např. `10000.0`) |
| `total_vat` | `float` | Celková vypočtená daň DPH v Kč (např. `2100.0`) |
| `total_with_vat` | `float` | Celková částka k úhradě včetně DPH v Kč (např. `12100.0`) |
| `supplier.name` | `string` | Název vystavovatele / dodavatelské firmy |
| `supplier.street` | `string` | Ulice a číslo popisné/orientační dodavatele |
| `supplier.zip` | `string` | PSČ dodavatele (např. `"110 00"`) |
| `supplier.city` | `string` | Město dodavatele |
| `supplier.ico` | `string` | IČO dodavatele |
| `supplier.dic` | `string` | DIČ dodavatele (může být prázdný řetězec `""`) |
| `supplier.bank_account` | `string` | Tuzemské číslo účtu (např. `"123456789/0100"`) |
| `supplier.iban` | `string` | Mezinárodní formát účtu IBAN pro platby a QR kód |
| `customer.name` | `string` | Název odběratele / firmy klienta |
| `customer.street` | `string` | Ulice a číslo odběratele |
| `customer.zip` | `string` | PSČ odběratele |
| `customer.city` | `string` | Město odběratele |
| `customer.ico` | `string` | IČO odběratele |
| `customer.dic` | `string` | DIČ odběratele |
| `items` | `array` | Pole položek faktury (viz níže) |
| `items[].description` | `string` | Textový popis položky / služby |
| `items[].quantity` | `float` | Počet / množství (např. `2.5`) |
| `items[].unit` | `string` | Jednotka (např. `"ks"`, `"hod"`, `"m"`) |
| `items[].unit_price` | `float` | Jednotková cena bez DPH |
| `items[].vat_rate` | `integer` | Sazba DPH v procentech (`21`, `12`, `0`) |
| `items[].total` | `float` | Celková cena položky včetně DPH |

### Užitečné pomocné funkce v Typst

#### Formátování částek podle české typografie (mezera mezi tisíci, desetinná čárka):
```typst
#let fmt(n) = {
  let s = str(calc.round(n, digits: 2))
  let parts = s.split(".")
  let int_part = parts.at(0)
  let dec_part = if parts.len() > 1 {
    let d = parts.at(1)
    "," + if d.len() == 1 { d + "0" } else { d }
  } else { "" }
  let chars = int_part.clusters().rev()
  let grouped = ()
  for (i, c) in chars.enumerate() {
    if i > 0 and calc.rem(i, 3) == 0 and c != "-" {
      grouped.push("\u{00A0}")
    }
    grouped.push(c)
  }
  grouped.rev().join() + dec_part
}
```

#### Vykreslení QR Platby (SPD 1.0):
```typst
#import "@preview/tiaoma:0.3.0": qrcode

#if data.supplier.iban != "" [
  #qrcode(
    "SPD*1.0*ACC:" + data.supplier.iban + "*AM:" + str(data.total_with_vat) + "*CC:CZK*X-VS:" + data.variable_symbol,
    options: (scale: 1.8)
  )
]
```

---

## Návod na sestavení a kompilaci

### Požadavky
- [Rust toolchain](https://rustup.rs/) (edice 2024 nebo 2021).
- Běžné systémové knihovny pro GUI a písma (`fontconfig`, `pkg-config`).

### 1. Nativní sestavení (Linux / macOS)

Nainstalujte vývojové balíčky pro váš systém:
- **Ubuntu / Debian:**
  ```bash
  sudo apt update
  sudo apt install pkg-config libfontconfig1-dev
  ```
- **Fedora / RHEL:**
  ```bash
  sudo dnf install pkg-config fontconfig-devel
  ```
- **macOS:**
  Vývojové nástroje Xcode jsou plně dostačující.

Sestavení aplikace v release režimu:
```bash
cargo build --release
```
Spustitelný binární soubor naleznete v `target/release/faktury`.

### 2. Křížová kompilace pro Windows (`x86_64-pc-windows-gnu`)

Aplikaci lze z Linuxu přímo zkompilovat do formátu `.exe` pro Windows pomocí MinGW toolchainu:

1. Přidejte Windows GNU target:
   ```bash
   rustup target add x86_64-pc-windows-gnu
   ```
2. Nainstalujte MinGW překladač:
   - **Ubuntu / Debian:** `sudo apt install mingw-w64`
   - **Fedora:** `sudo dnf install mingw64-gcc`
3. Spusťte kompilaci:
   ```bash
   cargo build --release --target x86_64-pc-windows-gnu
   ```
Výsledná binárka pro Windows se nachází v `target/x86_64-pc-windows-gnu/release/faktury.exe`.

### Spuštění testů
Ověření integrity databáze, výpočtů a kompilace PDF šablon:
```bash
cargo test --bin faktury
```

---

## Licence

Projekt je určen k volnému použití a úpravám.
