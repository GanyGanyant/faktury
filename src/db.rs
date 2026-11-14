use rusqlite::{Connection, OptionalExtension, Result, params};

use crate::models::{Customer, Invoice, LineItem};

// ---------------------------------------------------------------------------
// Schema initialisation
// ---------------------------------------------------------------------------

pub fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch("
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
    ")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Customers
// ---------------------------------------------------------------------------

pub fn save_customer(conn: &Connection, c: &Customer) -> Result<()> {
    if c.reg_id.trim().is_empty() {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO customers (ico, name, dic, street, zip, city)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(ico) DO UPDATE SET
             name   = excluded.name,
             dic    = excluded.dic,
             street = excluded.street,
             zip    = excluded.zip,
             city   = excluded.city",
        params![
            c.reg_id.trim(),
            c.name.trim(),
            c.vat_id.trim(),
            c.street.trim(),
            c.zip.trim(),
            c.city.trim(),
        ],
    )?;
    Ok(())
}

pub fn list_customers(conn: &Connection) -> Result<Vec<Customer>> {
    let mut stmt = conn.prepare(
        "SELECT ico, name, dic, street, zip, city
         FROM customers
         ORDER BY name COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Customer {
            reg_id: row.get(0)?,
            name: row.get(1)?,
            vat_id: row.get(2)?,
            street: row.get(3)?,
            zip: row.get(4)?,
            city: row.get(5)?,
        })
    })?;
    let mut customers = Vec::new();
    for c in rows {
        customers.push(c?);
    }
    Ok(customers)
}

pub fn delete_customer(conn: &Connection, ico: &str) -> Result<()> {
    conn.execute("DELETE FROM customers WHERE ico = ?1", params![ico])?;
    Ok(())
}


// ---------------------------------------------------------------------------
// Invoice numbering
// ---------------------------------------------------------------------------

fn max_invoice_seq_for_year(conn: &Connection, year: i32) -> Result<i64> {
    let pattern = format!("%/{}", year);
    let mut stmt = conn.prepare("SELECT number FROM invoices WHERE number LIKE ?1")?;
    let rows = stmt.query_map(params![pattern], |row| row.get::<_, String>(0))?;
    let mut max_seq: i64 = 0;
    for r in rows {
        if let Ok(num) = r {
            if let Some(prefix) = num.split('/').next() {
                if let Ok(val) = prefix.parse::<i64>() {
                    if val > max_seq {
                        max_seq = val;
                    }
                }
            }
        }
    }
    Ok(max_seq)
}

/// Returns the next invoice number in `NNNN/YYYY` format.
pub fn next_invoice_number(conn: &Connection, year: i32) -> Result<String> {
    let n = max_invoice_seq_for_year(conn, year)?;
    Ok(format!("{:04}/{}", n + 1, year))
}

/// Returns the next variable symbol (invoice number without slash).
pub fn next_variable_symbol(conn: &Connection, year: i32) -> Result<String> {
    let n = max_invoice_seq_for_year(conn, year)?;
    Ok(format!("{:04}{}", n + 1, year))
}

// ---------------------------------------------------------------------------
// Invoice persistence
// ---------------------------------------------------------------------------

pub fn save_invoice(
    conn: &Connection,
    inv: &Invoice,
    items: &[LineItem],
    customer: &Customer,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO invoices (
            number, variable_symbol, issue_date, due_date,
            payment_method, note,
            total_excl_vat, total_vat, total_incl_vat,
            customer_name, customer_reg_id, customer_vat_id,
            customer_street, customer_zip, customer_city
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![
            inv.number, inv.variable_symbol, inv.issue_date, inv.due_date,
            inv.payment_method, inv.note,
            inv.total_excl_vat, inv.total_vat, inv.total_incl_vat,
            customer.name, customer.reg_id, customer.vat_id,
            customer.street, customer.zip, customer.city
        ],
    )?;
    let invoice_id = conn.last_insert_rowid();

    for item in items {
        conn.execute(
            "INSERT INTO line_items
                (invoice_id, description, quantity, unit, unit_price, vat_rate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                invoice_id, item.description, item.quantity,
                item.unit, item.unit_price, item.vat_rate as i64
            ],
        )?;
    }

    Ok(invoice_id)
}

