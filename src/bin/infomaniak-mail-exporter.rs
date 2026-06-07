//! Binaire CLI : pilote l'export depuis la ligne de commande.

use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use infomaniak_mail_exporter::cli::Cli;
use infomaniak_mail_exporter::runner::{ExportConfig, ExportEvent, run_export};
use std::sync::atomic::AtomicBool;

fn main() -> Result<()> {
    let args = Cli::parse();
    let password = args.resolve_password()?;

    let cfg = ExportConfig {
        email: args.email,
        password,
        output: args.output,
        server: args.server,
        port: args.port,
        folders: args.folders,
        limit: args.limit,
        batch_size: args.batch_size,
    };

    let cancel = AtomicBool::new(false);
    let mut pb: Option<ProgressBar> = None;

    let total = run_export(&cfg, &cancel, |event| match event {
        ExportEvent::Connecting { server, port } => {
            eprintln!("Connexion à {server}:{port} en tant que {}…", cfg.email);
        }
        ExportEvent::FoldersListed { count } => {
            eprintln!("{count} dossier(s) à exporter.");
        }
        ExportEvent::FolderStarted {
            name,
            total,
            already,
        } => {
            eprintln!("→ {name} : {already} déjà exporté(s), {total} à télécharger");
            if total > 0 {
                let bar = ProgressBar::new(total as u64);
                bar.set_style(
                    ProgressStyle::with_template("  [{bar:40.magenta/red}] {pos}/{len} {msg}")
                        .unwrap()
                        .progress_chars("=>-"),
                );
                pb = Some(bar);
            }
        }
        ExportEvent::Mail { .. } => {
            if let Some(bar) = &pb {
                bar.inc(1);
            }
        }
        ExportEvent::FolderSkipped { name, reason } => {
            eprintln!("  ! dossier {name} ignoré : {reason}");
        }
        ExportEvent::FolderDone { .. } => {
            if let Some(bar) = pb.take() {
                bar.finish_and_clear();
            }
        }
        ExportEvent::Finished { total } => {
            eprintln!("Terminé. {total} nouveau(x) mail(s) exporté(s).");
        }
    })?;

    eprintln!(
        "{total} mail(s) exporté(s) dans {}",
        cfg.output
            .join(sanitize_filename::sanitize(&cfg.email))
            .display()
    );
    Ok(())
}
