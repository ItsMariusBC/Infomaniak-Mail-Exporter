mod cli;
mod export;
mod imap_client;
mod index;
mod state;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use imap_client::ImapClient;
use index::IndexWriter;
use indicatif::{ProgressBar, ProgressStyle};
use sanitize_filename::sanitize;
use state::ExportState;

fn main() -> Result<()> {
    let args = Cli::parse();
    let password = args.resolve_password()?;

    // Dossier de sortie par compte : <output>/<email>
    let account_dir = args.output.join(sanitize(&args.email));
    std::fs::create_dir_all(&account_dir)
        .with_context(|| format!("création de {}", account_dir.display()))?;

    eprintln!(
        "Connexion à {}:{} en tant que {}…",
        args.server, args.port, args.email
    );
    let mut client = ImapClient::connect(&args.server, args.port, &args.email, &password)?;

    let mut all_folders = client.list_folders()?;
    if !args.folders.is_empty() {
        all_folders.retain(|f| args.folders.iter().any(|sel| sel == f));
    }
    eprintln!("{} dossier(s) à exporter.", all_folders.len());

    let mut export_state = ExportState::load(&account_dir)?;
    let mut idx = IndexWriter::new();
    let mut total_new = 0usize;

    for folder in &all_folders {
        let listing = match client.examine_folder(folder) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("  ! dossier {folder} ignoré : {e:#}");
                continue;
            }
        };

        let folder_dir = account_dir.join(sanitize_folder(folder));
        std::fs::create_dir_all(&folder_dir)
            .with_context(|| format!("création de {}", folder_dir.display()))?;

        // UID à télécharger = non encore exportés (selon l'état de reprise).
        let to_fetch: Vec<u32> = {
            let fstate = export_state.folder_mut(folder, listing.uid_validity);
            let mut v: Vec<u32> = listing
                .uids
                .iter()
                .copied()
                .filter(|uid| !fstate.is_exported(*uid))
                .collect();
            if args.limit > 0 && v.len() > args.limit {
                v.truncate(args.limit);
            }
            v
        };

        let already = listing.uids.len().saturating_sub(to_fetch.len());
        eprintln!(
            "→ {folder} : {} mail(s), {} déjà exporté(s), {} à télécharger",
            listing.uids.len(),
            already,
            to_fetch.len()
        );
        if to_fetch.is_empty() {
            continue;
        }

        let pb = ProgressBar::new(to_fetch.len() as u64);
        pb.set_style(
            ProgressStyle::with_template("  [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );

        for batch in to_fetch.chunks(args.batch_size.max(1)) {
            let messages = client.fetch_batch(folder, batch)?;
            for (uid, raw) in messages {
                let seq = export_state
                    .folder_mut(folder, listing.uid_validity)
                    .record(uid);
                let meta = export::export_message(&folder_dir, seq, &raw)
                    .with_context(|| format!("export UID {uid} de {folder}"))?;
                idx.add(folder, uid, &meta);
                total_new += 1;
                pb.inc(1);
            }
            // Sauvegarde régulière de l'état → reprise fiable si coupure.
            export_state.save()?;
        }
        pb.finish_and_clear();
    }

    export_state.save()?;
    if !idx.is_empty() {
        idx.write(&account_dir)?;
    }

    if let Err(e) = client.logout() {
        eprintln!("Avertissement : LOGOUT a échoué : {e:#}");
    }

    eprintln!(
        "Terminé. {total_new} nouveau(x) mail(s) exporté(s) dans {}",
        account_dir.display()
    );
    Ok(())
}

/// Aplati un nom de dossier IMAP imbriqué (`Archives/2022`) en segment de chemin sûr.
fn sanitize_folder(folder: &str) -> String {
    folder
        .split(['/', '.'])
        .map(|seg| sanitize(seg.trim()))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(".")
}