pub fn update_invoice(
    conn: &Connection,
    id: i64,
    inv: &Invoice,
    items: &[LineItem],
    customer: &Customer,
) -> Result<()> {
    conn.execute(
        "UPDATE invoices SET
            number = ?1,
            variable_symbol = ?2,
            issue_date = ?3,
            due_date = ?4,
            payment_method = ?5,
            note = ?6,
            total_excl_vat = ?7,
            total_vat = ?8,
            total_incl_vat = ?9,
            customer_name = ?10,
            customer_reg_id = ?11,
            customer_vat_id = ?12,
            customer_street = ?13,
            customer_zip = ?14,
            customer_city = ?15
         WHERE id = ?16",
        params![
            inv.number, inv.variable_symbol, inv.issue_date, inv.due_date,
            inv.payment_method, inv.note,
            inv.total_excl_vat, inv.total_vat, inv.total_incl_vat,
            customer.name, customer.reg_id, customer.vat_id,
            customer.street, customer.zip, customer.city,
            id
        ],
    )?;

    conn.execute(
        "DELETE FROM line_items WHERE invoice_id = ?1",
        params![id],
    )?;

    for item in items {
        conn.execute(
            "INSERT INTO line_items
                (invoice_id, description, quantity, unit, unit_price, vat_rate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id, item.description, item.quantity,
                item.unit, item.unit_price, item.vat_rate as i64
            ],
        )?;
    }

    Ok(())
}

