use std::path::{Path, PathBuf};

use directories::{ProjectDirs, UserDirs};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::models::Supplier;

static BUILTIN_KLASICKA: &str = include_str!("../templates/faktura.typ");
static BUILTIN_MODERNI: &str = include_str!("../templates/moderni.typ");

fn default_template() -> String {
    "klasicka.typ".to_string()
}

fn default_due_days() -> u32 {
    14
}

// ---------------------------------------------------------------------------
// Path resolver
// ---------------------------------------------------------------------------

/// Standardized filesystem locations for application configuration, data, and PDFs.
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// Directory for configuration and templates (`~/.config/faktury` on Linux).
    pub config_dir: PathBuf,
    /// Directory for application data like SQLite database (`~/.local/share/faktury` on Linux).
    pub data_dir: PathBuf,
    /// Default output folder for exported PDF invoices (user's `Documents/Faktury`).
    pub default_output_dir: PathBuf,
}

impl AppPaths {
    /// Returns the absolute path to `config.toml`.
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// Returns the absolute path to the templates directory.
    pub fn templates_dir(&self) -> PathBuf {
        self.config_dir.join("templates")
    }

    /// Returns the absolute path to `faktury.db`.
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("faktury.db")
    }
}

/// Resolves standard OS-compliant directories for configuration and persistent data.
///
/// Uses `directories::ProjectDirs` and `directories::UserDirs`.
pub fn resolve_paths() -> AppPaths {
    let (config_dir, data_dir) = if let Some(proj) = ProjectDirs::from("", "", "faktury") {
        (proj.config_dir().to_path_buf(), proj.data_dir().to_path_buf())
    } else {
        (PathBuf::from(".config/faktury"), PathBuf::from(".local/share/faktury"))
    };

    let default_output_dir = UserDirs::new()
        .and_then(|u| u.document_dir().map(|d| d.join("Faktury")))
        .unwrap_or_else(|| data_dir.join("invoices"));

    AppPaths {
        config_dir,
        data_dir,
        default_output_dir,
    }
}

/// Creates required configuration, template, and data directories if missing.
pub fn ensure_dirs(paths: &AppPaths) -> Result<(), String> {
    std::fs::create_dir_all(&paths.config_dir)
        .map_err(|e| format!("Failed to create config dir {}: {}", paths.config_dir.display(), e))?;
    std::fs::create_dir_all(&paths.templates_dir())
        .map_err(|e| format!("Failed to create templates dir {}: {}", paths.templates_dir().display(), e))?;
    std::fs::create_dir_all(&paths.data_dir)
        .map_err(|e| format!("Failed to create data dir {}: {}", paths.data_dir.display(), e))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Configuration model
// ---------------------------------------------------------------------------

/// Persistent user settings stored in `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Supplier company or trade name.
    #[serde(default)]
    pub name: String,
    /// Supplier business registration number (IČO).
    #[serde(default, alias = "reg_id")]
    pub ico: String,
    /// Supplier VAT identification number (DIČ).
    #[serde(default, alias = "vat_id")]
    pub dic: String,
    /// Supplier street address.
    #[serde(default)]
    pub street: String,
    /// Supplier postal code.
    #[serde(default)]
    pub zip: String,
    /// Supplier city / municipality.
    #[serde(default)]
    pub city: String,
    /// Supplier bank account number (CZ format).
    #[serde(default, alias = "account_cz")]
    pub bank_account: String,
    /// Supplier IBAN.
    #[serde(default)]
    pub iban: String,
    /// Default payment due duration in days.
    #[serde(default = "default_due_days")]
    pub default_due_days: u32,
    /// Currently selected Typst template file name (e.g., "klasicka.typ").
    #[serde(default = "default_template")]
    pub template: String,
    /// Destination folder for saving exported invoice PDFs.
    #[serde(default)]
    pub output_dir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            ico: String::new(),
            dic: String::new(),
            street: String::new(),
            zip: String::new(),
            city: String::new(),
            bank_account: String::new(),
            iban: String::new(),
            default_due_days: default_due_days(),
            template: default_template(),
            output_dir: String::new(),
        }
    }
}

