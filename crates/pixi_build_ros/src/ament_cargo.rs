//! Native Cargo support for standalone ROS executable packages.

use std::path::{Path, PathBuf};

use miette::Diagnostic;
use rattler_conda_types::Platform;
use thiserror::Error;

pub const AMENT_CARGO_BUILD_TYPE: &str = "ament_cargo";

#[derive(Debug, Error, Diagnostic)]
pub enum AmentCargoError {
    #[error("ament_cargo is not supported on target platform {platform}")]
    #[diagnostic(help("ament_cargo packages support Linux, macOS, and Windows."))]
    UnsupportedPlatform { platform: String },

    #[error("ament_cargo requires a package-local Cargo.toml at {path}")]
    MissingCargoToml { path: PathBuf },

    #[error("failed to parse Cargo.toml at {path}")]
    InvalidCargoToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("Cargo.toml at {path} is missing a [package].name")]
    MissingPackageName { path: PathBuf },

    #[error(
        "Cargo package name `{cargo_package_name}` must equal ROS package name `{ros_package_name}`"
    )]
    PackageNameMismatch {
        cargo_package_name: String,
        ros_package_name: String,
    },

    #[error("ament_cargo cannot render {field} containing a quote or CR/LF")]
    InvalidBatchValue { field: &'static str },
}

#[derive(Debug, Clone)]
pub struct AmentCargoBuildScriptContext {
    source_dir: PathBuf,
    ros_package_name: String,
    host_platform: Platform,
}

impl AmentCargoBuildScriptContext {
    pub fn new(
        source_dir: impl Into<PathBuf>,
        ros_package_name: impl Into<String>,
        host_platform: Platform,
    ) -> Result<Self, AmentCargoError> {
        let source_dir = source_dir.into();
        let ros_package_name = ros_package_name.into();
        validate_cargo_manifest(&source_dir, &ros_package_name)?;
        validate_platform(host_platform)?;
        if host_platform.is_windows() {
            validate_batch_value("source path", &source_dir.display().to_string())?;
            validate_batch_value("package name", &ros_package_name)?;
        }
        Ok(Self {
            source_dir,
            ros_package_name,
            host_platform,
        })
    }

    pub fn render(&self) -> String {
        let template = if self.host_platform.is_windows() {
            include_str!("../templates/bld_ament_cargo.bat")
        } else {
            include_str!("../templates/build_ament_cargo.sh")
        };
        let source_dir = if self.host_platform.is_windows() {
            batch_escape(&self.source_dir.display().to_string())
        } else {
            shell_quote(self.source_dir.display().to_string())
        };
        let package_name = if self.host_platform.is_windows() {
            batch_escape(&self.ros_package_name)
        } else {
            shell_quote(&self.ros_package_name)
        };
        render_template(
            template,
            &[
                ("@SOURCE_DIR@", &source_dir),
                ("@ROS_PACKAGE_NAME@", &package_name),
            ],
        )
    }
}

pub fn validate_platform(platform: Platform) -> Result<(), AmentCargoError> {
    if platform.is_linux() || platform.is_osx() || platform.is_windows() {
        Ok(())
    } else {
        Err(AmentCargoError::UnsupportedPlatform {
            platform: platform.to_string(),
        })
    }
}

fn validate_cargo_manifest(
    source_dir: &Path,
    ros_package_name: &str,
) -> Result<(), AmentCargoError> {
    let cargo_toml = source_dir.join("Cargo.toml");
    if !cargo_toml.is_file() {
        return Err(AmentCargoError::MissingCargoToml { path: cargo_toml });
    }

    let content =
        fs_err::read_to_string(&cargo_toml).map_err(|_| AmentCargoError::MissingCargoToml {
            path: cargo_toml.clone(),
        })?;
    let manifest: toml::Value =
        toml::from_str(&content).map_err(|source| AmentCargoError::InvalidCargoToml {
            path: cargo_toml.clone(),
            source,
        })?;
    let cargo_package_name = manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| AmentCargoError::MissingPackageName {
            path: cargo_toml.clone(),
        })?;

    if cargo_package_name != ros_package_name {
        return Err(AmentCargoError::PackageNameMismatch {
            cargo_package_name: cargo_package_name.to_string(),
            ros_package_name: ros_package_name.to_string(),
        });
    }

    Ok(())
}

fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut index = 0;
    while index < template.len() {
        if let Some((placeholder, replacement)) = replacements
            .iter()
            .find(|(placeholder, _)| template[index..].starts_with(placeholder))
        {
            rendered.push_str(replacement);
            index += placeholder.len();
        } else {
            let character = template[index..]
                .chars()
                .next()
                .expect("template index is on a character boundary");
            rendered.push(character);
            index += character.len_utf8();
        }
    }
    rendered
}

fn shell_quote(value: impl AsRef<str>) -> String {
    format!("'{}'", value.as_ref().replace('\'', "'\\''"))
}

fn validate_batch_value(field: &'static str, value: &str) -> Result<(), AmentCargoError> {
    if value.contains(['\r', '\n', '"']) {
        return Err(AmentCargoError::InvalidBatchValue { field });
    }
    Ok(())
}

