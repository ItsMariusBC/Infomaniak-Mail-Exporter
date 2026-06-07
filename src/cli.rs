use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

/// Exporteur de mails Infomaniak (IMAP) vers fichiers .eml + index CSV.
///
/// Lecture non-destructive (mode EXAMINE) : ne marque rien comme lu et ne
/// supprime rien sur le serveur, contrairement à POP3.
#[derive(Parser, Debug)]
#[command(name = "infomaniak-mail-exporter", version, about)]
pub struct Cli {
    /// Adresse email complète (= identifiant IMAP).
    #[arg(short, long)]
    pub email: String,

    /// Mot de passe. Si omis : variable d'env IMAP_PASSWORD, sinon saisie masquée.
    #[arg(short, long)]
    pub password: Option<String>,

    /// Dossier de sortie.
    #[arg(short, long, default_value = "./export")]
    pub output: PathBuf,

    /// Serveur IMAP.
    #[arg(long, default_value = "mail.infomaniak.com")]
    pub server: String,

    /// Port IMAP (SSL/TLS implicite).
    #[arg(long, default_value_t = 993)]
    pub port: u16,

    /// Dossiers IMAP à exporter (répétable). Par défaut : tous.
    #[arg(short, long)]
    pub folders: Vec<String>,

    /// Nombre max de mails par dossier (debug/test). 0 = illimité.
    #[arg(long, default_value_t = 0)]
    pub limit: usize,

    /// Taille des lots de fetch IMAP.
    #[arg(long, default_value_t = 50)]
    pub batch_size: usize,
}

impl Cli {
    /// Résout le mot de passe : flag > env IMAP_PASSWORD > prompt masqué.
    pub fn resolve_password(&self) -> Result<String> {
        if let Some(p) = &self.password {
            return Ok(p.clone());
        }
        if let Ok(p) = std::env::var("IMAP_PASSWORD")
            && !p.is_empty()
        {
            return Ok(p);
        }
        rpassword::prompt_password(format!("Mot de passe IMAP pour {} : ", self.email))
            .context("lecture du mot de passe")
    }
}