impl AppConfig {
    /// Converts configuration into a `Supplier` domain model.
    pub fn to_supplier(&self) -> Supplier {
        Supplier {
            name: self.name.clone(),
            reg_id: self.ico.clone(),
            vat_id: self.dic.clone(),
            street: self.street.clone(),
            zip: self.zip.clone(),
            city: self.city.clone(),
            account_cz: self.bank_account.clone(),
            iban: self.iban.clone(),
            default_due_days: self.default_due_days,
        }
    }

    /// Updates internal supplier fields from a `Supplier` domain model.
    pub fn update_supplier_fields(&mut self, s: &Supplier) {
        self.name = s.name.clone();
        self.ico = s.reg_id.clone();
        self.dic = s.vat_id.clone();
        self.street = s.street.clone();
        self.zip = s.zip.clone();
        self.city = s.city.clone();
        self.bank_account = s.account_cz.clone();
        self.iban = s.iban.clone();
        self.default_due_days = s.default_due_days;
    }
}

// ---------------------------------------------------------------------------
// Template management
// ---------------------------------------------------------------------------

/// Seeds default built-in templates (`klasicka.typ` and `moderni.typ`) if not present on disk.
///
/// Existing user modifications are never overwritten.
pub fn ensure_default_templates(paths: &AppPaths) {
    let tdir = paths.templates_dir();
    let _ = std::fs::create_dir_all(&tdir);

    let klasicka_path = tdir.join("klasicka.typ");
    if !klasicka_path.exists() {
        let _ = std::fs::write(&klasicka_path, BUILTIN_KLASICKA);
    }

    let moderni_path = tdir.join("moderni.typ");
    if !moderni_path.exists() {
        let _ = std::fs::write(&moderni_path, BUILTIN_MODERNI);
    }
}

