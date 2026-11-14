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

    /// Returns the calculated VAT amount for this line item.
    #[allow(dead_code)]
    pub fn vat_amount(&self) -> f64 {
        self.total_with_vat() - self.subtotal()
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
