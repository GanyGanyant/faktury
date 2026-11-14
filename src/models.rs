use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Core domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Supplier {
    pub name: String,
    pub reg_id: String,       // IČO
    pub vat_id: String,       // DIČ
    pub street: String,
    pub zip: String,
    pub city: String,
    pub account_cz: String,   // domestic account number (e.g. CZ...)
    pub iban: String,
    pub default_due_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Customer {
    pub name: String,
    pub reg_id: String,
    pub vat_id: String,
    pub street: String,
    pub zip: String,
    pub city: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub unit_price: f64,     // excl. VAT
    pub vat_rate: u8,        // 0, 12, or 21
}

impl LineItem {
    pub fn subtotal(&self) -> f64 {
        self.quantity * self.unit_price
    }

    pub fn total_with_vat(&self) -> f64 {
        self.subtotal() * (1.0 + self.vat_rate as f64 / 100.0)
    }

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Invoice {
    pub id: Option<i64>,
    pub number: String,
    pub variable_symbol: String,
    pub issue_date: String,
    pub due_date: String,
    pub payment_method: String,
    pub note: String,
    pub total_excl_vat: f64,
    pub total_vat: f64,
    pub total_incl_vat: f64,
    pub customer_name: String,
    pub pdf_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Typst template JSON schema  (must match keys in templates/faktura.typ)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateData {
    pub invoice_number: String,
    pub variable_symbol: String,
    pub issue_date: String,
    pub due_date: String,
    pub payment_method: String,
    pub note: String,
    pub total_without_vat: f64,
    pub total_vat: f64,
    pub total_with_vat: f64,
    pub supplier: TemplateSupplier,
    pub customer: TemplateCustomer,
    pub items: Vec<TemplateLineItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSupplier {
    pub name: String,
    pub street: String,
    pub zip: String,
    pub city: String,
    pub ico: String,
    pub dic: String,
    pub bank_account: String,
    pub iban: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateCustomer {
    pub name: String,
    pub street: String,
    pub zip: String,
    pub city: String,
    pub ico: String,
    pub dic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateLineItem {
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub unit_price: f64,
    pub vat_rate: u8,
    pub total: f64,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

impl TemplateData {
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

/// Formats a number with 2 decimal places and comma as separator (Czech locale).
pub fn format_amount(value: f64) -> String {
    format!("{:.2}", round2(value)).replace('.', ",")
}

pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Computes (excl_vat, vat_amount, incl_vat) totals from a list of line items.
pub fn compute_totals(items: &[LineItem]) -> (f64, f64, f64) {
    let excl_vat: f64 = items.iter().map(|i| i.subtotal()).sum();
    let incl_vat: f64 = items.iter().map(|i| i.total_with_vat()).sum();
    let vat_amount = incl_vat - excl_vat;
    (round2(excl_vat), round2(vat_amount), round2(incl_vat))
}