/// Scans the templates directory for all `*.typ` files and returns their sorted names.
pub fn list_templates(paths: &AppPaths) -> Vec<String> {
    let tdir = paths.templates_dir();
    let mut templates = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&tdir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "typ" {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            templates.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    templates.sort();
    if templates.is_empty() {
        templates.push(default_template());
    }
    templates
}

// ---------------------------------------------------------------------------
// Persistence & Migration
// ---------------------------------------------------------------------------

/// Copies legacy `faktury.db` from the current working directory into `data_dir` if needed.
pub fn migrate_legacy_db_if_needed(paths: &AppPaths) {
    let target = paths.db_path();
    let legacy = Path::new("faktury.db");

    if !target.exists() && legacy.exists() {
        if let Some(parent) = target.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::copy(legacy, &target);
    }
}

fn load_legacy_supplier(conn: &Connection) -> rusqlite::Result<Option<Supplier>> {
    let mut stmt = match conn.prepare(
        "SELECT name, reg_id, vat_id, street, zip, city,
                account_cz, iban, default_due_days
         FROM supplier WHERE id = 1",
    ) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    let result = stmt.query_row([], |row| {
        Ok(Supplier {
            name: row.get(0)?,
            reg_id: row.get(1)?,
            vat_id: row.get(2)?,
            street: row.get(3)?,
            zip: row.get(4)?,
            city: row.get(5)?,
            account_cz: row.get(6)?,
            iban: row.get(7)?,
            default_due_days: row.get::<_, i64>(8)? as u32,
        })
    });

    match result {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Loads configuration from `config.toml`, migrating legacy database settings if missing.
pub fn load_or_init(paths: &AppPaths) -> Result<AppConfig, String> {
    ensure_dirs(paths)?;
    let config_file = paths.config_file();

    if config_file.exists() {
        let content = std::fs::read_to_string(&config_file)
            .map_err(|e| format!("Failed to read {}: {}", config_file.display(), e))?;
        let mut cfg: AppConfig = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", config_file.display(), e))?;

        if cfg.output_dir.trim().is_empty() {
            cfg.output_dir = paths.default_output_dir.to_string_lossy().to_string();
        }
        if cfg.template.trim().is_empty() {
            cfg.template = default_template();
        }
        return Ok(cfg);
    }

    // config.toml does not exist. Migrate supplier from legacy database if present
    let mut config = AppConfig::default();
    config.output_dir = paths.default_output_dir.to_string_lossy().to_string();

    let legacy_candidates = [
        PathBuf::from("faktury.db"),
        paths.db_path(),
    ];

    for candidate in &legacy_candidates {
        if candidate.exists() {
            if let Ok(conn) = Connection::open(candidate) {
                if let Ok(Some(legacy_sup)) = load_legacy_supplier(&conn) {
                    config.update_supplier_fields(&legacy_sup);
                    break;
                }
            }
        }
    }

    save_config(paths, &config)?;
    Ok(config)
}

/// Serializes and writes `config` to `config.toml` via `toml::to_string_pretty`.
pub fn save_config(paths: &AppPaths, config: &AppConfig) -> Result<(), String> {
    let config_file = paths.config_file();
    if let Some(parent) = config_file.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create directory {}: {}", parent.display(), e))?;
    }

    let toml_str = toml::to_string_pretty(config)
        .map_err(|e| format!("Cannot serialize configuration: {}", e))?;

    std::fs::write(&config_file, toml_str)
        .map_err(|e| format!("Cannot write config file {}: {}", config_file.display(), e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization_and_defaults() {
        let mut cfg = AppConfig::default();
        cfg.name = "Moje Firma s.r.o.".into();
        cfg.ico = "12345678".into();
        cfg.dic = "CZ12345678".into();
        cfg.street = "Karlova 10".into();
        cfg.zip = "110 00".into();
        cfg.city = "Praha 1".into();
        cfg.bank_account = "123456789/0100".into();
        cfg.iban = "CZ0000000000000012345678".into();
        cfg.default_due_days = 21;
        cfg.template = "moderni.typ".into();
        cfg.output_dir = "/tmp/invoices".into();

        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        assert!(toml_str.contains("name = \"Moje Firma s.r.o.\""));
        assert!(toml_str.contains("ico = \"12345678\""));
        assert!(toml_str.contains("template = \"moderni.typ\""));

        let deserialized: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.name, cfg.name);
        assert_eq!(deserialized.ico, cfg.ico);
        assert_eq!(deserialized.default_due_days, 21);
        assert_eq!(deserialized.template, "moderni.typ");
    }

    #[test]
    fn test_config_deserialization_with_aliases() {
        let toml_input = r#"
            name = "Firma"
            reg_id = "87654321"
            vat_id = "CZ87654321"
            account_cz = "987654321/0800"
        "#;
        let cfg: AppConfig = toml::from_str(toml_input).unwrap();
        assert_eq!(cfg.name, "Firma");
        assert_eq!(cfg.ico, "87654321");
        assert_eq!(cfg.dic, "CZ87654321");
        assert_eq!(cfg.bank_account, "987654321/0800");
        assert_eq!(cfg.default_due_days, 14);
        assert_eq!(cfg.template, "klasicka.typ");
    }

    #[test]
    fn test_templates_seeding_and_listing() {
        let temp_dir = std::env::temp_dir().join("faktury_test_paths");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let paths = AppPaths {
            config_dir: temp_dir.join("config"),
            data_dir: temp_dir.join("data"),
            default_output_dir: temp_dir.join("invoices"),
        };

        ensure_dirs(&paths).unwrap();
        ensure_default_templates(&paths);

        let templates = list_templates(&paths);
        assert!(templates.contains(&"klasicka.typ".to_string()));
        assert!(templates.contains(&"moderni.typ".to_string()));

        // Ensure user edits are not overwritten
        let custom_content = "// user edited";
        std::fs::write(paths.templates_dir().join("klasicka.typ"), custom_content).unwrap();
        ensure_default_templates(&paths);
        let read_back = std::fs::read_to_string(paths.templates_dir().join("klasicka.typ")).unwrap();
        assert_eq!(read_back, custom_content);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
