use anyhow::{Context, Result, anyhow};
use native_tls::TlsConnector;
use std::net::TcpStream;

type Session = imap::Session<native_tls::TlsStream<TcpStream>>;

/// Client IMAP non-destructif au-dessus du crate `imap`, avec reconnexion.
pub struct ImapClient {
    server: String,
    port: u16,
    email: String,
    password: String,
    session: Session,
}

/// Un dossier examiné : son UIDVALIDITY et la liste triée de ses UID.
pub struct FolderListing {
    pub uid_validity: Option<u32>,
    pub uids: Vec<u32>,
}

impl ImapClient {
    pub fn connect(server: &str, port: u16, email: &str, password: &str) -> Result<Self> {
        let session = Self::open_session(server, port, email, password)?;
        Ok(ImapClient {
            server: server.to_string(),
            port,
            email: email.to_string(),
            password: password.to_string(),
            session,
        })
    }

    fn open_session(server: &str, port: u16, email: &str, password: &str) -> Result<Session> {
        let tls = TlsConnector::builder().build().context("init TLS")?;
        let client = imap::connect((server, port), server, &tls)
            .with_context(|| format!("connexion IMAP à {server}:{port}"))?;
        client
            .login(email, password)
            .map_err(|(e, _)| anyhow!("échec login IMAP pour {email}: {e}"))
    }

    fn reconnect(&mut self) -> Result<()> {
        self.session = Self::open_session(&self.server, self.port, &self.email, &self.password)?;
        Ok(())
    }

    /// Liste les dossiers sélectionnables (exclut les `\Noselect`).
    pub fn list_folders(&mut self) -> Result<Vec<String>> {
        let names = self
            .session
            .list(Some(""), Some("*"))
            .context("LIST des dossiers")?;
        let mut folders = Vec::new();
        for name in names.iter() {
            let noselect = name
                .attributes()
                .iter()
                .any(|a| matches!(a, imap::types::NameAttribute::NoSelect));
            if !noselect {
                folders.push(name.name().to_string());
            }
        }
        Ok(folders)
    }

    /// EXAMINE (read-only, non-destructif) puis SEARCH ALL.
    pub fn examine_folder(&mut self, folder: &str) -> Result<FolderListing> {
        let mailbox = self
            .session
            .examine(folder)
            .with_context(|| format!("EXAMINE {folder}"))?;
        let uid_validity = mailbox.uid_validity;
        let set = self
            .session
            .uid_search("ALL")
            .with_context(|| format!("SEARCH ALL dans {folder}"))?;
        let mut uids: Vec<u32> = set.into_iter().collect();
        uids.sort_unstable();
        Ok(FolderListing { uid_validity, uids })
    }

    /// Fetch un lot d'UID en RFC822 (message brut). Reconnecte + ré-examine et
    /// réessaie une fois en cas d'erreur réseau.
    pub fn fetch_batch(&mut self, folder: &str, uids: &[u32]) -> Result<Vec<(u32, Vec<u8>)>> {
        let set = uids
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");

        match self.fetch_set(&set) {
            Ok(v) => Ok(v),
            Err(_) => {
                // Reconnexion + ré-sélection du dossier, puis nouvelle tentative.
                self.reconnect()
                    .context("reconnexion après échec de fetch")?;
                self.session
                    .examine(folder)
                    .with_context(|| format!("ré-EXAMINE {folder} après reconnexion"))?;
                self.fetch_set(&set)
                    .with_context(|| format!("fetch du lot dans {folder} (2e tentative)"))
            }
        }
    }

    fn fetch_set(&mut self, set: &str) -> Result<Vec<(u32, Vec<u8>)>> {
        let fetches = self.session.uid_fetch(set, "RFC822")?;
        let mut out = Vec::new();
        for f in fetches.iter() {
            if let (Some(uid), Some(body)) = (f.uid, f.body()) {
                out.push((uid, body.to_vec()));
            }
        }
        Ok(out)
    }

    pub fn logout(&mut self) -> Result<()> {
        self.session.logout().context("LOGOUT")?;
        Ok(())
    }
}
