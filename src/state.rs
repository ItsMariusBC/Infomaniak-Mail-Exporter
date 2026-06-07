use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// État de reprise persisté à la racine du dossier de sortie d'un compte.
///
/// Permet de relancer l'export après une coupure sans re-télécharger : on garde
/// par dossier l'UIDVALIDITY (pour détecter une invalidation côté serveur) et la
/// liste des UID déjà exportés.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ExportState {
    #[serde(skip)]
    path: PathBuf,
    pub folders: HashMap<String, FolderState>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FolderState {
    pub uid_validity: Option<u32>,
    pub exported_uids: BTreeSet<u32>,
    /// Prochain numéro de séquence pour nommer les fichiers (incrémental).
    pub next_seq: u32,
}

impl ExportState {
    /// Charge l'état depuis `<dir>/.export_state.json`, ou un état vide.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(".export_state.json");
        if path.exists() {
            let data = std::fs::read_to_string(&path)
                .with_context(|| format!("lecture de {}", path.display()))?;
            let mut state: ExportState = serde_json::from_str(&data)
                .with_context(|| format!("parsing de {}", path.display()))?;
            state.path = path;
            Ok(state)
        } else {
            Ok(ExportState {
                path,
                folders: HashMap::new(),
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&self.path, data)
            .with_context(|| format!("écriture de {}", self.path.display()))?;
        Ok(())
    }

    /// Prépare l'état d'un dossier pour cet UIDVALIDITY. Si l'UIDVALIDITY a
    /// changé, les anciens UID ne sont plus valides → on repart de zéro.
    pub fn folder_mut(&mut self, folder: &str, uid_validity: Option<u32>) -> &mut FolderState {
        let entry = self.folders.entry(folder.to_string()).or_default();
        if entry.uid_validity != uid_validity {
            entry.uid_validity = uid_validity;
            entry.exported_uids.clear();
            entry.next_seq = 0;
        }
        entry
    }
}

impl FolderState {
    pub fn is_exported(&self, uid: u32) -> bool {
        self.exported_uids.contains(&uid)
    }

    /// Réserve le prochain numéro de séquence (1-based) et marque l'UID exporté.
    pub fn record(&mut self, uid: u32) -> u32 {
        self.next_seq += 1;
        self.exported_uids.insert(uid);
        self.next_seq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_skips_exported_uids() {
        let dir = std::env::temp_dir().join(format!("ime-state-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let mut state = ExportState::load(&dir).unwrap();
        {
            let f = state.folder_mut("INBOX", Some(42));
            assert!(!f.is_exported(100));
            let seq = f.record(100);
            assert_eq!(seq, 1);
            assert!(f.is_exported(100));
        }
        state.save().unwrap();

        // Recharge : l'UID 100 doit rester marqué exporté.
        let mut reloaded = ExportState::load(&dir).unwrap();
        let f = reloaded.folder_mut("INBOX", Some(42));
        assert!(f.is_exported(100));
        assert_eq!(f.next_seq, 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn uidvalidity_change_resets_folder() {
        let dir = std::env::temp_dir().join(format!("ime-state-uv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let mut state = ExportState::load(&dir).unwrap();
        state.folder_mut("INBOX", Some(1)).record(7);
        assert!(state.folder_mut("INBOX", Some(1)).is_exported(7));

        // UIDVALIDITY différent → reset.
        let f = state.folder_mut("INBOX", Some(2));
        assert!(!f.is_exported(7));
        assert_eq!(f.next_seq, 0);

        std::fs::remove_dir_all(&dir).ok();
    }
}