pub fn delete_invoice(conn: &Connection, id: i64) -> Result<Option<String>> {
    let pdf_path: Option<String> = conn
        .query_row(
            "SELECT pdf_path FROM invoices WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;

    conn.execute("DELETE FROM line_items WHERE invoice_id = ?1", params![id])?;
    conn.execute("DELETE FROM invoices WHERE id = ?1", params![id])?;

    Ok(pdf_path)
}

pub fn set_invoice_pdf_path(conn: &Connection, id: i64, path: &str) -> Result<()> {
    conn.execute(
        "UPDATE invoices SET pdf_path = ?1 WHERE id = ?2",
        params![path, id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Invoice queries
// ---------------------------------------------------------------------------

pub fn list_invoices(conn: &Connection) -> Result<Vec<Invoice>> {
    let mut stmt = conn.prepare(
        "SELECT id, number, variable_symbol, issue_date, due_date,
                payment_method, note,
                total_excl_vat, total_vat, total_incl_vat,
                customer_name, pdf_path
         FROM invoices ORDER BY id DESC",
    )?;

    stmt.query_map([], |row| {
        Ok(Invoice {
            id: Some(row.get(0)?),
            number: row.get(1)?,
            variable_symbol: row.get(2)?,
            issue_date: row.get(3)?,
            due_date: row.get(4)?,
            payment_method: row.get(5)?,
            note: row.get(6)?,
            total_excl_vat: row.get(7)?,
            total_vat: row.get(8)?,
            total_incl_vat: row.get(9)?,
            customer_name: row.get(10)?,
            pdf_path: row.get(11)?,
        })
    })?
    .collect()
}

pub fn load_invoice_detail(
    conn: &Connection,
    id: i64,
) -> Result<(Invoice, Customer, Vec<LineItem>)> {
    let (invoice, customer) = conn.query_row(
        "SELECT id, number, variable_symbol, issue_date, due_date,
                payment_method, note,
                total_excl_vat, total_vat, total_incl_vat,
                customer_name, pdf_path,
                customer_reg_id, customer_vat_id,
                customer_street, customer_zip, customer_city
         FROM invoices WHERE id = ?1",
        params![id],
        |row| {
            let invoice = Invoice {
                id: Some(row.get(0)?),
                number: row.get(1)?,
                variable_symbol: row.get(2)?,
                issue_date: row.get(3)?,
                due_date: row.get(4)?,
                payment_method: row.get(5)?,
                note: row.get(6)?,
                total_excl_vat: row.get(7)?,
                total_vat: row.get(8)?,
                total_incl_vat: row.get(9)?,
                customer_name: row.get(10)?,
                pdf_path: row.get(11)?,
            };
            let customer = Customer {
                name: row.get(10)?,
                reg_id: row.get(12)?,
                vat_id: row.get(13)?,
                street: row.get(14)?,
                zip: row.get(15)?,
                city: row.get(16)?,
            };
            Ok((invoice, customer))
        },
    )?;

    let mut stmt = conn.prepare(
        "SELECT description, quantity, unit, unit_price, vat_rate
         FROM line_items WHERE invoice_id = ?1 ORDER BY id",
    )?;

    let items: Result<Vec<LineItem>> = stmt
        .query_map(params![id], |row| {
            Ok(LineItem {
                description: row.get(0)?,
                quantity: row.get(1)?,
                unit: row.get(2)?,
                unit_price: row.get(3)?,
                vat_rate: row.get::<_, i64>(4)? as u8,
            })
        })?
        .collect();

    Ok((invoice, customer, items?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoice_crud_and_numbering() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();

        let initial_number = next_invoice_number(&conn, 2026).unwrap();
        assert_eq!(initial_number, "0001/2026");

        let customer = Customer {
            name: "Customer 1".into(),
            reg_id: "11111111".into(),
            vat_id: "CZ11111111".into(),
            street: "Street 1".into(),
            zip: "100 00".into(),
            city: "City 1".into(),
        };

        let items = vec![LineItem {
            description: "Service A".into(),
            quantity: 2.0,
            unit: "ks".into(),
            unit_price: 100.0,
            vat_rate: 21,
        }];

        let invoice = Invoice {
            id: None,
            number: "0001/2026".into(),
            variable_symbol: "00012026".into(),
            issue_date: "14.11.2026".into(),
            due_date: "28.11.2026".into(),
            payment_method: "Převodem".into(),
            note: "First note".into(),
            total_excl_vat: 200.0,
            total_vat: 42.0,
            total_incl_vat: 242.0,
            customer_name: customer.name.clone(),
            pdf_path: Some("invoices/0001_2026.pdf".into()),
        };

        let id = save_invoice(&conn, &invoice, &items, &customer).unwrap();
        set_invoice_pdf_path(&conn, id, "invoices/0001_2026.pdf").unwrap();

        let next_number = next_invoice_number(&conn, 2026).unwrap();
        assert_eq!(next_number, "0002/2026");

        // Test update
        let updated_invoice = Invoice {
            note: "Updated note".into(),
            total_excl_vat: 300.0,
            ..invoice
        };
        let updated_items = vec![
            LineItem {
                description: "Service B".into(),
                quantity: 3.0,
                unit: "ks".into(),
                unit_price: 100.0,
                vat_rate: 21,
            },
        ];

        update_invoice(&conn, id, &updated_invoice, &updated_items, &customer).unwrap();
        let (detail_inv, _, detail_items) = load_invoice_detail(&conn, id).unwrap();
        assert_eq!(detail_inv.note, "Updated note");
        assert_eq!(detail_items.len(), 1);
        assert_eq!(detail_items[0].description, "Service B");

        // Test delete
        let pdf_path = delete_invoice(&conn, id).unwrap();
        assert_eq!(pdf_path, Some("invoices/0001_2026.pdf".into()));
        let list = list_invoices(&conn).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_customer_crud() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();

        let customer = Customer {
            name: "ACME Corp".into(),
            reg_id: "12345678".into(),
            vat_id: "CZ12345678".into(),
            street: "Main St 1".into(),
            zip: "110 00".into(),
            city: "Prague".into(),
        };
        save_customer(&conn, &customer).unwrap();

        let list = list_customers(&conn).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "ACME Corp");
        assert_eq!(list[0].reg_id, "12345678");

        // Test upsert update
        let updated = Customer {
            name: "ACME Corp s.r.o.".into(),
            ..customer
        };
        save_customer(&conn, &updated).unwrap();
        let list2 = list_customers(&conn).unwrap();
        assert_eq!(list2.len(), 1);
        assert_eq!(list2[0].name, "ACME Corp s.r.o.");

        // Test delete
        delete_customer(&conn, "12345678").unwrap();
        let list3 = list_customers(&conn).unwrap();
        assert!(list3.is_empty());
    }
}
