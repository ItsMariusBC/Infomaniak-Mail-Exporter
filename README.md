# Infomaniak Mail Exporter

Exporteur de mails **Infomaniak** en ligne de commande, écrit en Rust.

Infomaniak ne fournit pas d'outil d'export et ne recommande que Thunderbird, ou
le POP3 — **qui ne télécharge que la boîte de réception**, pas les dossiers
Envoyés / Archives / etc. Cet outil se connecte en **IMAP** et exporte **tous les
dossiers**, en lecture **non-destructive** (mode `EXAMINE` : rien n'est marqué
comme lu ni supprimé sur le serveur).

Chaque mail est enregistré en `.eml` brut (format standard, **sans perte**,
réimportable dans n'importe quel client mail), ses pièces jointes sont extraites
dans un dossier dédié, et un `index.csv` récapitule tout.

Deux interfaces, même moteur :

- **GUI** (`infomaniak-mail-exporter-gui`) — application graphique aux couleurs
  Infomaniak Mail : formulaire, barre de progression et journal en temps réel.
- **CLI** (`infomaniak-mail-exporter`) — ligne de commande, idéale pour scripts
  et serveurs.

## Installation

Téléchargez l'archive pour votre plateforme depuis les
[Releases](../../releases) (elle contient les deux binaires), ou compilez depuis
les sources :

```sh
cargo build --release
# GUI : target/release/infomaniak-mail-exporter-gui
# CLI : target/release/infomaniak-mail-exporter
```

> **Linux** — la GUI nécessite quelques bibliothèques système à la compilation :
> `sudo apt-get install -y libgtk-3-dev libxkbcommon-dev libwayland-dev libxcb1-dev librsvg2-dev`

## Interface graphique (GUI)

```sh
infomaniak-mail-exporter-gui
```

Renseignez votre adresse email, votre mot de passe et le dossier de sortie
(bouton « Parcourir »), dépliez « Avancé » pour cibler des dossiers ou limiter
le nombre de mails, puis cliquez **Exporter**. La progression et le journal
s'affichent en direct ; « Annuler » stoppe proprement (l'état de reprise est
sauvegardé).

## Interface en ligne de commande (CLI)

```sh
infomaniak-mail-exporter --email vous@votredomaine.ch --output ./export
```

Le mot de passe est demandé en saisie masquée. Vous pouvez aussi le passer via la
variable d'environnement `IMAP_PASSWORD` (évitez `--password` en clair dans
l'historique shell).

### Options

| Option | Défaut | Description |
|---|---|---|
| `-e, --email` | *(requis)* | Adresse email = identifiant IMAP |
| `-p, --password` | — | Mot de passe (sinon `IMAP_PASSWORD` ou prompt) |
| `-o, --output` | `./export` | Dossier de sortie |
| `--server` | `mail.infomaniak.com` | Serveur IMAP |
| `--port` | `993` | Port IMAP (SSL/TLS) |
| `-f, --folders` | *(tous)* | Limite à certains dossiers (répétable) |
| `--limit` | `0` | Max de mails par dossier (`0` = illimité), pour tester |
| `--batch-size` | `50` | Taille des lots de fetch IMAP |

### Exemples

```sh
# Test : 5 mails de l'INBOX seulement
infomaniak-mail-exporter -e vous@domaine.ch -o ./test --folders INBOX --limit 5

# Export complet d'une boîte
infomaniak-mail-exporter -e vous@domaine.ch -o ./export

# Deuxième boîte dans un autre dossier
infomaniak-mail-exporter -e autre@domaine.ch -o ./export2
```

## Reprise après coupure

L'export est **idempotent** : un fichier `.export_state.json` mémorise les UID
déjà téléchargés par dossier. Relancer la même commande reprend où l'export
s'était arrêté, sans rien re-télécharger. Si Infomaniak invalide les UID d'un
dossier (changement d'`UIDVALIDITY`), ce dossier est ré-exporté intégralement.

## Arborescence de sortie

```
export/
  vous@domaine.ch/
    INBOX/
      0001_2023-05-01_Facture-mars.eml
      0001_2023-05-01_Facture-mars_attachments/
        facture.pdf
    Sent/
    Archives.2022/
    index.csv
    .export_state.json
```

## Authentification à deux facteurs (2FA)

Si la 2FA est activée sur le compte, le mot de passe normal ne fonctionne pas en
IMAP : générez un **mot de passe d'application** dans le Manager Infomaniak
(*Sécurité → Mots de passe d'application*) et utilisez-le à la place.

## Réimporter les `.eml`

Les fichiers `.eml` s'ouvrent directement dans Apple Mail, Thunderbird, Outlook,
etc. Dans Thunderbird, l'extension *ImportExportTools NG* permet de réimporter un
dossier entier de `.eml` en un clic.

## Licence

MIT — voir [LICENSE](LICENSE).
