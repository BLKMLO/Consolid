# Consolid

Application Windows native, écrite uniquement en Rust, destinée à vérifier une
consolidation existante à partir de plusieurs pièces sources.

## Fonctionnement

1. L'utilisateur sélectionne les pièces sources.
2. Il sélectionne séparément la consolidation déjà réalisée.
3. L'application extrait les contenus en local.
4. Les identifiants structurés et les personnes, sociétés ou adresses explicitement
   libellées sont pseudonymisés avec une table commune au lot.
5. Seuls les contenus pseudonymisés et des identifiants neutres (`SOURCE_001`, etc.)
   sont envoyés à l'API Mistral.
6. Les jetons de la réponse sont contrôlés : un jeton obligatoire supprimé, altéré
   ou inventé bloque la restitution.
7. La réponse est désanonymisée en mémoire puis écrite atomiquement dans le fichier
   choisi par l'utilisateur.

Une même valeur, y compris si elle apparaît dans plusieurs fichiers, reçoit le même
jeton pendant toute l'opération. Un texte ressemblant déjà à un jeton interne est
échappé afin d'éviter toute collision.

## Formats

| Format | Lecture |
|---|---|
| CSV | Oui, séparateur `,`, `;` ou tabulation détecté automatiquement |
| XLSX, XLS, XLSB, ODS | Oui |
| DOCX | Oui |
| JSON | Oui, UTF-8, structure aplatie en chemins de champs |
| TXT, MD | Oui, UTF-8 |
| PDF | Non, refus explicite |

Le résultat est du texte ou du Markdown. L'application ne prétend pas reconstruire
à l'identique un conteneur Office binaire à partir de la réponse d'un LLM.

## Compilation

Prérequis : Rust stable 1.92 ou ultérieur.

```bash
cargo build --release --locked
```

Sous Windows, l'exécutable est généré dans :

```text
target\release\consolid-audit.exe
```

La CI GitHub compile et teste le projet sur `windows-latest`, puis publie
`consolid-audit.exe` et son fichier `consolid-audit.exe.sha256` comme artefacts du
workflow. Le build Windows release n'ouvre pas de console secondaire.

## Versions

Les dépendances Cargo et les actions GitHub sont suivies par Dependabot, avec une
revue hebdomadaire. La publication de l'exécutable Windows en GitHub Release sur
tag `v*` (build testé, empreinte SHA-256 jointe) reste à activer.

## Utilisation

```bash
cargo run --release
```

Dans l'interface :

- ajoutez ou glissez-déposez les pièces justificatives ;
- choisissez la consolidation existante à contrôler ;
- saisissez la clé API Mistral et, si nécessaire, le modèle ;
- choisissez un fichier de sortie `.md` ou `.txt` ;
- lancez la vérification.

Les doublons, les conflits de chemins et les fichiers supérieurs à 50 Mio sont
refusés. Annuler un sélecteur conserve le choix précédent. Un traitement peut être
annulé ; une requête HTTP déjà envoyée peut toutefois attendre son délai réseau
avant de s'arrêter.

Le modèle par défaut est `mistral-small-latest`. L'URL de l'API est fixe afin
d'éviter qu'une donnée sensible soit envoyée vers un serveur arbitraire.

## Sécurité et limites

- la clé API et la table de correspondance ne sont ni enregistrées ni journalisées ;
- aucun chemin ni nom de fichier local n'est inclus dans la requête ;
- HTTPS avec validation des certificats est obligatoire ;
- les erreurs temporaires de connexion, de quota et de passerelle sont retentées
  au maximum trois fois avec une attente bornée ;
- les entrées sont limitées à 50 Mio par fichier, le texte extrait à 20 Mio et la
  requête protégée à 700 Kio afin de réserver la place nécessaire au résultat dans
  la fenêtre de contexte du modèle par défaut ; la réponse HTTP est lue avec une limite stricte de
  10 Mio, y compris sans en-tête `Content-Length` ;
- la sortie ne peut jamais remplacer une pièce source ou la consolidation ;
- l'écriture utilise un fichier temporaire dans le dossier cible avec restauration
  de l'ancien résultat en cas d'échec ;
- les documents sont encodés comme données structurées et les instructions qu'ils
  contiennent sont explicitement déclarées non fiables au modèle ;
- la détection des personnes et sociétés repose volontairement sur des libellés
  explicites : une revue humaine reste obligatoire avant tout traitement de données
  réelles ;
- la pseudonymisation réduit l'exposition mais ne constitue pas une anonymisation
  irréversible au sens juridique.

Consultez [SECURITY.md](SECURITY.md) avant un usage en production.

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo build --release --locked
cargo audit
```

## Licence

MIT, voir [LICENSE](LICENSE).
