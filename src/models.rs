use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Core domain types
// ---------------------------------------------------------------------------

/// Represents the invoicing party (supplier / issuer of the invoice).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Supplier {
    /// Company or trade name.
    pub name: String,
    /// Business registration number (IČO).
    pub reg_id: String,
    /// VAT identification number (DIČ).
    pub vat_id: String,
    /// Street and house / orientation number.
    pub street: String,
    /// Postal code (ZIP).
    pub zip: String,
    /// City / municipality name.
    pub city: String,
    /// Domestic bank account number (CZ format).
    pub account_cz: String,
    /// International Bank Account Number (IBAN).
    pub iban: String,
    /// Default payment due duration in days.
    pub default_due_days: u32,
}

/// Represents the client / recipient of the invoice.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Customer {
    /// Company or person name.
    pub name: String,
    /// Business registration number (IČO).
    pub reg_id: String,
    /// VAT identification number (DIČ).
    pub vat_id: String,
    /// Street address.
    pub street: String,
    /// Postal code (ZIP).
    pub zip: String,
    /// City / municipality.
    pub city: String,
}

/// A single itemized billing row within an invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    /// Description of the goods or service.
    pub description: String,
    /// Quantity / amount.
    pub quantity: f64,
    /// Unit of measure (e.g., "ks", "hod", "m").
    pub unit: String,
    /// Unit price excluding VAT.
    pub unit_price: f64,
    /// VAT percentage rate (e.g. 0, 12, or 21).
    pub vat_rate: u8,
}

impl LineItem {
    /// Returns the subtotal price excluding VAT (`quantity * unit_price`).
    pub fn subtotal(&self) -> f64 {
        self.quantity * self.unit_price
    }

    /// Returns the total price including VAT for this line item.
    pub fn total_with_vat(&self) -> f64 {
        self.subtotal() * (1.0 + self.vat_rate as f64 / 100.0)
    }
}

impl Default for LineItem {
    fn default() -> Self {
        Self {
            description: String::new(),
            quantity: 1.0,
            unit: "ks".to_string(),
            unit_price: 0.0,
            vat_rate: 21,
        }
    }
}

/// Represents an invoice record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Invoice {
    /// Database primary key (`None` for unpersisted invoices).
    pub id: Option<i64>,
    /// Sequential invoice number (e.g., "0001/2026").
    pub number: String,
    /// Variable symbol for bank transfers (e.g., "00012026").
    pub variable_symbol: String,
    /// Issue date formatted as "DD.MM.YYYY".
    pub issue_date: String,
    /// Due date formatted as "DD.MM.YYYY".
    pub due_date: String,
    /// Payment method description (e.g., "Převodem").
    pub payment_method: String,
    /// Introductory text or note displayed on the invoice.
    pub note: String,
    /// Sum of all line item subtotals excluding VAT.
    pub total_excl_vat: f64,
    /// Total calculated VAT across all line items.
    pub total_vat: f64,
    /// Total invoice sum including VAT.
    pub total_incl_vat: f64,
    /// Cached customer name for quick history listings.
    pub customer_name: String,
    /// Path to the generated PDF file, if compiled.
    pub pdf_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Typst template JSON schema  (must match keys in templates/faktura.typ)
// ---------------------------------------------------------------------------

/// Complete data structure serialized into `invoice_data.json` for Typst rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateData {
    /// Formatted invoice number (e.g., "0001/2026").
    pub invoice_number: String,
    /// Variable symbol without slash (e.g., "00012026").
    pub variable_symbol: String,
    /// Date of invoice issue (e.g., "14.11.2026").
    pub issue_date: String,
    /// Date of invoice payment due (e.g., "28.11.2026").
    pub due_date: String,
    /// Payment method string (e.g., "Převodem").
    pub payment_method: String,
    /// Introductory note or remark.
    pub note: String,
    /// Grand total sum excluding VAT.
    pub total_without_vat: f64,
    /// Grand total VAT amount.
    pub total_vat: f64,
    /// Grand total payable amount including VAT.
    pub total_with_vat: f64,
    /// Supplier details block.
    pub supplier: TemplateSupplier,
    /// Customer details block.
    pub customer: TemplateCustomer,
    /// Array of line items.
    pub items: Vec<TemplateLineItem>,
}

