mod ares;
mod db;
mod models;
mod pdf;

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{Datelike, Local};
use rusqlite::Connection;
use slint::{Model, VecModel};

use models::{
    compute_totals, format_amount, Customer, Invoice, LineItem, Supplier, TemplateData,
};

slint::include_modules!();

fn main() {
    // ── Database ────────────────────────────────────────────────────────
    let conn = Connection::open("faktury.db")
        .expect("Cannot open database faktury.db");
    db::init(&conn).expect("Cannot initialise database schema");

    let supplier = db::load_supplier(&conn)
        .expect("Cannot read supplier settings")
        .unwrap_or_default();

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

    // Populate new-invoice defaults
    window.set_inv_number(initial_number.as_str().into());
    window.set_inv_variable_symbol(initial_vs.as_str().into());
    window.set_inv_issue_date(issue_date.as_str().into());
    window.set_inv_due_date(due_date.as_str().into());

    // Data models
    let items_model: Rc<VecModel<LineItemUI>> = Rc::new(VecModel::default());
    let history_model: Rc<VecModel<InvoiceRow>> = Rc::new(VecModel::default());
    let editing_invoice_id: Rc<RefCell<Option<i64>>> = Rc::new(RefCell::new(None));

    window.set_line_items(items_model.clone().into());
    window.set_history(history_model.clone().into());

    populate_history(&conn, &history_model);

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
                Err(msg) => win.set_ares_status(msg.as_str().into()),
            }
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

    // ── Callback: save supplier ─────────────────────────────────────────
    {
        let weak = window.as_weak();
        window.on_save_supplier(move || {
            let win = weak.unwrap();
            let supplier = Supplier {
                name: win.get_sup_name().to_string(),
                reg_id: win.get_sup_reg_id().to_string(),
                vat_id: win.get_sup_vat_id().to_string(),
                street: win.get_sup_street().to_string(),
                zip: win.get_sup_zip().to_string(),
                city: win.get_sup_city().to_string(),
                account_cz: win.get_sup_account().to_string(),
                iban: win.get_sup_iban().to_string(),
                default_due_days: win.get_sup_due_days() as u32,
            };
            let conn = Connection::open("faktury.db").unwrap();
            match db::save_supplier(&conn, &supplier) {
                Ok(_) => win.set_supplier_status("✓ Uloženo".into()),
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

        window.on_save_invoice(move || {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            win.set_invoice_status("Generuji PDF…".into());

            let conn = Connection::open("faktury.db").unwrap();
            let supplier = db::load_supplier(&conn)
                .ok()
                .flatten()
                .unwrap_or_default();

            let customer = Customer {
                name: win.get_cust_name().to_string(),
                reg_id: win.get_cust_reg_id().to_string(),
                vat_id: win.get_cust_vat_id().to_string(),
                street: win.get_cust_street().to_string(),
                zip: win.get_cust_zip().to_string(),
                city: win.get_cust_city().to_string(),
            };

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
                    let output_path = pdf::pdf_output_path(&invoice.number);

                    match pdf::compile(&template_data, &output_path) {
                        Ok(_) => {
                            let path_str = output_path.to_string_lossy().to_string();
                            let _ = db::set_invoice_pdf_path(&conn, id, &path_str);
                            win.set_invoice_status(if is_editing.is_some() {
                                "✓ Faktura upravena a PDF vygenerováno".into()
                            } else {
                                "✓ PDF vygenerováno a otevřeno".into()
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

        window.on_new_empty_invoice(move || {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            let conn = match Connection::open("faktury.db") {
                Ok(c) => c,
                Err(e) => {
                    win.set_invoice_status(format!("Chyba DB: {}", e).as_str().into());
                    return;
                }
            };

            *editing_id.borrow_mut() = None;
            reset_invoice_form(&win, &items_ref);
            advance_invoice_number(&conn, &win);

            let supplier = db::load_supplier(&conn).ok().flatten().unwrap_or_default();
            let today = Local::now().date_naive();
            let due = today + chrono::Duration::days(supplier.default_due_days as i64);
            win.set_inv_issue_date(today.format("%d.%m.%Y").to_string().as_str().into());
            win.set_inv_due_date(due.format("%d.%m.%Y").to_string().as_str().into());
            win.set_inv_payment_method("Převodem".into());
            win.set_invoice_status("".into());
        });
    }

    // ── Callback: edit invoice from history ─────────────────────────────
    {
        let weak = window.as_weak();
        let items_ref = items_model.clone();
        let editing_id = editing_invoice_id.clone();

        window.on_edit_invoice(move |id| {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            let conn = match Connection::open("faktury.db") {
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

                    win.set_invoice_status(
                        format!("Upravujete fakturu č. {}", invoice.number).as_str().into(),
                    );

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

        window.on_delete_invoice(move |id| {
            let win = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };

            let conn = match Connection::open("faktury.db") {
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
        window.on_regenerate_pdf(move |id| {
            let conn = Connection::open("faktury.db").unwrap();
            let supplier = db::load_supplier(&conn)
                .ok()
                .flatten()
                .unwrap_or_default();

            match db::load_invoice_detail(&conn, id as i64) {
                Ok((invoice, customer, items)) => {
                    let template_data =
                        TemplateData::from_parts(&invoice, &supplier, &customer, &items);
                    let output_path = pdf::pdf_output_path(&invoice.number);
                    match pdf::compile(&template_data, &output_path) {
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
}
