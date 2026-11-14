mod ares;
mod config;
mod db;
mod models;
mod pdf;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use chrono::{Datelike, Local, NaiveDate};
use rusqlite::Connection;
use slint::{Model, VecModel};

use config::AppPaths;
use models::{
    compute_totals, format_amount, Customer, Invoice, LineItem, TemplateData,
};

slint::include_modules!();

fn open_db(paths: &AppPaths) -> rusqlite::Result<Connection> {
    Connection::open(&paths.db_path())
}

fn main() {
    // ── Application paths & configuration ───────────────────────────────
    let paths = config::resolve_paths();
    config::ensure_dirs(&paths).expect("Cannot create application directories");
    config::migrate_legacy_db_if_needed(&paths);

    let app_config = config::load_or_init(&paths).expect("Cannot load application config");
    config::ensure_default_templates(&paths);
    let templates = config::list_templates(&paths);

    let config_state = Rc::new(RefCell::new(app_config.clone()));
    let paths_state = Rc::new(paths.clone());

    // ── Database ────────────────────────────────────────────────────────
    let conn = open_db(&paths).expect("Cannot open database faktury.db");
    db::init(&conn).expect("Cannot initialise database schema");

    let supplier = app_config.to_supplier();

    let year = Local::now().year();
    let initial_number = db::next_invoice_number(&conn, year)
        .unwrap_or_else(|_| format!("0001/{}", year));
    let initial_vs = db::next_variable_symbol(&conn, year)
        .unwrap_or_else(|_| format!("0001{}", year));

    let today = Local::now().date_naive();
    let due = today + chrono::Duration::days(supplier.default_due_days as i64);
    let issue_date = today.format("%d.%m.%Y").to_string();
    let due_date = due.format("%d.%m.%Y").to_string();

    // ── Window ──────────────────────────────────────────────────────────
    let window = AppWindow::new().expect("Cannot create application window");

    // Populate supplier settings tab
    window.set_sup_name(supplier.name.as_str().into());
    window.set_sup_reg_id(supplier.reg_id.as_str().into());
    window.set_sup_vat_id(supplier.vat_id.as_str().into());
    window.set_sup_street(supplier.street.as_str().into());
    window.set_sup_zip(supplier.zip.as_str().into());
    window.set_sup_city(supplier.city.as_str().into());
    window.set_sup_account(supplier.account_cz.as_str().into());
    window.set_sup_iban(supplier.iban.as_str().into());
    window.set_sup_due_days(supplier.default_due_days as i32);

    // Populate template & output directory settings
    let tmpl_strings: Vec<slint::SharedString> =
        templates.into_iter().map(|s| s.as_str().into()).collect();
    window.set_available_templates(Rc::new(VecModel::from(tmpl_strings)).into());
    window.set_sup_template(app_config.template.as_str().into());
    window.set_sup_output_dir(app_config.output_dir.as_str().into());

    // Populate new-invoice defaults
    window.set_inv_number(initial_number.as_str().into());
    window.set_inv_variable_symbol(initial_vs.as_str().into());
    window.set_inv_issue_date(issue_date.as_str().into());
    window.set_inv_due_date(due_date.as_str().into());

    // Data models
    let items_model: Rc<VecModel<LineItemUI>> = Rc::new(VecModel::default());
    let history_model: Rc<VecModel<InvoiceRow>> = Rc::new(VecModel::default());
    let editing_invoice_id: Rc<RefCell<Option<i64>>> = Rc::new(RefCell::new(None));
    let saved_customers: Rc<RefCell<Vec<Customer>>> = Rc::new(RefCell::new(Vec::new()));

    window.set_line_items(items_model.clone().into());
    window.set_history(history_model.clone().into());

    populate_history(&conn, &history_model);
    refresh_saved_customers(&conn, &window, &saved_customers);

    // Calendar state
    let cal_target: Rc<RefCell<String>> = Rc::new(RefCell::new("issue".into()));
    let cal_year: Rc<RefCell<i32>> = Rc::new(RefCell::new(today.year()));
    let cal_month: Rc<RefCell<u32>> = Rc::new(RefCell::new(today.month()));
    let cal_selected: Rc<RefCell<Option<NaiveDate>>> = Rc::new(RefCell::new(Some(today)));
    update_calendar_view(&window, today.year(), today.month(), Some(today));

    // ── Callback: fetch ARES ────────────────────────────────────────────
    {
        let weak = window.as_weak();
        window.on_fetch_ares(move |reg_id| {
            let win = weak.unwrap();
            match ares::fetch(reg_id.as_str()) {
                Ok(data) => {
                    win.set_cust_name(data.name.as_str().into());
                    win.set_cust_reg_id(data.reg_id.as_str().into());
                    win.set_cust_vat_id(data.vat_id.as_str().into());
                    win.set_cust_street(data.street.as_str().into());
                    win.set_cust_zip(data.zip.as_str().into());
                    win.set_cust_city(data.city.as_str().into());
                    win.set_ares_status("".into());
                }
                Err(msg) => {
                    win.set_ares_status_is_error(true);
                    win.set_ares_status(msg.as_str().into());
                }
            }
        });
    }

    // ── Callback: select saved customer ─────────────────────────────────
    {
        let weak = window.as_weak();
        let custs = saved_customers.clone();
        window.on_select_saved_customer(move |index| {
            if index <= 0 {
                return;
            }
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            let idx = (index - 1) as usize;
            let list = custs.borrow();
            if let Some(c) = list.get(idx) {
                win.set_cust_name(c.name.as_str().into());
                win.set_cust_reg_id(c.reg_id.as_str().into());
                win.set_cust_vat_id(c.vat_id.as_str().into());
                win.set_cust_street(c.street.as_str().into());
                win.set_cust_zip(c.zip.as_str().into());
                win.set_cust_city(c.city.as_str().into());
                win.set_ares_status("".into());
            }
        });
    }

    // ── Callback: manually save customer ────────────────────────────────
    {
        let weak = window.as_weak();
        let custs = saved_customers.clone();
        let p = paths_state.clone();
        window.on_save_customer(move || {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            let customer = Customer {
                name: win.get_cust_name().to_string(),
                reg_id: win.get_cust_reg_id().to_string(),
                vat_id: win.get_cust_vat_id().to_string(),
                street: win.get_cust_street().to_string(),
                zip: win.get_cust_zip().to_string(),
                city: win.get_cust_city().to_string(),
            };
            if customer.reg_id.trim().is_empty() {
                win.set_ares_status_is_error(true);
                win.set_ares_status("Zadejte IČO pro uložení firmy".into());
                return;
            }
            let conn = match open_db(&p) {
                Ok(c) => c,
                Err(e) => {
                    win.set_ares_status_is_error(true);
                    win.set_ares_status(format!("Chyba DB: {}", e).as_str().into());
                    return;
                }
            };
            match db::save_customer(&conn, &customer) {
                Ok(_) => {
                    refresh_saved_customers(&conn, &win, &custs);
                    win.set_ares_status_is_error(false);
                    win.set_ares_status("✓ Firma uložena".into());

                    let list = custs.borrow();
                    if let Some(pos) = list.iter().position(|c| c.reg_id == customer.reg_id) {
                        win.set_selected_customer_index((pos + 1) as i32);
                    }
                }
                Err(e) => {
                    win.set_ares_status_is_error(true);
                    win.set_ares_status(format!("Chyba DB: {}", e).as_str().into());
                }
            }
        });
    }

    // ── Callback: delete saved customer ─────────────────────────────────
    {
        let weak = window.as_weak();
        let custs = saved_customers.clone();
        let p = paths_state.clone();
        window.on_delete_saved_customer(move |index| {
            if index <= 0 {
                return;
            }
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            let idx = (index - 1) as usize;
            let reg_id = {
                let list = custs.borrow();
                list.get(idx).map(|c| c.reg_id.clone())
            };

            if let Some(ico) = reg_id {
                let conn = match open_db(&p) {
                    Ok(c) => c,
                    Err(e) => {
                        win.set_ares_status_is_error(true);
                        win.set_ares_status(format!("Chyba DB: {}", e).as_str().into());
                        return;
                    }
                };
                match db::delete_customer(&conn, &ico) {
                    Ok(_) => {
                        refresh_saved_customers(&conn, &win, &custs);
                        win.set_selected_customer_index(0);
                        win.set_ares_status_is_error(false);
                        win.set_ares_status("✓ Odběratel smazán".into());
                    }
                    Err(e) => {
                        win.set_ares_status_is_error(true);
                        win.set_ares_status(format!("Chyba DB: {}", e).as_str().into());
                    }
                }
            }
        });
    }

    // ── Callback: issue date edited ─────────────────────────────────────
    {
        let weak = window.as_weak();
        window.on_issue_date_edited(move |val| {
            if let Some(win) = weak.upgrade() {
                update_due_date_from_issue(&win, val.as_str());
            }
        });
    }

    // ── Callback: open calendar popup ───────────────────────────────────
    {
        let weak = window.as_weak();
        let target_ref = cal_target.clone();
        let year_ref = cal_year.clone();
        let month_ref = cal_month.clone();
        let sel_ref = cal_selected.clone();
        window.on_open_calendar(move |target, current_date_str| {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            *target_ref.borrow_mut() = target.to_string();

            let parsed = parse_date(current_date_str.as_str());
            let (y, m) = match parsed {
                Some(d) => (d.year(), d.month()),
                None => {
                    let now = Local::now().date_naive();
                    (now.year(), now.month())
                }
            };
            *year_ref.borrow_mut() = y;
            *month_ref.borrow_mut() = m;
            *sel_ref.borrow_mut() = parsed;

            update_calendar_view(&win, y, m, parsed);
        });
    }

    // ── Callback: calendar navigation ───────────────────────────────────
    {
        let weak = window.as_weak();
        let year_ref = cal_year.clone();
        let month_ref = cal_month.clone();
        let sel_ref = cal_selected.clone();
        window.on_prev_calendar_month(move || {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            let mut y = *year_ref.borrow();
            let mut m = *month_ref.borrow();
            if m == 1 {
                m = 12;
                y -= 1;
            } else {
                m -= 1;
            }
            *year_ref.borrow_mut() = y;
            *month_ref.borrow_mut() = m;
            update_calendar_view(&win, y, m, *sel_ref.borrow());
        });
    }

    {
        let weak = window.as_weak();
        let year_ref = cal_year.clone();
        let month_ref = cal_month.clone();
        let sel_ref = cal_selected.clone();
        window.on_next_calendar_month(move || {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            let mut y = *year_ref.borrow();
            let mut m = *month_ref.borrow();
            if m == 12 {
                m = 1;
                y += 1;
            } else {
                m += 1;
            }
            *year_ref.borrow_mut() = y;
            *month_ref.borrow_mut() = m;
            update_calendar_view(&win, y, m, *sel_ref.borrow());
        });
    }

    // ── Callback: select calendar day ───────────────────────────────────
    {
        let weak = window.as_weak();
        let target_ref = cal_target.clone();
        let sel_ref = cal_selected.clone();
        window.on_select_calendar_day(move |date_str| {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            let s = date_str.to_string();
            let target = target_ref.borrow().clone();

            if target == "issue" {
                win.set_inv_issue_date(s.as_str().into());
                update_due_date_from_issue(&win, s.as_str());
            } else {
                win.set_inv_due_date(s.as_str().into());
            }

            *sel_ref.borrow_mut() = parse_date(s.as_str());
        });
    }

    // ── Callback: add line item ─────────────────────────────────────────
    {
        let model = items_model.clone();
        let weak = window.as_weak();
        window.on_add_line_item(move || {
            model.push(LineItemUI {
                description: "".into(),
                quantity: 1.0,
                unit: "ks".into(),
                unit_price: 0.0,
                vat_rate: 21,
            });
            if let Some(win) = weak.upgrade() {
                recalculate_totals(&win, &model);
            }
        });
    }

    // ── Callback: remove line item ──────────────────────────────────────
    {
        let model = items_model.clone();
        let weak = window.as_weak();
        window.on_remove_line_item(move |index| {
            if (index as usize) < model.row_count() {
                model.remove(index as usize);
            }
            if let Some(win) = weak.upgrade() {
                recalculate_totals(&win, &model);
            }
        });
    }

    // ── Callback: recalculate totals ────────────────────────────────────
    {
        let model = items_model.clone();
        let weak = window.as_weak();
        window.on_recalculate_totals(move || {
            if let Some(win) = weak.upgrade() {
                recalculate_totals(&win, &model);
            }
        });
    }

    // ── Callback: save supplier & config ────────────────────────────────
    {
        let weak = window.as_weak();
        let cfg_ref = config_state.clone();
        let p = paths_state.clone();
        window.on_save_supplier(move || {
            let win = weak.unwrap();
            let mut cfg = cfg_ref.borrow_mut();
            cfg.name = win.get_sup_name().to_string();
            cfg.ico = win.get_sup_reg_id().to_string();
            cfg.dic = win.get_sup_vat_id().to_string();
            cfg.street = win.get_sup_street().to_string();
            cfg.zip = win.get_sup_zip().to_string();
            cfg.city = win.get_sup_city().to_string();
            cfg.bank_account = win.get_sup_account().to_string();
            cfg.iban = win.get_sup_iban().to_string();
            cfg.default_due_days = win.get_sup_due_days().max(1) as u32;
            cfg.template = win.get_sup_template().to_string();
            cfg.output_dir = win.get_sup_output_dir().to_string();

            match config::save_config(&p, &cfg) {
                Ok(_) => win.set_supplier_status("✓ Nastavení uloženo".into()),
                Err(e) => win.set_supplier_status(
                    format!("Chyba: {}", e).as_str().into(),
                ),
            }
        });
    }

    // ── Callback: save invoice and generate PDF ─────────────────────────
    {
        let weak = window.as_weak();
        let items_ref = items_model.clone();
        let history_ref = history_model.clone();
        let editing_id = editing_invoice_id.clone();
        let custs_ref = saved_customers.clone();
        let cfg_ref = config_state.clone();
        let p = paths_state.clone();

        window.on_save_invoice(move || {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            win.set_invoice_status("Generuji PDF…".into());

            let conn = match open_db(&p) {
                Ok(c) => c,
                Err(e) => {
                    win.set_invoice_status(format!("Chyba DB: {}", e).as_str().into());
                    return;
                }
            };

            let cfg = cfg_ref.borrow().clone();
            let supplier = cfg.to_supplier();

            let customer = Customer {
                name: win.get_cust_name().to_string(),
                reg_id: win.get_cust_reg_id().to_string(),
                vat_id: win.get_cust_vat_id().to_string(),
                street: win.get_cust_street().to_string(),
                zip: win.get_cust_zip().to_string(),
                city: win.get_cust_city().to_string(),
            };

            // Automatically save/update customer in customers table if reg_id is non-empty
            if !customer.reg_id.trim().is_empty() {
                let _ = db::save_customer(&conn, &customer);
                refresh_saved_customers(&conn, &win, &custs_ref);
            }

            let items = collect_items(&items_ref);
            let (excl_vat, vat_amount, incl_vat) = compute_totals(&items);

            let invoice = Invoice {
                id: None,
                number: win.get_inv_number().to_string(),
                variable_symbol: win.get_inv_variable_symbol().to_string(),
                issue_date: win.get_inv_issue_date().to_string(),
                due_date: win.get_inv_due_date().to_string(),
                payment_method: win.get_inv_payment_method().to_string(),
                note: win.get_inv_note().to_string(),
                total_excl_vat: excl_vat,
                total_vat: vat_amount,
                total_incl_vat: incl_vat,
                customer_name: customer.name.clone(),
                pdf_path: None,
            };

            let is_editing = *editing_id.borrow();

            let save_result = match is_editing {
                Some(id) => db::update_invoice(&conn, id, &invoice, &items, &customer).map(|_| id),
                None => db::save_invoice(&conn, &invoice, &items, &customer),
            };

            match save_result {
                Err(e) => {
                    win.set_invoice_status(format!("Chyba DB: {}", e).as_str().into());
                }
                Ok(id) => {
                    let template_data =
                        TemplateData::from_parts(&invoice, &supplier, &customer, &items);
                    let output_dir = Path::new(&cfg.output_dir);
                    let output_path = pdf::pdf_output_path(output_dir, &invoice.number);

                    match pdf::compile(&template_data, &p.templates_dir(), &cfg.template, &output_path) {
                        Ok(_) => {
                            let path_str = output_path.to_string_lossy().to_string();
                            let _ = db::set_invoice_pdf_path(&conn, id, &path_str);
                            win.set_is_editing(false);
                            win.set_invoice_status(if is_editing.is_some() {
                                "✓ Faktura upravena".into()
                            } else {
                                "✓ PDF vygenerováno".into()
                            });
                            pdf::open(&output_path);

                            // Clear editing ID and reset form for new invoice
                            *editing_id.borrow_mut() = None;
                            populate_history(&conn, &history_ref);
                            advance_invoice_number(&conn, &win);
                            reset_invoice_form(&win, &items_ref);

                            // Reset dates for next new invoice
                            let today = Local::now().date_naive();
                            let due = today + chrono::Duration::days(supplier.default_due_days as i64);
                            win.set_inv_issue_date(today.format("%d.%m.%Y").to_string().as_str().into());
                            win.set_inv_due_date(due.format("%d.%m.%Y").to_string().as_str().into());
                            win.set_inv_payment_method("Převodem".into());
                        }
                        Err(e) => {
                            win.set_invoice_status(
                                format!("Chyba PDF: {}", e).as_str().into(),
                            );
                        }
                    }
                }
            }
        });
    }

    // ── Callback: new empty invoice ─────────────────────────────────────
    {
        let weak = window.as_weak();
        let items_ref = items_model.clone();
        let editing_id = editing_invoice_id.clone();
        let cfg_ref = config_state.clone();
        let p = paths_state.clone();

        window.on_new_empty_invoice(move || {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            let conn = match open_db(&p) {
                Ok(c) => c,
                Err(e) => {
                    win.set_invoice_status(format!("Chyba DB: {}", e).as_str().into());
                    return;
                }
            };

            *editing_id.borrow_mut() = None;
            reset_invoice_form(&win, &items_ref);
            advance_invoice_number(&conn, &win);

            let supplier = cfg_ref.borrow().to_supplier();
            let today = Local::now().date_naive();
            let due = today + chrono::Duration::days(supplier.default_due_days as i64);
            win.set_inv_issue_date(today.format("%d.%m.%Y").to_string().as_str().into());
            win.set_inv_due_date(due.format("%d.%m.%Y").to_string().as_str().into());
            win.set_inv_payment_method("Převodem".into());
            win.set_is_editing(false);
            win.set_invoice_status("".into());
        });
    }

    // ── Callback: edit invoice from history ─────────────────────────────
    {
        let weak = window.as_weak();
        let items_ref = items_model.clone();
        let editing_id = editing_invoice_id.clone();
        let p = paths_state.clone();

        window.on_edit_invoice(move |id| {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            let conn = match open_db(&p) {
                Ok(c) => c,
                Err(e) => {
                    win.set_invoice_status(format!("Chyba DB: {}", e).as_str().into());
                    return;
                }
            };

            match db::load_invoice_detail(&conn, id as i64) {
                Ok((invoice, customer, items)) => {
                    *editing_id.borrow_mut() = Some(id as i64);

                    // Populate customer fields
                    win.set_cust_name(customer.name.as_str().into());
                    win.set_cust_reg_id(customer.reg_id.as_str().into());
                    win.set_cust_vat_id(customer.vat_id.as_str().into());
                    win.set_cust_street(customer.street.as_str().into());
                    win.set_cust_zip(customer.zip.as_str().into());
                    win.set_cust_city(customer.city.as_str().into());

                    // Populate invoice metadata
                    win.set_inv_number(invoice.number.as_str().into());
                    win.set_inv_variable_symbol(invoice.variable_symbol.as_str().into());
                    win.set_inv_issue_date(invoice.issue_date.as_str().into());
                    win.set_inv_due_date(invoice.due_date.as_str().into());
                    win.set_inv_payment_method(invoice.payment_method.as_str().into());
                    win.set_inv_note(invoice.note.as_str().into());

                    // Populate line items
                    let uis: Vec<LineItemUI> = items
                        .into_iter()
                        .map(|item| LineItemUI {
                            description: item.description.as_str().into(),
                            quantity: item.quantity as f32,
                            unit: item.unit.as_str().into(),
                            unit_price: item.unit_price as f32,
                            vat_rate: item.vat_rate as i32,
                        })
                        .collect();
                    items_ref.set_vec(uis);

                    // Recalculate totals
                    recalculate_totals(&win, &items_ref);

                    win.set_is_editing(true);
                    win.set_invoice_status("".into());

                    // Switch active tab to "Nová faktura" (index 0)
                    win.set_active_tab(0);
                }
                Err(e) => eprintln!("Cannot load invoice {}: {}", id, e),
            }
        });
    }

    // ── Callback: delete invoice from history ───────────────────────────
    {
        let weak = window.as_weak();
        let hist_model = history_model.clone();
        let items_ref = items_model.clone();
        let editing_id = editing_invoice_id.clone();
        let p = paths_state.clone();

        window.on_delete_invoice(move |id| {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            let conn = match open_db(&p) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Cannot open DB: {}", e);
                    return;
                }
            };

            match db::delete_invoice(&conn, id as i64) {
                Ok(pdf_path) => {
                    if let Some(path) = pdf_path {
                        let _ = std::fs::remove_file(path);
                    }

                    // If the deleted invoice was currently being edited, reset the form
                    let was_editing = *editing_id.borrow() == Some(id as i64);
                    if was_editing {
                        *editing_id.borrow_mut() = None;
                        reset_invoice_form(&win, &items_ref);
                        advance_invoice_number(&conn, &win);
                        win.set_is_editing(false);
                        win.set_invoice_status("".into());
                    }

                    // Auto-refresh history
                    populate_history(&conn, &hist_model);
                }
                Err(e) => eprintln!("Cannot delete invoice {}: {}", id, e),
            }
        });
    }

    // ── Callback: regenerate PDF from history ───────────────────────────
    {
        let weak = window.as_weak();
        let cfg_ref = config_state.clone();
        let p = paths_state.clone();

        window.on_regenerate_pdf(move |id| {
            let conn = match open_db(&p) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Cannot open DB: {}", e);
                    return;
                }
            };

            let cfg = cfg_ref.borrow().clone();
            let supplier = cfg.to_supplier();

            match db::load_invoice_detail(&conn, id as i64) {
                Ok((invoice, customer, items)) => {
                    let template_data =
                        TemplateData::from_parts(&invoice, &supplier, &customer, &items);
                    let output_dir = Path::new(&cfg.output_dir);
                    let output_path = pdf::pdf_output_path(output_dir, &invoice.number);
                    match pdf::compile(&template_data, &p.templates_dir(), &cfg.template, &output_path) {
                        Ok(_) => pdf::open(&output_path),
                        Err(e) => eprintln!("PDF regeneration error: {}", e),
                    }
                }
                Err(e) => eprintln!("Cannot load invoice {}: {}", id, e),
            }
            let _ = weak.unwrap(); // keep weak alive
        });
    }

    // ── Event loop ──────────────────────────────────────────────────────
    window.run().expect("Slint event loop error");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Recomputes invoice totals from the current item model and updates the UI.
