use anyhow::{Context, Result};
use mail_parser::{MessageParser, MimeHeaders};
use std::path::{Path, PathBuf};

/// Métadonnées extraites d'un mail, pour l'index CSV.
pub struct MailMeta {
    pub date: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub attachment_count: usize,
    pub eml_relative: String,
}

/// Écrit un mail brut (.eml, sans perte) dans `folder_dir`, extrait ses pièces
/// jointes dans un sous-dossier dédié, et renvoie ses métadonnées pour l'index.
///
/// `seq` est un numéro 1-based servant à ordonner/nommer les fichiers.
pub fn export_message(folder_dir: &Path, seq: u32, raw: &[u8]) -> Result<MailMeta> {
    let parsed = MessageParser::default().parse(raw);

    let subject = parsed
        .as_ref()
        .and_then(|m| m.subject())
        .unwrap_or("(sans objet)")
        .to_string();
    let date = parsed
        .as_ref()
        .and_then(|m| m.date())
        .map(|d| format!("{:04}-{:02}-{:02}", d.year, d.month, d.day))
        .unwrap_or_else(|| "0000-00-00".to_string());
    let from = parsed
        .as_ref()
        .and_then(|m| m.from())
        .and_then(|a| a.first())
        .and_then(|addr| addr.address())
        .unwrap_or("")
        .to_string();
    let to = parsed
        .as_ref()
        .and_then(|m| m.to())
        .and_then(|a| a.first())
        .and_then(|addr| addr.address())
        .unwrap_or("")
        .to_string();

    let stem = file_stem(seq, &date, &subject);
    let eml_name = format!("{stem}.eml");
    let eml_path = folder_dir.join(&eml_name);
    std::fs::write(&eml_path, raw)
        .with_context(|| format!("écriture de {}", eml_path.display()))?;

    // Pièces jointes → <stem>_attachments/
    let mut attachment_count = 0;
    if let Some(msg) = parsed.as_ref() {
        let attachments: Vec<_> = msg.attachments().collect();
        if !attachments.is_empty() {
            let att_dir = folder_dir.join(format!("{stem}_attachments"));
            std::fs::create_dir_all(&att_dir)
                .with_context(|| format!("création de {}", att_dir.display()))?;
            for (i, part) in attachments.iter().enumerate() {
                let raw_name = part
                    .attachment_name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("attachment_{}", i + 1));
                let safe = unique_attachment_name(&att_dir, &raw_name, i);
                let att_path = att_dir.join(&safe);
                std::fs::write(&att_path, part.contents())
                    .with_context(|| format!("écriture de {}", att_path.display()))?;
                attachment_count += 1;
            }
        }
    }

    Ok(MailMeta {
        date,
        from,
        to,
        subject,
        attachment_count,
        eml_relative: eml_name,
    })
}

/// Construit un nom de fichier sûr : `0001_2023-05-01_Sujet-slug`.
fn file_stem(seq: u32, date: &str, subject: &str) -> String {
    let slug = slugify(subject, 60);
    format!("{seq:04}_{date}_{slug}")
}

/// Slugifie un texte arbitraire : sûr pour tous les FS, tronqué à `max` octets.
fn slugify(input: &str, max: usize) -> String {
    let sanitized = sanitize_filename::sanitize(input);
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in sanitized.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    let result: String = trimmed.chars().take(max).collect();
    let result = result.trim_matches('-').to_string();
    if result.is_empty() {
        "sans-objet".to_string()
    } else {
        result
    }
}

/// Nom de pièce jointe sûr et non-collisionnant dans `dir`.
fn unique_attachment_name(dir: &Path, raw_name: &str, index: usize) -> String {
    let safe = sanitize_filename::sanitize(raw_name);
    let safe = if safe.trim().is_empty() {
        format!("attachment_{}", index + 1)
    } else {
        safe
    };
    let candidate = dir.join(&safe);
    if !candidate.exists() {
        return safe;
    }
    // Collision : insère l'index avant l'extension.
    let path = PathBuf::from(&safe);
    let ext = path.extension().and_then(|e| e.to_str());
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(&safe);
    match ext {
        Some(e) => format!("{stem}_{}.{e}", index + 1),
        None => format!("{stem}_{}", index + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_handles_special_chars() {
        assert_eq!(
            slugify("Re: Facture / Mars 2023!", 60),
            "Re-Facture-Mars-2023"
        );
        assert_eq!(slugify("", 60), "sans-objet");
        assert_eq!(slugify("***", 60), "sans-objet");
        assert_eq!(slugify("Café ☕ émojis", 60), "Café-émojis");
    }

    #[test]
    fn slugify_truncates() {
        let long = "a".repeat(200);
        assert_eq!(slugify(&long, 10).len(), 10);
    }

    #[test]
    fn file_stem_format() {
        assert_eq!(
            file_stem(1, "2023-05-01", "Hello World"),
            "0001_2023-05-01_Hello-World"
        );
    }
}
