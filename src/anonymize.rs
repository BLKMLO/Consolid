use regex::{Captures, Regex};
use std::collections::HashMap;

const TOKEN_PREFIX: &str = "[[CONSOLID_";

#[derive(Debug, Clone, Copy)]
struct Pattern {
    kind: &'static str,
    regex_index: usize,
}

/// Table de pseudonymisation conservée uniquement en mémoire pendant un traitement.
///
/// Deux graphies identiques, sans distinction de casse ni d'espaces superflus,
/// reçoivent le même jeton dans tous les documents du lot.
pub struct Pseudonymizer {
    regexes: Vec<Regex>,
    patterns: Vec<Pattern>,
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
                r"(?x)\b(?:\+33|0033|0)\s*[1-9](?:[\s.\-]*\d{2}){4}\b",
            ),
        ];

        let mut regexes = Vec::new();
        let mut patterns = Vec::new();
        for (kind, source) in definitions {
            let regex_index = regexes.len();
            regexes.push(Regex::new(source).expect("motif interne valide"));
            patterns.push(Pattern { kind, regex_index });
        }

        Self {
            regexes,
            patterns,
            forward: HashMap::new(),
            reverse: HashMap::new(),
            next_by_kind: HashMap::new(),
        }
    }
}

impl Pseudonymizer {
    pub fn anonymize(&mut self, input: &str) -> String {
        let mut output = input.to_owned();

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

        // Les personnes, sociétés et adresses ne sont remplacées que lorsqu'un
        // libellé explicite permet de les identifier sans détruire les données métier.
        let labelled = [
            (
                "ORGANIZATION",
                r"(?im)\b(société|societe|entreprise|company|client|fournisseur|vendor)\s*([:=]\s*)([^\r\n\t,;]{2,160})",
            ),
            (
                "PERSON",
                r"(?im)\b(nom|prénom|prenom|personne|contact|dirigeant|salarié|salarie)\s*([:=]\s*)([^\r\n\t,;]{2,120})",
            ),
            (
                "ADDRESS",
                r"(?im)\b(adresse|address)\s*([:=]\s*)([^\r\n\t;]{4,200})",
            ),
        ];

        for (kind, source) in labelled {
            let regex = Regex::new(source).expect("motif interne valide");
            output = regex
                .replace_all(&output, |caps: &Captures<'_>| {
                    let label = caps.get(1).expect("libellé").as_str();
                    let separator = caps.get(2).expect("séparateur").as_str();
                    let value = caps.get(3).expect("valeur").as_str();
                    format!("{label}{separator}{}", self.token_for(kind, value))
                })
                .into_owned();
        }

        output
    }

    pub fn restore(&self, input: &str) -> String {
        let mut output = input.to_owned();
        let mut mappings: Vec<_> = self.reverse.iter().collect();
        mappings.sort_unstable_by_key(|(token, _)| std::cmp::Reverse(token.len()));
        for (token, original) in mappings {
            output = output.replace(token, original);
        }
        output
    }

    pub fn replacement_count(&self) -> usize {
        self.reverse.len()
    }

    fn token_for(&mut self, kind: &str, original: &str) -> String {
        if original.starts_with(TOKEN_PREFIX) {
            return original.to_owned();
        }

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
        assert_eq!(p.restore(&anonymized), "Contact: Marie Dupont");
    }

    #[test]
    fn structured_identifiers_are_replaced() {
        let mut p = Pseudonymizer::default();
        let value = p.anonymize("Email: a.personne@example.fr, SIREN: 123 456 789");
        assert!(!value.contains("a.personne@example.fr"));
        assert!(!value.contains("123 456 789"));
        assert_eq!(p.replacement_count(), 2);
    }

    #[test]
    fn existing_tokens_are_not_reprocessed() {
        let mut p = Pseudonymizer::default();
        let value = p.anonymize("Nom: [[CONSOLID_PERSON_0042]]");
        assert_eq!(value, "Nom: [[CONSOLID_PERSON_0042]]");
    }
}