fn recalculate_totals(win: &AppWindow, model: &VecModel<LineItemUI>) {
    let items = collect_items(model);
    let (excl, vat, incl) = compute_totals(&items);
    win.set_inv_total_excl_vat(format_amount(excl).as_str().into());
    win.set_inv_total_vat(format_amount(vat).as_str().into());
    win.set_inv_total_incl_vat(format_amount(incl).as_str().into());
}

/// Converts the Slint `VecModel<LineItemUI>` into a `Vec<LineItem>`.
fn collect_items(model: &VecModel<LineItemUI>) -> Vec<LineItem> {
    (0..model.row_count())
        .map(|i| {
            let ui = model.row_data(i).unwrap();
            LineItem {
                description: ui.description.to_string(),
                quantity: ui.quantity as f64,
                unit: ui.unit.to_string(),
                unit_price: ui.unit_price as f64,
                vat_rate: ui.vat_rate as u8,
            }
        })
        .collect()
}

/// Fills the invoice history model from the database.
fn populate_history(conn: &Connection, model: &VecModel<InvoiceRow>) {
    match db::list_invoices(conn) {
        Ok(invoices) => {
            let rows: Vec<InvoiceRow> = invoices
                .into_iter()
                .map(|inv| InvoiceRow {
                    id: inv.id.unwrap_or(0) as i32,
                    number: inv.number.as_str().into(),
                    customer: inv.customer_name.as_str().into(),
                    issue_date: inv.issue_date.as_str().into(),
                    total: format_amount(inv.total_incl_vat).as_str().into(),
                })
                .collect();
            model.set_vec(rows);
        }
        Err(e) => eprintln!("Cannot load invoice history: {}", e),
    }
}