/// Supplier block matching the Typst template schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSupplier {
    /// Supplier company or name.
    pub name: String,
    /// Street address.
    pub street: String,
    /// Postal code (ZIP).
    pub zip: String,
    /// City / municipality.
    pub city: String,
    /// Business registration ID (IČO).
    pub ico: String,
    /// Tax / VAT registration ID (DIČ).
    pub dic: String,
    /// Domestic bank account number.
    pub bank_account: String,
    /// IBAN for international transfers and QR payments.
    pub iban: String,
}

/// Customer block matching the Typst template schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCustomer {
    /// Customer company or name.
    pub name: String,
    /// Street address.
    pub street: String,
    /// Postal code (ZIP).
    pub zip: String,
    /// City / municipality.
    pub city: String,
    /// Business registration ID (IČO).
    pub ico: String,
    /// Tax / VAT registration ID (DIČ).
    pub dic: String,
}

/// Item row matching the Typst template schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateLineItem {
    /// Item description.
    pub description: String,
    /// Quantity.
    pub quantity: f64,
    /// Measurement unit.
    pub unit: String,
    /// Unit price without VAT.
    pub unit_price: f64,
    /// VAT percentage rate (e.g. 21).
    pub vat_rate: u8,
    /// Total price including VAT for this row.
    pub total: f64,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

