use crate::anonymize::Pseudonymizer;
use crate::extract::{extract, extract_batch};
use crate::mistral;
#[cfg(unix)]
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use zeroize::Zeroize;

const MAX_PROMPT_SIZE: usize = 30 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("aucune pièce source sélectionnée")]
    NoSources,
    #[error("le fichier de consolidation cible est aussi présent dans les pièces sources")]
    TargetInSources,
    #[error("sortie invalide : choisissez un fichier dans un dossier existant")]
    InvalidOutput,
    #[error("{0}")]
    Extract(#[from] crate::extract::ExtractError),
    #[error("{0}")]
    Mistral(#[from] mistral::MistralError),
    #[error("requête trop volumineuse (maximum 30 Mio après extraction)")]
    PromptTooLarge,
    #[error("écriture du résultat impossible : {0}")]
    Write(#[from] io::Error),
}

pub struct RunConfig {
    pub sources: Vec<PathBuf>,
    pub consolidation: PathBuf,
    pub output: PathBuf,
    pub api_key: String,
    pub model: String,
}

impl Drop for RunConfig {
    fn drop(&mut self) {
        self.api_key.zeroize();
    }
}

pub struct RunResult {
    pub output: PathBuf,
    pub replacements: usize,
}

pub fn run(config: RunConfig) -> Result<RunResult, WorkflowError> {
    validate(&config)?;
    let documents = extract_batch(&config.sources)?;
    let consolidation = extract(&config.consolidation)?;
    let mut pseudonymizer = Pseudonymizer::default();
    let mut prompt = String::from(
        "OBJECTIF\nVérifier la consolidation proposée à partir des pièces sources, corriger les incohérences et restituer le document consolidé complet.\n\n",
    );

    for document in documents {
        prompt.push_str("=== ");
        prompt.push_str(&document.label);
        prompt.push_str(" ===\n");
        prompt.push_str(&pseudonymizer.anonymize(&document.text));
        prompt.push_str("\n\n");
        if prompt.len() > MAX_PROMPT_SIZE {
            return Err(WorkflowError::PromptTooLarge);
        }
    }
    prompt.push_str("=== CONSOLIDATION_A_VERIFIER ===\n");
    prompt.push_str(&pseudonymizer.anonymize(&consolidation));
    prompt.push_str(
        "\n\nCONTRAINTES\n- Vérifier calculs, totaux, cohérence et omissions.\n- Conserver exactement chaque jeton [[CONSOLID_*]].\n- Ne jamais inventer une donnée absente.\n- Retourner uniquement la consolidation complète corrigée.\n",
    );
    if prompt.len() > MAX_PROMPT_SIZE {
        return Err(WorkflowError::PromptTooLarge);
    }

    let response = mistral::audit(&config.api_key, config.model.trim(), &prompt)?;
    let restored = pseudonymizer.restore(&response);
    atomic_write(&config.output, restored.as_bytes())?;
    Ok(RunResult {
        output: config.output.clone(),
        replacements: pseudonymizer.replacement_count(),
    })
}

fn validate(config: &RunConfig) -> Result<(), WorkflowError> {
    if config.sources.is_empty() {
        return Err(WorkflowError::NoSources);
    }
    let target = canonical_if_possible(&config.consolidation);
    if config
        .sources
        .iter()
        .map(|source| canonical_if_possible(source))
        .any(|source| source == target)
    {
        return Err(WorkflowError::TargetInSources);
    }
    if config.output.file_name().is_none()
        || config.output.parent().is_none_or(|parent| !parent.is_dir())
    {
        return Err(WorkflowError::InvalidOutput);
    }
    Ok(())
}

fn canonical_if_possible(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let base = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("consolidation");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(".{base}.{nonce}.tmp"));
    let backup = parent.join(format!(".{base}.{nonce}.bak"));

    let result = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;

        let had_existing = path.exists();
        if had_existing {
            std::fs::rename(path, &backup)?;
        }
        if let Err(error) = std::fs::rename(&temporary, path) {
            if had_existing {
                let _ = std::fs::rename(&backup, path);
            }
            return Err(error);
        }
        if had_existing {
            std::fs::remove_file(&backup)?;
        }
        sync_directory(parent);
        Ok(())
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn sync_directory(path: &Path) {
    if let Ok(directory) = File::open(path) {
        let _ = directory.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_labels_never_use_source_filenames() {
        let documents = vec![crate::extract::SourceDocument {
            label: "SOURCE_001".into(),
            text: "montant: 42".into(),
        }];
        let mut prompt = String::new();
        for document in documents {
            prompt.push_str(&document.label);
            prompt.push_str(&document.text);
        }
        assert!(!prompt.contains("Entreprise-Secrete"));
        assert!(prompt.contains("SOURCE_001"));
    }

    #[test]
    fn atomic_write_replaces_and_cleans_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("resultat.md");
        std::fs::write(&output, "ancien").unwrap();
        atomic_write(&output, b"nouveau").unwrap();
        assert_eq!(std::fs::read_to_string(&output).unwrap(), "nouveau");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