/// Advances the invoice number and variable symbol in the UI after a successful save.
fn advance_invoice_number(conn: &Connection, win: &AppWindow) {
    let year = Local::now().year();
    if let Ok(n) = db::next_invoice_number(conn, year) {
        win.set_inv_number(n.as_str().into());
    }
    if let Ok(vs) = db::next_variable_symbol(conn, year) {
        win.set_inv_variable_symbol(vs.as_str().into());
    }
}

/// Resets the new-invoice form after a successful save.
fn reset_invoice_form(win: &AppWindow, items: &VecModel<LineItemUI>) {
    items.set_vec(vec![]);
    win.set_cust_name("".into());
    win.set_cust_reg_id("".into());
    win.set_cust_vat_id("".into());
    win.set_cust_street("".into());
    win.set_cust_zip("".into());
    win.set_cust_city("".into());
    win.set_inv_note("".into());
    win.set_inv_total_excl_vat("0,00".into());
    win.set_inv_total_vat("0,00".into());
    win.set_inv_total_incl_vat("0,00".into());
    win.set_selected_customer_index(0);
}

/// Refreshes the saved customers list and updates the UI dropdown.
fn refresh_saved_customers(
    conn: &Connection,
    win: &AppWindow,
    cache: &Rc<RefCell<Vec<Customer>>>,
) {
    match db::list_customers(conn) {
        Ok(list) => {
            let mut labels: Vec<slint::SharedString> =
                vec!["-- Vyberte odběratele --".into()];
            for c in &list {
                labels.push(format!("{} ({})", c.name, c.reg_id).as_str().into());
            }
            *cache.borrow_mut() = list;
            win.set_saved_customers(Rc::new(VecModel::from(labels)).into());
        }
        Err(e) => eprintln!("Cannot load customers: {}", e),
    }
}

