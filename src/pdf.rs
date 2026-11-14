use std::path::{Path, PathBuf};

use typst_as_lib::TypstEngine;

use crate::models::TemplateData;

// The Typst template is embedded at compile time so no external file is needed
// at runtime. The template reads invoice data from a virtual file called
// "invoice_data.json" which we serve in-memory via a static file resolver.
static TEMPLATE_SOURCE: &str = include_str!("../templates/faktura.typ");

// Embedded fonts matching the template's font requirement ("Libertinus Sans")
static FONT_REGULAR: &[u8] = include_bytes!("../fonts/LibertinusSans-Regular.otf");
static FONT_BOLD: &[u8] = include_bytes!("../fonts/LibertinusSans-Bold.otf");
static FONT_ITALIC: &[u8] = include_bytes!("../fonts/LibertinusSans-Italic.otf");

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compiles the invoice PDF and writes it to `output_path`.
///
/// The Typst template (`templates/faktura.typ`) and required fonts are embedded
/// in the binary, making compilation completely self-contained.
/// Invoice data is provided as an in-memory virtual file so no temporary
/// JSON file is written to disk.
pub fn compile(data: &TemplateData, output_path: &Path) -> Result<(), String> {
    let json_bytes = serde_json::to_vec_pretty(data)
        .map_err(|e| format!("Failed to serialise invoice data: {}", e))?;

    // Also persist templates/invoice_data.json for external Typst CLI compatibility
    if let Ok(_) = std::fs::create_dir_all("templates") {
        let _ = std::fs::write("templates/invoice_data.json", &json_bytes);
    }

    let engine = TypstEngine::builder()
        .main_file(TEMPLATE_SOURCE)
        .fonts([FONT_REGULAR, FONT_BOLD, FONT_ITALIC])
        .with_package_file_resolver()
        .with_static_file_resolver([("invoice_data.json", json_bytes)])
        .build();

    let document = engine
        .compile()
        .output
        .map_err(|err| format!("Typst error: {:?}", err))?;

    let pdf_bytes = typst_pdf::pdf(&document, &Default::default())
        .map_err(|e| format!("PDF export error: {:?}", e))?;

    // Create output directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create output directory: {}", e))?;
    }

    std::fs::write(output_path, &pdf_bytes)
        .map_err(|e| format!("Cannot write PDF file: {}", e))?;

    Ok(())
}

/// Opens the PDF file with the system default PDF viewer.
pub fn open(path: &Path) {
    if let Err(e) = open::that(path) {
        eprintln!("Cannot open PDF viewer: {}", e);
    }
}

/// Derives the output PDF path from an invoice number.
///
/// Example: `"0001/2026"` → `./invoices/0001_2026.pdf`
pub fn pdf_output_path(invoice_number: &str) -> PathBuf {
    let safe_name = invoice_number.replace('/', "_");
    PathBuf::from("invoices").join(format!("{}.pdf", safe_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Customer, Invoice, LineItem, Supplier};

    #[test]
    fn test_pdf_compile_with_template_data() {
        let supplier = Supplier {
            name: "Test Dodavatel s.r.o.".into(),
            reg_id: "12345678".into(),
            vat_id: "CZ12345678".into(),
            street: "Vodičkova 123/4".into(),
            zip: "110 00".into(),
            city: "Praha 1".into(),
            account_cz: "123456789/0100".into(),
            iban: "CZ0000000000000012345678".into(),
            default_due_days: 14,
        };

        let customer = Customer {
            name: "Test Odběratel a.s.".into(),
            reg_id: "87654321".into(),
            vat_id: "CZ87654321".into(),
            street: "Nádražní 5".into(),
            zip: "602 00".into(),
            city: "Brno".into(),
        };

        let items = vec![
            LineItem {
                description: "Konzultační služby".into(),
                quantity: 2.0,
                unit: "hod".into(),
                unit_price: 1500.0,
                vat_rate: 21,
            },
            LineItem {
                description: "Položka bez DPH".into(),
                quantity: 1.0,
                unit: "ks".into(),
                unit_price: 500.0,
                vat_rate: 0,
            },
        ];

        let invoice = Invoice {
            id: None,
            number: "0001/2026".into(),
            variable_symbol: "00012026".into(),
            issue_date: "14.11.2026".into(),
            due_date: "28.11.2026".into(),
            payment_method: "Převodem".into(),
            note: "Děkujeme za spolupráci.\nToto je víceřádková poznámka.".into(),
            total_excl_vat: 3500.0,
            total_vat: 630.0,
            total_incl_vat: 4130.0,
            customer_name: customer.name.clone(),
            pdf_path: None,
        };

        let template_data = TemplateData::from_parts(&invoice, &supplier, &customer, &items);
        let temp_dir = std::env::temp_dir();
        let pdf_path = temp_dir.join("test_invoice.pdf");

        let result = compile(&template_data, &pdf_path);
        assert!(result.is_ok(), "Typst compilation failed: {:?}", result.err());
        assert!(pdf_path.exists(), "PDF file was not created");
        let _ = std::fs::remove_file(pdf_path);
    }
}