fn batch_escape(value: &str) -> String {
    value.replace('%', "%%")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir: &Path, name: &str) {
        fs_err::write(
            dir.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
        )
        .unwrap();
    }

    #[test]
    fn validates_matching_local_manifest_name() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "demo");
        assert!(AmentCargoBuildScriptContext::new(dir.path(), "demo", Platform::Linux64).is_ok());
        assert!(matches!(
            AmentCargoBuildScriptContext::new(dir.path(), "other", Platform::Linux64),
            Err(AmentCargoError::PackageNameMismatch { .. })
        ));
    }

    #[test]
    fn rejects_missing_and_malformed_manifests() {
        let missing = tempfile::tempdir().unwrap();
        assert!(matches!(
            AmentCargoBuildScriptContext::new(missing.path(), "demo", Platform::Linux64),
            Err(AmentCargoError::MissingCargoToml { .. })
        ));

        let malformed = tempfile::tempdir().unwrap();
        fs_err::write(malformed.path().join("Cargo.toml"), "not = [valid").unwrap();
        assert!(matches!(
            AmentCargoBuildScriptContext::new(malformed.path(), "demo", Platform::Linux64),
            Err(AmentCargoError::InvalidCargoToml { .. })
        ));
    }

    #[test]
    fn renders_linux_template_without_advanced_cargo_contract() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "demo");
        let context =
            AmentCargoBuildScriptContext::new(dir.path(), "demo", Platform::Linux64).unwrap();
        let script = context.render();

        assert!(script.contains("\"$cargo\" install"));
        assert!(script.contains("--root \"$staging_dir\""));
        assert!(script.contains("--target-dir \"$target_dir\""));
        assert!(script.contains("CARGO_HOME"));
        assert!(script.contains("resource_index/packages"));
        assert!(script.contains("package.xml"));
        assert!(!script.contains("--locked"));
        assert!(!script.contains("--config"));
        assert!(!script.contains("rust_packages"));
        assert!(!script.contains("Cargo.toml\" \"$PREFIX"));
        assert!(!script.contains("$SRC_DIR"));
        assert!(script.contains("RATTLER_BUILD_PACKAGE_FILES"));
    }

    #[test]
    fn accepts_linux_macos_and_windows_platforms() {
        assert!(validate_platform(Platform::Osx64).is_ok());
        assert!(validate_platform(Platform::Win64).is_ok());
        assert!(validate_platform(Platform::Linux64).is_ok());
    }

    #[test]
    fn renders_unix_for_linux_and_macos() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "demo");
        for platform in [Platform::Linux64, Platform::Osx64] {
            let script = AmentCargoBuildScriptContext::new(dir.path(), "demo", platform)
                .unwrap()
                .render();
            assert!(script.starts_with("#!/usr/bin/env bash"));
            assert!(script.contains("for binary in \"$staging_dir/bin/\"*"));
        }
    }

    #[test]
    fn renders_plain_batch_with_safe_percent_and_no_transport() {
        let context = AmentCargoBuildScriptContext {
            source_dir: PathBuf::from(r"C:\source\100%\demo"),
            ros_package_name: "demo".to_string(),
            host_platform: Platform::Win64,
        };
        let script = context.render();
        assert!(script.starts_with("@echo off\r\nsetlocal DisableDelayedExpansion"));
        assert!(script.contains(r#"set "source_dir=C:\source\100%%\demo""#));
        assert!(script.contains("Library\\bin\\cargo.exe"));
        assert!(script.contains("LIBRARY_PREFIX"));
        assert!(script.contains(".exe"));
        assert!(!script.to_ascii_lowercase().contains("call "));
        assert!(!script.to_ascii_lowercase().contains("python"));
        assert!(!script.to_ascii_lowercase().contains("powershell"));
        assert!(!script.to_ascii_lowercase().contains("base64"));
        assert!(script.contains("DisableDelayedExpansion"));
        assert!(script.contains("RATTLER_BUILD_PACKAGE_FILES"));
        assert!(!script.contains("@SOURCE_DIR@"));
        assert!(!script.contains("@ROS_PACKAGE_NAME@"));
    }

    #[test]
    fn batch_template_handles_existing_dirs_and_cleans_owned_outputs() {
        let template = include_str!("../templates/bld_ament_cargo.bat");
        assert!(template.contains("if exist \"%runtime_state%\" goto :failure"));
        assert!(template.contains("if not exist \"%cargo_home%\\\" goto :failure"));
        assert!(template.contains("del /q \"%output_root%\\lib\\%ros_package_name%\\*.exe\""));
        assert!(template.contains("echo(%%~fP"));
        assert!(template.contains(":failure"));
        assert!(template.contains("if exist \"%runtime_state%\" exit /b 1"));
    }

    #[test]
    fn rejects_batch_quotes_and_newlines() {
        for value in ["C:\\bad\"path", "C:\\bad\rpath", "C:\\bad\npath"] {
            assert!(matches!(
                validate_batch_value("source path", value),
                Err(AmentCargoError::InvalidBatchValue { .. })
            ));
        }
    }
}
