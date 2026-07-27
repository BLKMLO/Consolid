# CLAUDE.md — contexte de travail

Mémo de contexte pour reprendre le projet sans le réexplorer. À mettre à jour à la
fin de chaque série de modifications (section « Journal » en bas).

## 1. Objet

`consolid-audit` (crate Rust, v0.4.0) : application de bureau Windows, GUI native,
qui **vérifie une consolidation comptable existante** à partir de pièces sources.
Elle pseudonymise localement, envoie le contenu protégé à l'API Mistral, contrôle
la réponse, la désanonymise en mémoire et reconstruit un classeur `.xlsx`.

Entrées : `.xlsx`, `.xls`, `.xlsb` uniquement. Sortie : `.xlsx` uniquement.
Interface et messages entièrement en français ; les commentaires et messages
d'erreur du code le sont aussi — conserver cette langue.

## 2. Repères

- Dépôt : `blkmlo/consolid` — branche de travail :
  `claude/consolidation-verification-setup-a6g9lu` (au 2026-07-27, identique à `main`).
- Toolchain : `rust-version = "1.92"` (conteneur : 1.94.1). Édition 2021.
- Licence MIT. Docs : `README.md` (usage), `SECURITY.md` (politique).
- CI : `.github/workflows/ci.yml` — job `windows` (fmt, clippy `-D warnings`,
  test, build release, SHA-256, upload artefact) + job `audit` (`rustsec/audit-check`).
  Dependabot hebdo sur cargo et github-actions.
- Pas de workflow de release sur tag `v*` : annoncé « reste à activer » dans le README.

## 3. Architecture (`src/`, ~2 400 lignes, 5 modules)

| Fichier | Rôle |
|---|---|
| `main.rs` (809 l.) | GUI `eframe`/`egui`, état `ConsolidApp`, sélection de fichiers `rfd`, thread de travail + `mpsc`, statuts |
| `workflow.rs` (702 l.) | Orchestration : validation, extraction, prompt JSON, appel Mistral, contrôle, **génération du .xlsx**, écriture atomique |
| `extract.rs` (145 l.) | Lecture des classeurs via `calamine`, sérialisation texte `SHEET_n` / `ROW_n` / `clé: valeur` |
| `anonymize.rs` (302 l.) | `Pseudonymizer` : jetons `[[CONSOLID_<KIND>_<NNNN>]]`, table en mémoire, restauration contrôlée |
| `mistral.rs` (454 l.) | Client HTTP bloquant `reqwest`, endpoint fixe, retries, parsing/validation de la réponse |

### Chaîne de traitement (`workflow::run_with_progress`)

1. `validate` — clé/modèle, ≤ 100 sources, doublons, conflits entrée/sortie, extension `.xlsx`, dossier parent existant.
2. Pour chaque source puis pour la consolidation : `extract` → `Pseudonymizer::anonymize`, `zeroize` du texte clair, garde cumulée `MAX_PROMPT_SIZE` (700 Kio).
3. `tokens_in(consolidation)` → jetons **obligatoires** dans la réponse.
4. Sérialisation `AuditPrompt` (objectif, `sources[]` avec ids neutres `SOURCE_001`, `consolidation_a_verifier`, 6 `contraintes`).
5. `mistral::audit` — POST `https://api.mistral.ai/v1/chat/completions`, `temperature 0.0`, `max_tokens 32768`, 3 tentatives max, statuts transitoires 429/502/503/504, `Retry-After` borné à 1–15 s.
6. `restore_checked` — refuse jeton inconnu, jeton obligatoire manquant, jeton malformé ; puis remplacement du plus long au plus court.
7. `build_xlsx` — reconstruit un ZIP OOXML *à la main* (Content_Types, rels, workbook, feuilles en `inlineStr`), puis `atomic_write` (tmp + `.bak` + rollback + `sync_all`).

Annulation : `CancellationToken` (`Arc<AtomicBool>`) contrôlé entre chaque étape et
pendant les attentes de retry ; une requête HTTP déjà partie doit expirer.

## 4. Invariants à ne pas casser

- **Endpoint Mistral en dur** : aucune URL configurable par l'utilisateur.
- **Aucun chemin ni nom de fichier local dans le prompt** (ids neutres `SOURCE_00n`) — testé par `prompt_payload_never_contains_local_filenames`.
- **Clé API et table de correspondance jamais persistées ni journalisées** ; `zeroize` sur `api_key`, `RunConfig::drop`, `Pseudonymizer::drop`, textes extraits, prompt et réponse (`Zeroizing`).
- **La sortie ne peut jamais écraser une entrée** (vérifié dans l'UI *et* dans `validate`).
- **Contrôle des jetons obligatoire** avant toute restitution : une réponse qui perd, altère ou invente un jeton est rejetée.
- Le contenu des documents est déclaré non fiable au modèle (message système + `contraintes[0]`).
- Limites : 50 Mio/fichier, 20 Mio de texte extrait, 700 Kio de prompt, 10 Mio de réponse HTTP.

## 5. Validation locale

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo build --release --locked
```

La CI ne tourne que sur `windows-latest` (+ `audit` sur ubuntu). Sous Linux, la
compilation d'`eframe` exige les dépendances système X11/Wayland ; le code métier
(`extract`, `anonymize`, `workflow`, `mistral`) reste testable.

Tests présents : 4 dans `extract.rs`, 6 dans `anonymize.rs`, 7 dans `workflow.rs`,
8 dans `mistral.rs` (dont un serveur HTTP mock sur `127.0.0.1:0` via
`audit_with_endpoint`, l'injection d'endpoint réservée aux tests).

## 6. Points d'attention / dette connue

- `SECURITY.md` est **désynchronisé** : il annonce encore « La sortie est limitée à `.md` ou `.txt` » alors que le code n'accepte que `.xlsx`.
- `MAX_SOURCE_FILES = 100` est dupliqué dans `main.rs:14` et `workflow.rs:20`.
- `worksheet_xml` (`workflow.rs:464`) indexe `row.cells[0]` quand aucune en-tête n'a
  été détectée : une réponse ne contenant que des marqueurs `ROW_n` sans paire
  `clé: valeur` produit des lignes vides et un accès hors bornes. Le panic est
  rattrapé par `catch_unwind` dans `main.rs`, mais le cas mérite un correctif.
- `build_xlsx` nomme les feuilles « Feuille n » ; les noms d'origine des onglets ne
  sont pas conservés (`extract` ne transmet que `SHEET_n`).
- Le style « carte » de l'UI est défini localement (`card`, `section_title`) ; fenêtre
  fixe 960×860 non redimensionnable.
- Pas de workflow de publication de release ; à écrire si demandé.

## 7. Conventions

- Messages de commit en français, à l'impératif/indicatif court (cf. historique :
  « Régénère Cargo.lock désynchronisé… »).
- Ne pas créer de PR sans demande explicite.
- Toujours pousser sur la branche de travail indiquée en §2.

## 8. Journal des modifications

| Date | Modification | Notes |
|---|---|---|
| 2026-07-27 | Création de `CLAUDE.md` (imprégnation initiale, aucun changement de code) | Aucune modification fonctionnelle |
