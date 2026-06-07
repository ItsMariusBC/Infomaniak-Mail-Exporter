use crate::export::MailMeta;
use anyhow::{Context, Result};
use std::path::Path;

/// Accumule les lignes d'index et écrit un `index.csv` par compte.
pub struct IndexWriter {
    rows: Vec<Row>,
}

struct Row {
    folder: String,
    uid: u32,
    date: String,
    from: String,
    to: String,
    subject: String,
    attachments: usize,
    file: String,
}

impl IndexWriter {
    pub fn new() -> Self {
        IndexWriter { rows: Vec::new() }
    }

    pub fn add(&mut self, folder: &str, uid: u32, meta: &MailMeta) {
        self.rows.push(Row {
            folder: folder.to_string(),
            uid,
            date: meta.date.clone(),
            from: meta.from.clone(),
            to: meta.to.clone(),
            subject: meta.subject.clone(),
            attachments: meta.attachment_count,
            file: format!("{folder}/{}", meta.eml_relative),
        });
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Écrit `<dir>/index.csv`. Trie par dossier puis date pour la lisibilité.
    pub fn write(mut self, dir: &Path) -> Result<()> {
        self.rows
            .sort_by(|a, b| a.folder.cmp(&b.folder).then(a.date.cmp(&b.date)));
        let path = dir.join("index.csv");
        let mut wtr = csv::Writer::from_path(&path)
            .with_context(|| format!("création de {}", path.display()))?;
        wtr.write_record([
            "folder",
            "uid",
            "date",
            "from",
            "to",
            "subject",
            "attachments",
            "file",
        ])?;
        for r in &self.rows {
            wtr.write_record([
                &r.folder,
                &r.uid.to_string(),
                &r.date,
                &r.from,
                &r.to,
                &r.subject,
                &r.attachments.to_string(),
                &r.file,
            ])?;
        }
        wtr.flush()
            .with_context(|| format!("flush de {}", path.display()))?;
        Ok(())
    }
}

impl Default for IndexWriter {
    fn default() -> Self {
        Self::new()
    }
}
