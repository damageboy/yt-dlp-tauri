use super::ToolchainSource;
use serde::Deserialize;
use serde::{Serialize, Serializer};
use std::{collections::BTreeSet, path::Path};

const PLATFORM_CATALOG_SCHEMA_VERSION: u32 = 1;
const BUNDLED_PLATFORM_CATALOG: &str = include_str!("../../platform-toolchains.json");

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformCatalog {
    pub schema_version: u32,
    pub targets: Vec<PlatformToolchainDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformToolchainDefinition {
    pub target: String,
    pub os: String,
    pub arch: String,
    pub provider: ManagedProviderDefinition,
    pub source_labels: SourceLabels,
    pub default_source: ToolchainSource,
    pub executable_names: ExecutableNames,
    pub capabilities: ProviderCapabilities,
}

impl PlatformToolchainDefinition {
    pub fn presentation(&self) -> PlatformPresentation {
        PlatformPresentation {
            target: self.target.clone(),
            managed_provider: self.provider.kind(),
            source_labels: self.source_labels.clone(),
            default_source: self.default_source,
            executable_extension: Path::new(&self.executable_names.yt_dlp)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_owned),
            capabilities: self.capabilities,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ManagedProviderDefinition {
    ArchiveManifest {
        manifest_target: String,
    },
    Homebrew {
        packages: Vec<HomebrewPackageDefinition>,
    },
}

impl ManagedProviderDefinition {
    pub fn kind(&self) -> ManagedProviderKind {
        match self {
            Self::ArchiveManifest { .. } => ManagedProviderKind::ArchiveManifest,
            Self::Homebrew { .. } => ManagedProviderKind::Homebrew,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedProviderKind {
    ArchiveManifest,
    Homebrew,
}

impl Serialize for ManagedProviderKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::ArchiveManifest => "archive-manifest",
            Self::Homebrew => "homebrew",
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HomebrewPackageDefinition {
    pub formula: String,
    pub executables: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutableNames {
    pub yt_dlp: String,
    pub ffmpeg: String,
    pub ffprobe: String,
    pub deno: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLabels {
    pub managed: String,
    pub local: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCapabilities {
    pub install: bool,
    pub update: bool,
    pub reinstall: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformPresentation {
    pub target: String,
    pub managed_provider: ManagedProviderKind,
    pub source_labels: SourceLabels,
    pub default_source: ToolchainSource,
    pub executable_extension: Option<String>,
    pub capabilities: ProviderCapabilities,
}

pub fn parse_platform_catalog(json: &str) -> Result<PlatformCatalog, String> {
    let catalog: PlatformCatalog = serde_json::from_str(json)
        .map_err(|error| format!("Invalid platform toolchain catalog: {error}"))?;
    validate_platform_catalog(&catalog)?;
    Ok(catalog)
}

pub fn bundled_platform_catalog() -> Result<PlatformCatalog, String> {
    parse_platform_catalog(BUNDLED_PLATFORM_CATALOG)
}

pub fn platform_definition_from(
    catalog: &PlatformCatalog,
    os: &str,
    arch: &str,
) -> Result<PlatformToolchainDefinition, String> {
    catalog
        .targets
        .iter()
        .find(|target| target.os == os && target.arch == arch)
        .cloned()
        .ok_or_else(|| format!("Unsupported platform toolchain: {os}-{arch}"))
}

fn validate_platform_catalog(catalog: &PlatformCatalog) -> Result<(), String> {
    if catalog.schema_version != PLATFORM_CATALOG_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported platform catalog schema: {}",
            catalog.schema_version
        ));
    }

    let mut target_ids = BTreeSet::new();
    let mut platform_pairs = BTreeSet::new();
    for target in &catalog.targets {
        require_name("target ID", &target.target)?;
        require_name("operating system", &target.os)?;
        require_name("architecture", &target.arch)?;
        require_name("managed source label", &target.source_labels.managed)?;
        require_name("local source label", &target.source_labels.local)?;

        if !target_ids.insert(target.target.as_str()) {
            return Err(format!(
                "Platform toolchain catalog contains duplicate target ID: {}",
                target.target
            ));
        }
        if !platform_pairs.insert((target.os.as_str(), target.arch.as_str())) {
            return Err(format!(
                "Platform toolchain catalog contains duplicate platform mapping: {}-{}",
                target.os, target.arch
            ));
        }

        for (tool, name) in [
            ("yt-dlp", &target.executable_names.yt_dlp),
            ("ffmpeg", &target.executable_names.ffmpeg),
            ("ffprobe", &target.executable_names.ffprobe),
            ("deno", &target.executable_names.deno),
        ] {
            require_name(&format!("{tool} executable name"), name)?;
        }

        match &target.provider {
            ManagedProviderDefinition::ArchiveManifest { manifest_target } => {
                require_name("archive provider manifestTarget", manifest_target)?;
            }
            ManagedProviderDefinition::Homebrew { packages } => {
                if packages.is_empty() {
                    return Err(format!(
                        "Homebrew provider for {} has an empty package list",
                        target.target
                    ));
                }
                for package in packages {
                    require_name("Homebrew formula", &package.formula)?;
                    if package.executables.is_empty() {
                        return Err(format!(
                            "Homebrew formula {} has an empty executable list",
                            package.formula
                        ));
                    }
                    for executable in &package.executables {
                        require_name("Homebrew executable name", executable)?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn require_name(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!(
            "Platform toolchain catalog {label} must not be empty"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TARGET: &str = r#"{"target":"one","os":"macos","arch":"aarch64","provider":{"kind":"homebrew","packages":[{"formula":"yt-dlp","executables":["yt-dlp"]}]},"sourceLabels":{"managed":"Homebrew","local":"Custom"},"defaultSource":"managed","executableNames":{"ytDlp":"yt-dlp","ffmpeg":"ffmpeg","ffprobe":"ffprobe","deno":"deno"},"capabilities":{"install":true,"update":true,"reinstall":true}}"#;

    fn catalog_with(targets: &str) -> String {
        format!(r#"{{"schemaVersion":1,"targets":[{targets}]}}"#)
    }

    #[test]
    fn bundled_catalog_selects_windows_and_both_macos_architectures() {
        let catalog = bundled_platform_catalog().expect("bundled catalog should parse");

        let windows = platform_definition_from(&catalog, "windows", "x86_64").unwrap();
        assert_eq!(windows.target, "win-x64");
        assert_eq!(windows.executable_names.yt_dlp, "yt-dlp.exe");
        assert_eq!(windows.source_labels.managed, "Managed");

        let arm = platform_definition_from(&catalog, "macos", "aarch64").unwrap();
        assert_eq!(arm.target, "macos-arm64");
        assert_eq!(arm.executable_names.yt_dlp, "yt-dlp");
        assert_eq!(arm.source_labels.managed, "Homebrew");
        assert_eq!(arm.source_labels.local, "Custom");

        let intel = platform_definition_from(&catalog, "macos", "x86_64").unwrap();
        assert_eq!(intel.target, "macos-x64");
    }

    #[test]
    fn catalog_rejects_duplicate_platform_pairs() {
        let duplicate = VALID_TARGET.replace("\"target\":\"one\"", "\"target\":\"two\"");
        let json = catalog_with(&format!("{VALID_TARGET},{duplicate}"));

        assert!(parse_platform_catalog(&json)
            .unwrap_err()
            .contains("duplicate platform mapping"));
    }

    #[test]
    fn unsupported_platform_error_names_os_and_architecture() {
        let catalog = bundled_platform_catalog().unwrap();
        let error = platform_definition_from(&catalog, "linux", "x86_64").unwrap_err();
        assert!(error.contains("linux-x86_64"));
    }

    #[test]
    fn catalog_rejects_unsupported_schema_versions() {
        let json = format!(r#"{{"schemaVersion":2,"targets":[{VALID_TARGET}]}}"#);
        assert!(parse_platform_catalog(&json)
            .unwrap_err()
            .contains("Unsupported platform catalog schema"));
    }

    #[test]
    fn catalog_rejects_duplicate_target_ids() {
        let duplicate = VALID_TARGET.replace("\"arch\":\"aarch64\"", "\"arch\":\"x86_64\"");
        let json = catalog_with(&format!("{VALID_TARGET},{duplicate}"));
        assert!(parse_platform_catalog(&json)
            .unwrap_err()
            .contains("duplicate target ID"));
    }

    #[test]
    fn catalog_rejects_empty_executable_names() {
        let target = VALID_TARGET.replace("\"ytDlp\":\"yt-dlp\"", "\"ytDlp\":\"  \"");
        assert!(parse_platform_catalog(&catalog_with(&target))
            .unwrap_err()
            .contains("executable name"));
    }

    #[test]
    fn catalog_rejects_empty_homebrew_formulas_and_executable_lists() {
        let empty_formula = VALID_TARGET.replace("\"formula\":\"yt-dlp\"", "\"formula\":\" \"");
        assert!(parse_platform_catalog(&catalog_with(&empty_formula))
            .unwrap_err()
            .contains("Homebrew formula"));

        let empty_executables = VALID_TARGET.replace("[\"yt-dlp\"]", "[]");
        assert!(parse_platform_catalog(&catalog_with(&empty_executables))
            .unwrap_err()
            .contains("executable list"));
    }

    #[test]
    fn catalog_rejects_empty_archive_manifest_targets() {
        let target = VALID_TARGET.replace(
            r#"{"kind":"homebrew","packages":[{"formula":"yt-dlp","executables":["yt-dlp"]}]}"#,
            r#"{"kind":"archive-manifest","manifestTarget":" "}"#,
        );
        assert!(parse_platform_catalog(&catalog_with(&target))
            .unwrap_err()
            .contains("manifestTarget"));
    }

    #[test]
    fn catalog_rejects_unknown_default_sources() {
        let target = VALID_TARGET.replace(
            "\"defaultSource\":\"managed\"",
            "\"defaultSource\":\"system\"",
        );
        assert!(parse_platform_catalog(&catalog_with(&target)).is_err());
    }
}
