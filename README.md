# Consolid Audit

> Outil d'audit et de consolidation comptable avec anonymisation locale et analyse IA via Mistral

## 🚀 Fonctionnalités

- **Interface graphique moderne** avec glisser-déposer
- **Anonymisation locale** des données sensibles (emails, téléphones, SIREN/SIRET, noms, adresses, etc.)
- **Validation des fichiers** avant envoi
- **Intégration avec Mistral API** pour l'analyse IA
- **Support multi-formats** : CSV, Excel (XLSX, XLS, ODS), TXT
- **Gestion des erreurs** complète avec feedback visuel
- **Indicateur de statut** pour vérifier que tout est prêt avant envoi

## 📦 Prérequis

- Node.js 18+ (pour le frontend)
- Rust 1.70+ (pour le backend)
- Tauri CLI (`npm install -g @tauri-apps/cli`)
- Une clé API Mistral (optionnelle pour tester l'interface)

## 🛠 Installation

```bash
# Cloner le dépôt
git clone https://github.com/BLKMLO/Consolid.git
cd Consolid

# Installer les dépendances npm
npm install

# Installer les dépendances Rust (si nécessaire)
cargo build
```

## ⚡ Utilisation

### Développement

```bash
# Lancer l'application en mode développement
npm run tauri dev
```

### Production

```bash
# Construire l'application
npm run tauri build

# L'exécutable sera généré dans le dossier target/release
```

### Configuration

1. **Clé API Mistral** : Configurez votre clé API dans les paramètres de l'application
2. **Modèle** : Choisissez le modèle Mistral à utiliser (tiny, small, medium, large)
3. **Paramètres d'anonymisation** : Activez/désactivez les types de données à anonymiser

## 📁 Structure du projet

```
Consolid/
├── src/                    # Frontend (Svelte)
│   ├── components/        # Composants UI
│   ├── stores/            # Gestion d'état
│   └── App.svelte         # Composant principal
├── src-tauri/             # Backend (Rust)
│   ├── src/
│   │   ├── anonymizer/    # Module d'anonymisation
│   │   ├── validator/     # Module de validation
│   │   ├── api/           # Client Mistral API
│   │   └── file_handler/  # Gestion des fichiers
│   └── tauri.conf.json   # Configuration Tauri
├── package.json           # Dépendances npm
└── Cargo.toml             # Dépendances Rust
```

## 🔒 Sécurité

- **Anonymisation locale** : Toutes les données sensibles sont anonymisées sur votre machine avant envoi
- **Pas de stockage cloud** : Vos fichiers ne quittent jamais votre ordinateur
- **Chiffrement** : La communication avec Mistral API se fait via HTTPS
- **Clé API sécurisée** : La clé API est stockée localement et n'est jamais partagée

## 🎯 Cas d'usage

1. **Audit comptable** : Vérifiez la cohérence de vos données comptables
2. **Consolidation** : Consolidez plusieurs fichiers comptables
3. **Analyse IA** : Obtenez des insights et des recommandations basées sur l'IA
4. **Conformité** : Vérifiez que vos données respectent les normes comptables

## 📊 Formats supportés

- **CSV** : Fichiers CSV standard
- **Excel** : XLSX, XLS, ODS
- **Texte** : TXT, JSON
- **PDF** : (Lecture seule pour l'instant)

## 🤝 Contribution

Les contributions sont les bienvenues ! Veuillez ouvrir une issue ou une pull request.

## 📄 Licence

Ce projet est sous licence MIT. Voir le fichier [LICENSE](LICENSE) pour plus de détails.

## 🙏 Remerciements

- [Tauri](https://tauri.app/) - Framework pour applications desktop
- [Svelte](https://svelte.dev/) - Framework frontend
- [Mistral AI](https://mistral.ai/) - Modèles d'IA
- [Rust](https://www.rust-lang.org/) - Langage de programmation

---

Développé avec ❤️ par [BLKMLO](https://github.com/BLKMLO)
