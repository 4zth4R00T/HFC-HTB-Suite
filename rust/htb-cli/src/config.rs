use crate::ui;
use crate::vault::{self, Kdf, Secret, VaultError};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

pub struct Config {
    pub dir: PathBuf,
    pub vault_file: PathBuf,
    pub legacy_token_file: PathBuf,
    pub theme_file: PathBuf,
    pub lang_file: PathBuf,
    pub vpn_dir: PathBuf,
    pub download_dir: PathBuf,
}

fn home_dir() -> PathBuf {
    if let Ok(h) = std::env::var("HOME") {
        return PathBuf::from(h);
    }
    if let Ok(h) = std::env::var("USERPROFILE") {
        return PathBuf::from(h);
    }
    PathBuf::from(".")
}

impl Config {
    pub fn load() -> Self {
        let dir = home_dir().join(".config").join("htb-hfc");
        let cfg = Config {
            vault_file: dir.join("token.vault"),
            legacy_token_file: dir.join("token"),
            theme_file: dir.join("theme"),
            lang_file: dir.join("lang"),
            vpn_dir: dir.join("vpn"),
            download_dir: std::env::var("HTB_DL_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home_dir().join("HTB")),
            dir,
        };
        secure_dir(&cfg.dir);
        secure_dir(&cfg.vpn_dir);
        let _ = fs::create_dir_all(&cfg.download_dir);
        cfg
    }


    pub fn has_vault(&self) -> bool {
        self.vault_file.is_file()
    }

    pub fn has_legacy_token(&self) -> bool {
        self.legacy_token_file.is_file()
    }

    pub fn vault_kdf(&self) -> Option<Kdf> {
        let blob = vault::b64_decode(&fs::read_to_string(&self.vault_file).ok()?)?;
        vault::describe(&blob)
    }

    pub fn open_vault(&self, passphrase: &str) -> Result<Secret, VaultError> {
        let text = fs::read_to_string(&self.vault_file).map_err(|_| VaultError::Corrupt)?;
        let blob = vault::b64_decode(&text).ok_or(VaultError::Corrupt)?;
        vault::open(&blob, passphrase)
    }

    pub fn write_vault(&self, token: &str, passphrase: &str) -> Result<(), String> {
        self.write_vault_with(token, passphrase, Kdf::argon2id())
    }

    pub fn write_vault_with(&self, token: &str, passphrase: &str, kdf: Kdf) -> Result<(), String> {
        let blob = vault::seal(token, passphrase, kdf).map_err(|e| e.to_string())?;
        let mut text = Zeroizing::new(vault::b64_encode(&blob));
        text.push('\n');
        write_private(&self.vault_file, text.as_bytes()).map_err(|e| e.to_string())
    }

    pub fn read_legacy_token(&self) -> Option<Secret> {
        read_trimmed_secret(&self.legacy_token_file).or_else(|| {
            let legacy = home_dir().join(".config").join("htb-suite").join("token");
            read_trimmed_secret(&legacy)
        })
    }

    pub fn purge_legacy_token(&self) {
        shred(&self.legacy_token_file);
        shred(&home_dir().join(".config").join("htb-suite").join("token"));
    }

    pub fn forget_token(&self) {
        shred(&self.vault_file);
        self.purge_legacy_token();
    }


    pub fn read_theme(&self) -> Option<String> {
        read_trimmed(&self.theme_file).filter(|t| t == "dark" || t == "light")
    }

    pub fn write_theme(&self, theme: &str) -> std::io::Result<()> {
        fs::File::create(&self.theme_file)?.write_all(theme.trim().as_bytes())
    }

    pub fn read_lang(&self) -> Option<String> {
        read_trimmed(&self.lang_file).filter(|l| ["es", "en", "ru"].contains(&l.as_str()))
    }

    pub fn write_lang(&self, lang: &str) -> std::io::Result<()> {
        fs::File::create(&self.lang_file)?.write_all(lang.trim().as_bytes())
    }
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_trimmed_secret(path: &Path) -> Option<Secret> {
    let raw = Zeroizing::new(fs::read_to_string(path).ok()?);
    let trimmed = Zeroizing::new(raw.trim().to_string());
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    restrict_file(&tmp);
    fs::rename(&tmp, path)?;
    restrict_file(path);
    Ok(())
}

fn shred(path: &Path) {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.is_file() {
            if let Ok(mut f) = fs::OpenOptions::new().write(true).open(path) {
                let zeros = vec![0u8; meta.len() as usize];
                let _ = f.write_all(&zeros);
                let _ = f.sync_all();
            }
        }
        let _ = fs::remove_file(path);
    }
}

fn secure_dir(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        if !path.exists() {
            let _ = fs::DirBuilder::new().recursive(true).mode(0o700).create(path);
        }
        let Ok(meta) = fs::symlink_metadata(path) else {
            return;
        };
        if meta.file_type().is_symlink() {
            ui::fail(&format!(
                "{} es un enlace simbolico. No guardo secretos ahi.",
                path.display()
            ));
            std::process::exit(1);
        }
        if meta.uid() != unsafe { libc::getuid() } {
            ui::fail(&format!(
                "{} pertenece a otro usuario (uid {}). No guardo secretos ahi.",
                path.display(),
                meta.uid()
            ));
            std::process::exit(1);
        }
        if meta.permissions().mode() & 0o077 != 0 {
            let mut perms = meta.permissions();
            perms.set_mode(0o700);
            let _ = fs::set_permissions(path, perms);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = fs::create_dir_all(path);
    }
}

#[cfg(unix)]
pub fn restrict_file(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
pub fn restrict_file(_path: &Path) {}
