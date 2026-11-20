---
trigger: glob
globs: "ui/**/*.slint, src/main.rs"
description: Slint UI separation, Czech UI labels, non-blocking I/O off the main thread, and safe Weak::upgrade.
---

# UI & Main Event Loop Rules

1. **Oddělení UI a aplikační logiky**
   - Rozložení, styly a komponenty drž v `ui/` (`.slint`), obsluhu callbacků a práci se `slint::VecModel` v Rustu.
   - Veškeré uživatelsky viditelné popisky, tlačítka, hlášky a chybové stavy musí být **česky** (kódové identifikátory a komentáře anglicky).

2. **Neblokující UI vlákno**
   - Síťové požadavky (ARES) a náročné I/O / výpočetní operace (kompilace Typst PDF) nespouštěj blokujícím způsobem přímo na hlavním vlákně Slintu.
   - Používej standardní OS vlákno (`std::thread::spawn`) a návrat výsledku do UI přes `slint::invoke_from_event_loop` (bez Tokio / async runtime).

3. **Bezpečná práce s `slint::Weak`**
   - V callbacích i ve vláknech předávaných do `invoke_from_event_loop` vždy používej bezpečné `weak.upgrade()` s explicitním ošetřením `None` větve; **nikdy nevolej `.unwrap()`** na `Weak`.
