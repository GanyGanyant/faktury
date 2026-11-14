use serde::Deserialize;

/// Data returned by the ARES business registry API.
#[derive(Debug, Clone)]
pub struct AresResult {
    pub name: String,
    pub reg_id: String,
    pub vat_id: String,
    pub street: String,
    pub zip: String,
    pub city: String,
}

// ---------------------------------------------------------------------------
// Internal deserialization types for ARES v2 JSON response
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AresResponse {
    ico: Option<String>,
    obchodni_jmeno: Option<String>,
    dic: Option<String>,
    sidlo: Option<AresAddress>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AresAddress {
    nazev_ulice: Option<String>,
    /// House number – may arrive as int or string in the JSON
    cislo_domovni: Option<serde_json::Value>,
    /// Orientation number – optional
    cislo_orientacni: Option<serde_json::Value>,
    nazev_obce: Option<String>,
    /// Postal code – may arrive as int (11000) or string ("110 00")
    psc: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Fetches company details from the Czech ARES registry by registration ID (IČO).
///
/// Returns `Err(String)` with a human-readable Czech error message on failure.
pub fn fetch(reg_id: &str) -> Result<AresResult, String> {
    let reg_id = reg_id.trim();
    if reg_id.is_empty() {
        return Err("IČO nesmí být prázdné.".to_string());
    }

    let url = format!(
        "https://ares.gov.cz/ekonomicke-subjekty-v-be/rest/ekonomicke-subjekty/{}",
        reg_id
    );

    let response = ureq::get(&url)
        .set("Accept", "application/json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => {
                format!("Subjekt s IČO {} nebyl nalezen v ARES.", reg_id)
            }
            other => format!("Chyba při volání ARES: {}", other),
        })?;

    if response.status() != 200 {
        return Err(format!("ARES vrátilo chybu HTTP {}.", response.status()));
    }

    let parsed: AresResponse = response
        .into_json()
        .map_err(|e| format!("Chyba při parsování odpovědi ARES: {}", e))?;

    let name = parsed.obchodni_jmeno.unwrap_or_default();
    let ico_out = parsed.ico.unwrap_or_else(|| reg_id.to_string());
    let vat_id = parsed.dic.unwrap_or_default();

    let (street, zip, city) = match parsed.sidlo {
        Some(addr) => {
            let street = build_street_address(&addr);
            let zip = format_zip_code(&addr.psc);
            let city = addr.nazev_obce.unwrap_or_default();
            (street, zip, city)
        }
        None => (String::new(), String::new(), String::new()),
    };

    Ok(AresResult {
        name,
        reg_id: ico_out,
        vat_id,
        street,
        zip,
        city,
    })
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Assembles a street address string from ARES address components.
fn build_street_address(addr: &AresAddress) -> String {
    let street_name = match &addr.nazev_ulice {
        Some(s) if !s.is_empty() => s.clone(),
        _ => return String::new(),
    };
    let house = json_value_to_string(&addr.cislo_domovni);
    let orient = json_value_to_string(&addr.cislo_orientacni);

    match (house.as_str(), orient.as_str()) {
        ("", _) => street_name,
        (h, "") => format!("{} {}", street_name, h),
        (h, o) => format!("{} {}/{}", street_name, h, o),
    }
}

/// Converts a JSON value that may be a number or string into an owned `String`.
fn json_value_to_string(value: &Option<serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// Formats a raw postal code value into the Czech format `NNN NN` (e.g. `110 00`).
fn format_zip_code(psc: &Option<serde_json::Value>) -> String {
    let raw = match psc {
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::String(s)) => s.replace(' ', ""),
        _ => return String::new(),
    };
    if raw.len() == 5 {
        format!("{} {}", &raw[..3], &raw[3..])
    } else {
        raw
    }
}
