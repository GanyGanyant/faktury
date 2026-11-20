use std::path::{Path, PathBuf};
use std::process::Command;

use typst_as_lib::TypstEngine;

use crate::models::TemplateData;

// Embedded fonts matching the template's font requirement ("Libertinus Sans")
static FONT_REGULAR: &[u8] = include_bytes!("../fonts/LibertinusSans-Regular.otf");
static FONT_BOLD: &[u8] = include_bytes!("../fonts/LibertinusSans-Bold.otf");
static FONT_ITALIC: &[u8] = include_bytes!("../fonts/LibertinusSans-Italic.otf");

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compiles the invoice PDF using the specified template and writes it to `output_path`.
///
/// 1. Serializes `data` to `templates_dir/invoice_data.json`.
/// 2. Executes `typst compile <template_path> <output_path>`.
/// 3. If the external CLI is not available, falls back to embedded `TypstEngine`.
pub fn compile(
    data: &TemplateData,
    templates_dir: &Path,
    template_name: &str,
    output_path: &Path,
) -> Result<(), String> {
    let json_bytes = serde_json::to_vec_pretty(data)
        .map_err(|e| format!("Failed to serialize invoice data: {}", e))?;

    // Ensure templates directory exists and write invoice_data.json
    std::fs::create_dir_all(templates_dir).map_err(|e| {
        format!(
            "Cannot create templates directory {}: {}",
            templates_dir.display(),
            e
        )
    })?;

    let json_path = templates_dir.join("invoice_data.json");
    std::fs::write(&json_path, &json_bytes)
        .map_err(|e| format!("Cannot write {}: {}", json_path.display(), e))?;

    let template_path = templates_dir.join(template_name);
    if !template_path.exists() {
        return Err(format!(
            "Template file does not exist: {}",
            template_path.display()
        ));
    }

    // Ensure output directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create output directory {}: {}", parent.display(), e))?;
    }

    // Attempt compilation via external typst CLI first
    let cli_status = Command::new("typst")
        .arg("compile")
        .arg(&template_path)
        .arg(output_path)
        .output();

    match cli_status {
        Ok(output) if output.status.success() => {
            return Ok(());
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "External typst CLI returned error: {}. Trying embedded engine...",
                stderr
            );
        }
        Err(err) => {
            eprintln!(
                "External typst CLI unavailable ({}). Using embedded engine...",
                err
            );
        }
    }

    // Fallback: in-process compile with typst-as-lib
    let template_content = std::fs::read_to_string(&template_path)
        .map_err(|e| format!("Cannot read template {}: {}", template_path.display(), e))?;

    let engine = TypstEngine::builder()
        .main_file(template_content)
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

    std::fs::write(output_path, &pdf_bytes)
        .map_err(|e| format!("Cannot write PDF file {}: {}", output_path.display(), e))?;

    Ok(())
}

/// Opens the PDF file with the system default PDF viewer.
pub fn open(path: &Path) {
    if let Err(e) = open::that(path) {
        eprintln!("Cannot open PDF viewer: {}", e);
    }
}

/// Derives the output PDF path inside `output_dir` from an invoice number.
///
/// Example: `"0001/2026"` → `<output_dir>/faktura_0001_2026.pdf`
pub fn pdf_output_path(output_dir: &Path, invoice_number: &str) -> PathBuf {
    let safe_name = invoice_number.replace('/', "_");
    let file_name = if safe_name.starts_with("faktura_") {
        format!("{}.pdf", safe_name)
    } else {
        format!("faktura_{}.pdf", safe_name)
    };
    output_dir.join(file_name)
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
            street: "Nádražní 55".into(),
            zip: "602 00".into(),
            city: "Brno".into(),
        };

        let items = vec![
            LineItem {
                description: "Programátorské práce".into(),
                quantity: 10.0,
                unit: "hod".into(),
                unit_price: 1200.0,
                vat_rate: 21,
            },
            LineItem {
                description: "Konzultace".into(),
                quantity: 2.0,
                unit: "hod".into(),
                unit_price: 1500.0,
                vat_rate: 21,
            },
        ];

        let invoice = Invoice {
            id: Some(1),
            number: "0001/2026".into(),
            variable_symbol: "00012026".into(),
            issue_date: "14.11.2026".into(),
            due_date: "28.11.2026".into(),
            payment_method: "Převodem".into(),
            note: "Děkujeme za spolupráci.".into(),
            total_excl_vat: 15000.0,
            total_vat: 3150.0,
            total_incl_vat: 18150.0,
            customer_name: customer.name.clone(),
            pdf_path: None,
        };

        let template_data = TemplateData::from_parts(&invoice, &supplier, &customer, &items);

        let temp = crate::test_util::TestTempDir::new("pdf_compile");
        let temp_dir = temp.path();

        // Test klasicka template
        let klasicka_path = temp_dir.join("klasicka.typ");
        std::fs::write(&klasicka_path, include_str!("../templates/faktura.typ")).unwrap();
        let out_klasicka = temp_dir.join("test_klasicka.pdf");
        let res = compile(&template_data, temp_dir, "klasicka.typ", &out_klasicka);
        assert!(res.is_ok(), "Failed to compile klasicka: {:?}", res);
        assert!(out_klasicka.exists());

        // Test moderni template
        let moderni_path = temp_dir.join("moderni.typ");
        std::fs::write(&moderni_path, include_str!("../templates/moderni.typ")).unwrap();
        let out_moderni = temp_dir.join("test_moderni.pdf");
        let res_moderni = compile(&template_data, temp_dir, "moderni.typ", &out_moderni);
        assert!(
            res_moderni.is_ok(),
            "Failed to compile moderni: {:?}",
            res_moderni
        );
        assert!(out_moderni.exists());
    }

    #[test]
    fn test_pdf_output_path() {
        let output_dir = Path::new("/tmp/invoices");
        assert_eq!(
            pdf_output_path(output_dir, "0001/2026"),
            output_dir.join("faktura_0001_2026.pdf")
        );
        assert_eq!(
            pdf_output_path(output_dir, "faktura_0001/2026"),
            output_dir.join("faktura_0001_2026.pdf")
        );
    }
}