impl TemplateData {
    /// Assembles a `TemplateData` instance from invoice, supplier, customer, and item parts.
    pub fn from_parts(
        invoice: &Invoice,
        supplier: &Supplier,
        customer: &Customer,
        items: &[LineItem],
    ) -> Self {
        let template_items = items
            .iter()
            .map(|item| TemplateLineItem {
                description: item.description.clone(),
                quantity: round2(item.quantity),
                unit: item.unit.clone(),
                unit_price: round2(item.unit_price),
                vat_rate: item.vat_rate,
                total: round2(item.total_with_vat()),
            })
            .collect();

        Self {
            invoice_number: invoice.number.clone(),
            variable_symbol: invoice.variable_symbol.clone(),
            issue_date: invoice.issue_date.clone(),
            due_date: invoice.due_date.clone(),
            payment_method: invoice.payment_method.clone(),
            note: invoice.note.clone(),
            total_without_vat: round2(invoice.total_excl_vat),
            total_vat: round2(invoice.total_vat),
            total_with_vat: round2(invoice.total_incl_vat),
            supplier: TemplateSupplier {
                name: supplier.name.clone(),
                street: supplier.street.clone(),
                zip: supplier.zip.clone(),
                city: supplier.city.clone(),
                ico: supplier.reg_id.clone(),
                dic: supplier.vat_id.clone(),
                bank_account: supplier.account_cz.clone(),
                iban: supplier.iban.clone(),
            },
            customer: TemplateCustomer {
                name: customer.name.clone(),
                street: customer.street.clone(),
                zip: customer.zip.clone(),
                city: customer.city.clone(),
                ico: customer.reg_id.clone(),
                dic: customer.vat_id.clone(),
            },
            items: template_items,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Formats a monetary number with two decimal places and a comma as the decimal separator (Czech format).
pub fn format_amount(value: f64) -> String {
    format!("{:.2}", round2(value)).replace('.', ",")
}

/// Rounds a floating-point number to two decimal places.
pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Computes totals (`(total_excl_vat, total_vat, total_incl_vat)`) across all given line items.
pub fn compute_totals(items: &[LineItem]) -> (f64, f64, f64) {
    let excl_vat: f64 = items.iter().map(|i| i.subtotal()).sum();
    let incl_vat: f64 = items.iter().map(|i| i.total_with_vat()).sum();
    let vat_amount = incl_vat - excl_vat;
    (round2(excl_vat), round2(vat_amount), round2(incl_vat))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_approx_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn test_round2() {
        assert_approx_eq(round2(1.234), 1.23);
        assert_approx_eq(round2(1.235), 1.24);
        assert_approx_eq(round2(1.236), 1.24);
        assert_approx_eq(round2(0.0), 0.0);
        assert_approx_eq(round2(-1.234), -1.23);
        assert_approx_eq(round2(-1.235), -1.24);
        assert_approx_eq(round2(0.1 + 0.2), 0.3);
    }

    #[test]
    fn test_format_amount() {
        assert_eq!(format_amount(100.0), "100,00");
        assert_eq!(format_amount(1234.5), "1234,50");
        assert_eq!(format_amount(12.345), "12,35");
        assert_eq!(format_amount(0.0), "0,00");
        assert_eq!(format_amount(-42.1), "-42,10");
    }

    #[test]
    fn test_line_item_defaults_and_calculations() {
        let def = LineItem::default();
        assert_eq!(def.description, "");
        assert_approx_eq(def.quantity, 1.0);
        assert_eq!(def.unit, "ks");
        assert_approx_eq(def.unit_price, 0.0);
        assert_eq!(def.vat_rate, 21);

        let item_21 = LineItem {
            description: "Development".into(),
            quantity: 2.5,
            unit: "hod".into(),
            unit_price: 100.0,
            vat_rate: 21,
        };
        assert_approx_eq(item_21.subtotal(), 250.0);
        assert_approx_eq(item_21.total_with_vat(), 302.5);

        let item_12 = LineItem {
            description: "Book".into(),
            quantity: 1.5,
            unit: "ks".into(),
            unit_price: 200.0,
            vat_rate: 12,
        };
        assert_approx_eq(item_12.subtotal(), 300.0);
        assert_approx_eq(item_12.total_with_vat(), 336.0);

        let item_0 = LineItem {
            description: "Exempt".into(),
            quantity: 3.0,
            unit: "ks".into(),
            unit_price: 50.0,
            vat_rate: 0,
        };
        assert_approx_eq(item_0.subtotal(), 150.0);
        assert_approx_eq(item_0.total_with_vat(), 150.0);
    }

    #[test]
    fn test_compute_totals() {
        let (excl_empty, vat_empty, incl_empty) = compute_totals(&[]);
        assert_approx_eq(excl_empty, 0.0);
        assert_approx_eq(vat_empty, 0.0);
        assert_approx_eq(incl_empty, 0.0);

        let single = [LineItem {
            description: "Service".into(),
            quantity: 2.0,
            unit: "hod".into(),
            unit_price: 1000.0,
            vat_rate: 21,
        }];
        let (excl_s, vat_s, incl_s) = compute_totals(&single);
        assert_approx_eq(excl_s, 2000.0);
        assert_approx_eq(vat_s, 420.0);
        assert_approx_eq(incl_s, 2420.0);

        let mixed = [
            LineItem {
                description: "Item 21%".into(),
                quantity: 1.5,
                unit: "hod".into(),
                unit_price: 333.33,
                vat_rate: 21,
            },
            LineItem {
                description: "Item 12%".into(),
                quantity: 2.25,
                unit: "ks".into(),
                unit_price: 40.5,
                vat_rate: 12,
            },
            LineItem {
                description: "Item 0%".into(),
                quantity: 3.0,
                unit: "ks".into(),
                unit_price: 10.11,
                vat_rate: 0,
            },
        ];
        // excl = 499.995 + 91.125 + 30.33 = 621.45
        // incl = 604.99395 + 102.06 + 30.33 = 737.38395 -> round2 = 737.38
        // vat  = 737.38395 - 621.45 = 115.93395 -> round2 = 115.93
        let (excl_m, vat_m, incl_m) = compute_totals(&mixed);
        assert_approx_eq(excl_m, 621.45);
        assert_approx_eq(vat_m, 115.93);
        assert_approx_eq(incl_m, 737.38);

        let zero_only = [LineItem {
            description: "Zero VAT".into(),
            quantity: 4.0,
            unit: "ks".into(),
            unit_price: 25.5,
            vat_rate: 0,
        }];
        let (excl_z, vat_z, incl_z) = compute_totals(&zero_only);
        assert_approx_eq(excl_z, 102.0);
        assert_approx_eq(vat_z, 0.0);
        assert_approx_eq(incl_z, 102.0);
    }

    #[test]
    fn test_template_data_from_parts_and_json_contract() {
        let invoice = Invoice {
            id: Some(7),
            number: "0007/2026".into(),
            variable_symbol: "00072026".into(),
            issue_date: "14.11.2026".into(),
            due_date: "28.11.2026".into(),
            payment_method: "Převodem".into(),
            note: "Poznámka k faktuře".into(),
            total_excl_vat: 100.126,
            total_vat: 21.024,
            total_incl_vat: 121.154,
            customer_name: "Odběratel s.r.o.".into(),
            pdf_path: None,
        };

        let supplier = Supplier {
            name: "Dodavatel s.r.o.".into(),
            reg_id: "12345678".into(),
            vat_id: "CZ12345678".into(),
            street: "Dlouhá 1".into(),
            zip: "110 00".into(),
            city: "Praha".into(),
            account_cz: "123456/0100".into(),
            iban: "CZ6508000000192000145399".into(),
            default_due_days: 14,
        };

        let customer = Customer {
            name: "Odběratel s.r.o.".into(),
            reg_id: "87654321".into(),
            vat_id: "CZ87654321".into(),
            street: "Krátká 2".into(),
            zip: "602 00".into(),
            city: "Brno".into(),
        };

        let items = vec![LineItem {
            description: "Konzultace".into(),
            quantity: 1.236,
            unit: "hod".into(),
            unit_price: 99.995,
            vat_rate: 21,
        }];

        let data = TemplateData::from_parts(&invoice, &supplier, &customer, &items);

        assert_eq!(data.invoice_number, "0007/2026");
        assert_eq!(data.variable_symbol, "00072026");
        assert_eq!(data.issue_date, "14.11.2026");
        assert_eq!(data.due_date, "28.11.2026");
        assert_eq!(data.payment_method, "Převodem");
        assert_eq!(data.note, "Poznámka k faktuře");
        assert_approx_eq(data.total_without_vat, 100.13);
        assert_approx_eq(data.total_vat, 21.02);
        assert_approx_eq(data.total_with_vat, 121.15);

        assert_eq!(data.supplier.name, "Dodavatel s.r.o.");
        assert_eq!(data.supplier.street, "Dlouhá 1");
        assert_eq!(data.supplier.zip, "110 00");
        assert_eq!(data.supplier.city, "Praha");
        assert_eq!(data.supplier.ico, "12345678");
        assert_eq!(data.supplier.dic, "CZ12345678");
        assert_eq!(data.supplier.bank_account, "123456/0100");
        assert_eq!(data.supplier.iban, "CZ6508000000192000145399");

        assert_eq!(data.customer.name, "Odběratel s.r.o.");
        assert_eq!(data.customer.street, "Krátká 2");
        assert_eq!(data.customer.zip, "602 00");
        assert_eq!(data.customer.city, "Brno");
        assert_eq!(data.customer.ico, "87654321");
        assert_eq!(data.customer.dic, "CZ87654321");

        assert_eq!(data.items.len(), 1);
        assert_eq!(data.items[0].description, "Konzultace");
        assert_approx_eq(data.items[0].quantity, 1.24);
        assert_eq!(data.items[0].unit, "hod");
        assert_approx_eq(data.items[0].unit_price, 100.0);
        assert_eq!(data.items[0].vat_rate, 21);
        // 1.236 * 99.995 * 1.21 = 149.5484222 -> round2 = 149.55
        assert_approx_eq(data.items[0].total, 149.55);

        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["invoice_number"], "0007/2026");
        assert_eq!(json["variable_symbol"], "00072026");
        assert_eq!(json["issue_date"], "14.11.2026");
        assert_eq!(json["due_date"], "28.11.2026");
        assert_eq!(json["payment_method"], "Převodem");
        assert_eq!(json["note"], "Poznámka k faktuře");
        assert!(json.get("total_without_vat").is_some());
        assert!(json.get("total_vat").is_some());
        assert!(json.get("total_with_vat").is_some());
        assert_eq!(json["supplier"]["ico"], "12345678");
        assert_eq!(json["supplier"]["dic"], "CZ12345678");
        assert_eq!(json["supplier"]["bank_account"], "123456/0100");
        assert_eq!(json["customer"]["ico"], "87654321");
        assert_eq!(json["customer"]["dic"], "CZ87654321");
        assert!(json["items"].is_array());
        assert_eq!(json["items"][0]["vat_rate"], 21);
    }
}