/// Parses date string in "DD.MM.YYYY" or "D.M.YYYY" format.
fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    if let Ok(d) = NaiveDate::parse_from_str(s, "%d.%m.%Y") {
        return Some(d);
    }
    let parts: Vec<&str> = s.split('.').map(|p| p.trim()).collect();
    if parts.len() == 3 {
        let day: u32 = parts[0].parse().ok()?;
        let month: u32 = parts[1].parse().ok()?;
        let year: i32 = parts[2].parse().ok()?;
        NaiveDate::from_ymd_opt(year, month, day)
    } else {
        None
    }
}

/// Updates the invoice due date when the issue date changes.
fn update_due_date_from_issue(win: &AppWindow, issue_str: &str) {
    if let Some(issue_date) = parse_date(issue_str) {
        let due_days = win.get_sup_due_days().max(1) as i64;
        let due_date = issue_date + chrono::Duration::days(due_days);
        win.set_inv_due_date(due_date.format("%d.%m.%Y").to_string().as_str().into());
    }
}

/// Rebuilds the 42-day calendar model for the specified month/year.
fn update_calendar_view(
    win: &AppWindow,
    year: i32,
    month: u32,
    selected: Option<NaiveDate>,
) {
    let czech_months = [
        "Leden", "Únor", "Březen", "Duben", "Květen", "Červen",
        "Červenec", "Srpen", "Září", "Říjen", "Listopad", "Prosinec",
    ];
    let month_name = czech_months.get(month.saturating_sub(1) as usize).unwrap_or(&"");
    win.set_calendar_title(format!("{} {}", month_name, year).as_str().into());

    let first_day = match NaiveDate::from_ymd_opt(year, month, 1) {
        Some(d) => d,
        None => return,
    };

    let days_before = first_day.weekday().num_days_from_monday();
    let start_date = first_day - chrono::Duration::days(days_before as i64);
    let today = Local::now().date_naive();

    let mut days = Vec::with_capacity(42);
    for i in 0..42 {
        let curr = start_date + chrono::Duration::days(i);
        let in_month = curr.month() == month;
        let is_today = curr == today;
        let is_selected = selected == Some(curr);
        days.push(CalendarDay {
            day_text: curr.day().to_string().as_str().into(),
            date_str: curr.format("%d.%m.%Y").to_string().as_str().into(),
            in_month,
            is_today,
            is_selected,
        });
    }

    let mut weeks = Vec::with_capacity(6);
    for w in 0..6 {
        let base = w * 7;
        weeks.push(CalendarWeek {
            d0: days[base].clone(),
            d1: days[base + 1].clone(),
            d2: days[base + 2].clone(),
            d3: days[base + 3].clone(),
            d4: days[base + 4].clone(),
            d5: days[base + 5].clone(),
            d6: days[base + 6].clone(),
        });
    }

    win.set_calendar_weeks(Rc::new(VecModel::from(weeks)).into());
}
