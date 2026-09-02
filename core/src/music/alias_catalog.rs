//! Device alias catalog (`devices/aliases.yml`).
//!
//! Maps aliases to canonical Bitwig device names.

use super::device::norm;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceHostKind {
    Bitwig,
    Clap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAlias {
    pub id: String,
    pub bitwig_name: String,
    pub kind: DeviceHostKind,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AliasCatalog {
    devices: Vec<DeviceAlias>,
}

#[derive(Debug, Deserialize)]
struct AliasCatalogYaml {
    devices: HashMap<String, DeviceAliasYaml>,
}

#[derive(Debug, Deserialize)]
struct DeviceAliasYaml {
    bitwig_name: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
}

impl AliasCatalog {
    pub fn from_yaml(text: &str) -> Result<Self, String> {
        let text = text.trim_start_matches('\u{feff}');
        let parsed: AliasCatalogYaml =
            serde_norway::from_str(text).map_err(|e| format!("aliases.yml: {e}"))?;

        let mut devices = Vec::with_capacity(parsed.devices.len());
        for (id, entry) in parsed.devices {
            let kind_str = entry.kind.as_deref().unwrap_or("bitwig");
            let kind = match kind_str.trim().to_lowercase().as_str() {
                "bitwig" => DeviceHostKind::Bitwig,
                "clap" => DeviceHostKind::Clap,
                other => {
                    return Err(format!(
                        "device '{id}': kind '{other}' unsupported — use bitwig|clap"
                    ));
                }
            };
            devices.push(DeviceAlias {
                id,
                bitwig_name: entry.bitwig_name,
                kind,
                aliases: entry.aliases,
            });
        }

        // Detect duplicate IDs, bitwig names, or aliases across different devices.
        let mut owners: HashMap<String, String> = HashMap::new();
        for dev in &devices {
            let names = std::iter::once(&dev.id)
                .chain(std::iter::once(&dev.bitwig_name))
                .chain(dev.aliases.iter());
            for name in names {
                let n = norm(name);
                if let Some(owner) = owners.get(&n) {
                    if owner != &dev.id {
                        let msg = format!(
                            "aliases.yml conflict: name '{name}' is used by both '{owner}' and '{}'",
                            dev.id
                        );
                        return Err(msg);
                    }
                } else {
                    owners.insert(n, dev.id.clone());
                }
            }
        }

        devices.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(Self { devices })
    }

    pub fn resolve(&self, name: &str) -> Option<&DeviceAlias> {
        let n = norm(name);
        self.devices.iter().find(|d| {
            norm(&d.id) == n || norm(&d.bitwig_name) == n || d.aliases.iter().any(|a| norm(a) == n)
        })
    }

    pub fn devices(&self) -> &[DeviceAlias] {
        &self.devices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_polymer_alias() {
        let yaml = r#"
id: aliases
kind: aliases
devices:
  polymer:
    bitwig_name: "Polymer"
    kind: bitwig
    aliases: ["poly", "Polymer"]
"#;
        let catalog = AliasCatalog::from_yaml(yaml).expect("parse should succeed");
        let dev = catalog.resolve("poly").expect("poly should resolve");
        assert_eq!(dev.bitwig_name, "Polymer");
        assert_eq!(dev.kind, DeviceHostKind::Bitwig);
    }

    #[test]
    fn rejects_duplicate_alias_across_devices() {
        let yaml = r#"
id: aliases
kind: aliases
devices:
  polymer:
    bitwig_name: "Polymer"
    aliases: ["poly"]
  polysynth:
    bitwig_name: "Polysynth"
    aliases: ["poly"]
"#;
        let err = AliasCatalog::from_yaml(yaml).expect_err("should fail on duplicate alias");
        assert!(
            err.contains("conflict"),
            "error should mention conflict: {err}"
        );
        assert!(
            err.contains("poly"),
            "error should mention the duplicated name: {err}"
        );
    }
}
