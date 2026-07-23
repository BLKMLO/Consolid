use regex::{Captures, Regex};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use zeroize::Zeroize;

const TOKEN_PREFIX: &str = "[[CONSOLID_";

#[derive(Debug, Clone, Copy)]
struct Pattern {
    kind: &'static str,
    regex_index: usize,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RestoreError {
    #[error("réponse IA incohérente : {0} jeton(s) obligatoire(s) absent(s)")]
    MissingTokens(usize),
    #[error("réponse IA incohérente : jeton de pseudonymisation inconnu")]
    UnknownToken,
    #[error("réponse IA incohérente : jeton de pseudonymisation altéré")]
    MalformedToken,
}

/// Table de pseudonymisation conservée uniquement en mémoire pendant un traitement.
///
/// Deux graphies identiques, sans distinction de casse ni d'espaces superflus,
/// reçoivent le même jeton dans tous les documents du lot.
pub struct Pseudonymizer {
    regexes: Vec<Regex>,
    patterns: Vec<Pattern>,
    labelled_regexes: Vec<Regex>,
    labelled_patterns: Vec<Pattern>,
    token_regex: Regex,
    reserved_token_regex: Regex,
    forward: HashMap<(String, String), String>,
    reverse: HashMap<String, String>,
    next_by_kind: HashMap<String, usize>,
}

impl Default for Pseudonymizer {
    fn default() -> Self {
        let definitions = [
            ("EMAIL", r"(?i)\b[A-Z0-9._%+\-]+@[A-Z0-9.\-]+\.[A-Z]{2,}\b"),
            ("IBAN", r"(?i)\b[A-Z]{2}\s?\d{2}(?:\s?[A-Z0-9]){11,30}\b"),
            ("SIRET", r"\b(?:\d[\s.]?){14}\b"),
            ("SIREN", r"\b(?:\d[\s.]?){9}\b"),
            (
                "PHONE",
                r"(?x)(?:\+33|0033|0)\s*[1-9](?:[\s.\-]*\d{2}){4}\b",
            ),
        ];

        let mut regexes = Vec::new();
        let mut patterns = Vec::new();
        for (kind, source) in definitions {
            let regex_index = regexes.len();
            regexes.push(Regex::new(source).expect("motif interne valide"));
            patterns.push(Pattern { kind, regex_index });
        }

        let labelled_definitions = [
            (
                "ORGANIZATION",
                r#"(?im)\b(société|societe|entreprise|raison\s+sociale|dénomination|denomination|company|client|customer|fournisseur|supplier|vendor)\s*(["']?\s*(?::|=|\||;)\s*["']?)([^\r\n\t,;|"]{2,160})"#,
            ),
            (
                "PERSON",
                r#"(?im)\b(nom|prénom|prenom|personne|contact|dirigeant|gérant|gerant|responsable|salarié|salarie|employé|employe|employee|bénéficiaire|beneficiaire)\s*(["']?\s*(?::|=|\||;)\s*["']?)([^\r\n\t,;|"]{2,120})"#,
            ),
            (
                "ADDRESS",
                r#"(?im)\b(adresse|address|siège\s+social|siege\s+social)\s*(["']?\s*(?::|=|\||;)\s*["']?)([^\r\n\t;|"]{4,200})"#,
            ),
        ];
        let mut labelled_regexes = Vec::new();
        let mut labelled_patterns = Vec::new();
        for (kind, source) in labelled_definitions {
            let regex_index = labelled_regexes.len();
            labelled_regexes.push(Regex::new(source).expect("motif interne valide"));
            labelled_patterns.push(Pattern { kind, regex_index });
        }

        Self {
            regexes,
            patterns,
            labelled_regexes,
            labelled_patterns,
            token_regex: Regex::new(r"\[\[CONSOLID_[A-Z]+_\d{4,}\]\]")
                .expect("motif interne valide"),
            reserved_token_regex: Regex::new(r"\[\[CONSOLID_[^\]\r\n]{1,80}\]\]")
                .expect("motif interne valide"),
            forward: HashMap::new(),
            reverse: HashMap::new(),
            next_by_kind: HashMap::new(),
        }
    }
}

impl Drop for Pseudonymizer {
    fn drop(&mut self) {
        for ((mut kind, mut canonical), mut token) in self.forward.drain() {
            kind.zeroize();
            canonical.zeroize();
            token.zeroize();
        }
        for (mut token, mut original) in self.reverse.drain() {
            token.zeroize();
            original.zeroize();
        }
        for (mut kind, _) in self.next_by_kind.drain() {
            kind.zeroize();
        }
    }
}

impl Pseudonymizer {
    pub fn anonymize(&mut self, input: &str) -> String {
        // Un jeton déjà présent dans une entrée ne doit jamais pouvoir entrer en
        // collision avec un jeton généré pendant ce traitement.
        let reserved_regex = self.reserved_token_regex.clone();
        let mut output = reserved_regex
            .replace_all(input, |caps: &Captures<'_>| {
                self.token_for_internal("LITERAL", caps.get(0).expect("capture complète").as_str())
            })
            .into_owned();

        // Les identifiants structurés sont traités avant les champs libellés.
        for pattern in self.patterns.clone() {
            let regex = self.regexes[pattern.regex_index].clone();
            output = regex
                .replace_all(&output, |caps: &Captures<'_>| {
                    self.token_for(
                        pattern.kind,
                        caps.get(0).expect("capture complète").as_str(),
                    )
                })
                .into_owned();
        }

