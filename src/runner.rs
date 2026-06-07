//! Orchestration de l'export, découplée de l'affichage (CLI ou GUI).
//!
//! `run_export` exécute la boucle complète et émet des [`ExportEvent`] via un
//! callback, sans rien imprimer. Il vérifie un drapeau d'annulation entre les
//! lots pour permettre un arrêt propre depuis la GUI.

use crate::export;
use crate::imap_client::ImapClient;
use crate::index::IndexWriter;
use crate::state::ExportState;
use anyhow::{Context, Result};
use sanitize_filename::sanitize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// Paramètres d'un export, indépendants de la source (flags CLI ou formulaire GUI).
#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub email: String,
    pub password: String,
    pub output: PathBuf,
    pub server: String,
    pub port: u16,
    /// Vide = tous les dossiers.
    pub folders: Vec<String>,
    /// 0 = illimité.
    pub limit: usize,
    pub batch_size: usize,
}

/// Évènements émis pendant l'export, pour piloter une UI ou un affichage CLI.
#[derive(Debug, Clone)]
pub enum ExportEvent {
    Connecting {
        server: String,
        port: u16,
    },
    FoldersListed {
        count: usize,
    },
    FolderStarted {
        name: String,
        total: usize,
        already: usize,
    },
    Mail {
        folder: String,
        subject: String,
    },
    FolderSkipped {
        name: String,
        reason: String,
    },
    FolderDone {
        name: String,
    },
    Finished {
        total: usize,
    },
}

/// Exécute l'export complet. Renvoie le nombre de mails nouvellement exportés.
///
/// `cancel` est vérifié entre les lots : passé à `true`, l'export s'arrête
/// proprement (l'état de reprise est sauvegardé, l'index écrit).
pub fn run_export(
    cfg: &ExportConfig,
    cancel: &AtomicBool,
    mut on: impl FnMut(ExportEvent),
) -> Result<usize> {
    let account_dir = cfg.output.join(sanitize(&cfg.email));
    std::fs::create_dir_all(&account_dir)
        .with_context(|| format!("création de {}", account_dir.display()))?;

    on(ExportEvent::Connecting {
        server: cfg.server.clone(),
        port: cfg.port,
    });
    let mut client = ImapClient::connect(&cfg.server, cfg.port, &cfg.email, &cfg.password)?;

    let mut all_folders = client.list_folders()?;
    if !cfg.folders.is_empty() {
        all_folders.retain(|f| cfg.folders.iter().any(|sel| sel == f));
    }
    on(ExportEvent::FoldersListed {
        count: all_folders.len(),
    });

    let mut export_state = ExportState::load(&account_dir)?;
    let mut idx = IndexWriter::new();
    let mut total_new = 0usize;

    'folders: for folder in &all_folders {
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        let listing = match client.examine_folder(folder) {
            Ok(l) => l,
            Err(e) => {
                on(ExportEvent::FolderSkipped {
                    name: folder.clone(),
                    reason: format!("{e:#}"),
                });
                continue;
            }
        };

        let folder_dir = account_dir.join(sanitize_folder(folder));
        std::fs::create_dir_all(&folder_dir)
            .with_context(|| format!("création de {}", folder_dir.display()))?;

        let to_fetch: Vec<u32> = {
            let fstate = export_state.folder_mut(folder, listing.uid_validity);
            let mut v: Vec<u32> = listing
                .uids
                .iter()
                .copied()
                .filter(|uid| !fstate.is_exported(*uid))
                .collect();
            if cfg.limit > 0 && v.len() > cfg.limit {
                v.truncate(cfg.limit);
            }
            v
        };

        let already = listing.uids.len().saturating_sub(to_fetch.len());
        on(ExportEvent::FolderStarted {
            name: folder.clone(),
            total: to_fetch.len(),
            already,
        });
        if to_fetch.is_empty() {
            on(ExportEvent::FolderDone {
                name: folder.clone(),
            });
            continue;
        }

        for batch in to_fetch.chunks(cfg.batch_size.max(1)) {
            if cancel.load(Ordering::Relaxed) {
                export_state.save()?;
                break 'folders;
            }
            let messages = client.fetch_batch(folder, batch)?;
            for (uid, raw) in messages {
                let seq = export_state
                    .folder_mut(folder, listing.uid_validity)
                    .record(uid);
                let meta = export::export_message(&folder_dir, seq, &raw)
                    .with_context(|| format!("export UID {uid} de {folder}"))?;
                on(ExportEvent::Mail {
                    folder: folder.clone(),
                    subject: meta.subject.clone(),
                });
                idx.add(folder, uid, &meta);
                total_new += 1;
            }
            export_state.save()?;
        }
        on(ExportEvent::FolderDone {
            name: folder.clone(),
        });
    }

    export_state.save()?;
    if !idx.is_empty() {
        idx.write(&account_dir)?;
    }
    let _ = client.logout();

    on(ExportEvent::Finished { total: total_new });
    Ok(total_new)
}

/// Aplati un nom de dossier IMAP imbriqué (`Archives/2022`) en segment de chemin sûr.
pub fn sanitize_folder(folder: &str) -> String {
    folder
        .split(['/', '.'])
        .map(|seg| sanitize(seg.trim()))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(".")
}