        for pattern in self.labelled_patterns.clone() {
            let regex = self.labelled_regexes[pattern.regex_index].clone();
            output = regex
                .replace_all(&output, |caps: &Captures<'_>| {
                    let label = caps.get(1).expect("libellé").as_str();
                    let separator = caps.get(2).expect("séparateur").as_str();
                    let value = caps.get(3).expect("valeur").as_str();
                    format!(
                        "{label}{separator}{}",
                        self.token_for(pattern.kind, value.trim_end())
                    )
                })
                .into_owned();
        }

        output
    }

    pub fn tokens_in(&self, input: &str) -> HashSet<String> {
        self.token_regex
            .find_iter(input)
            .map(|value| value.as_str().to_owned())
            .collect()
    }

    pub fn restore_checked(
        &self,
        input: &str,
        required_tokens: &HashSet<String>,
    ) -> Result<String, RestoreError> {
        let observed = self.tokens_in(input);
        if observed
            .iter()
            .any(|token| !self.reverse.contains_key(token))
        {
            return Err(RestoreError::UnknownToken);
        }
        let missing = required_tokens.difference(&observed).count();
        if missing != 0 {
            return Err(RestoreError::MissingTokens(missing));
        }
        let without_valid_tokens = self.token_regex.replace_all(input, "");
        if without_valid_tokens.contains(TOKEN_PREFIX) {
            return Err(RestoreError::MalformedToken);
        }

        let mut output = input.to_owned();
        let mut mappings: Vec<_> = self.reverse.iter().collect();
        mappings.sort_unstable_by_key(|(token, _)| std::cmp::Reverse(token.len()));
        for (token, original) in mappings {
            output = output.replace(token, original);
        }
        Ok(output)
    }

    pub fn replacement_count(&self) -> usize {
        self.reverse.len()
    }

    fn token_for(&mut self, kind: &str, original: &str) -> String {
        if original.starts_with(TOKEN_PREFIX) {
            return original.to_owned();
        }

        self.token_for_internal(kind, original)
    }

    fn token_for_internal(&mut self, kind: &str, original: &str) -> String {
        let canonical = original
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let key = (kind.to_owned(), canonical);
        if let Some(token) = self.forward.get(&key) {
            return token.clone();
        }

        let counter = self.next_by_kind.entry(kind.to_owned()).or_insert(0);
        *counter += 1;
        let token = format!("{TOKEN_PREFIX}{kind}_{counter:04}]]");
        self.forward.insert(key, token.clone());
        self.reverse.insert(token.clone(), original.to_owned());
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_company_gets_same_token_across_documents() {
        let mut p = Pseudonymizer::default();
        let first = p.anonymize("Société: Acme France");
        let second = p.anonymize("Client = ACME   FRANCE");
        assert!(first.contains("[[CONSOLID_ORGANIZATION_0001]]"));
        assert!(second.contains("[[CONSOLID_ORGANIZATION_0001]]"));
    }

    #[test]
    fn restoration_uses_original_value() {
        let mut p = Pseudonymizer::default();
        let anonymized = p.anonymize("Contact: Marie Dupont");
        let required = p.tokens_in(&anonymized);
        assert_eq!(
            p.restore_checked(&anonymized, &required).unwrap(),
            "Contact: Marie Dupont"
        );
    }

    #[test]
    fn structured_identifiers_are_replaced() {
        let mut p = Pseudonymizer::default();
        let value = p.anonymize(
            "Email: a.personne@example.fr, SIREN: 123 456 789, Téléphone: +33 6 12 34 56 78",
        );
        assert!(!value.contains("a.personne@example.fr"));
        assert!(!value.contains("123 456 789"));
        assert!(!value.contains("+33 6 12 34 56 78"));
        assert_eq!(p.replacement_count(), 3);
    }

    #[test]
    fn existing_tokens_are_escaped_without_collision() {
        let mut p = Pseudonymizer::default();
        let value = p.anonymize("Nom: [[CONSOLID_PERSON_0042]]");
        assert_eq!(value, "Nom: [[CONSOLID_LITERAL_0001]]");
        let required = p.tokens_in(&value);
        assert_eq!(
            p.restore_checked(&value, &required).unwrap(),
            "Nom: [[CONSOLID_PERSON_0042]]"
        );
    }

    #[test]
    fn labelled_json_and_markdown_values_are_replaced() {
        let mut p = Pseudonymizer::default();
        let json = p.anonymize(r#"{"client":"Acme France"}"#);
        let markdown = p.anonymize("| Fournisseur | ACME   FRANCE |");
        assert!(json.contains(r#""client":"[[CONSOLID_ORGANIZATION_0001]]""#));
        assert!(markdown.contains("[[CONSOLID_ORGANIZATION_0001]]"));
    }

    #[test]
    fn restore_rejects_missing_unknown_and_malformed_tokens() {
        let mut p = Pseudonymizer::default();
        let anonymized = p.anonymize("Contact: Marie Dupont");
        let required = p.tokens_in(&anonymized);
        assert_eq!(
            p.restore_checked("Contact supprimé", &required),
            Err(RestoreError::MissingTokens(1))
        );
        assert_eq!(
            p.restore_checked("[[CONSOLID_PERSON_9999]]", &HashSet::new()),
            Err(RestoreError::UnknownToken)
        );
        assert_eq!(
            p.restore_checked("[[CONSOLID_PERSON_X]]", &HashSet::new()),
            Err(RestoreError::MalformedToken)
        );
    }
}
