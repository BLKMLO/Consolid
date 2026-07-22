//! Anonymization patterns for French and international data

use once_cell::sync::Lazy;
use regex::Regex;

/// All patterns for anonymization
/// Each pattern is a tuple of (name, regex_pattern)
pub static ANONYMIZATION_PATTERNS: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    vec![
        // French specific patterns
        
        // SIREN (9 digits)
        ("siren", r"\b\d{9}\b"),
        
        // SIRET (14 digits)
        ("siret", r"\b\d{14}\b"),
        
        // French phone numbers
        ("phone_fr", r"\b(?:\+33|0)[1-9](?:\d{2}){4}\b"),
        ("phone_fr_spaced", r"\b(?:\+33|0)[1-9](?:[\s.-]?\d{2}){4}\b"),
        
        // French mobile numbers
        ("phone_fr_mobile", r"\b(?:\+33|0)[67]\d{8}\b"),
        
        // Email addresses
        ("email", r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b"),
        
        // IBAN (French and international)
        ("iban", r"\bFR\d{2}[A-Za-z0-9]{11,30}\b"),
        ("iban_international", r"\b[A-Za-z]{2}\d{2}[A-Za-z0-9]{4,30}\b"),
        
        // Credit card numbers (basic pattern)
        ("credit_card", r"\b(?:\d{4}[\s-]?){3}\d{4}\b"),
        
        // Names (French common patterns)
        ("name_french", r"\b(?:M|Mme|Mlle|Mr|Madame|Mademoiselle|Monsieur)\s+[A-ZÀ-ÿ][a-zà-ÿ'-]+\b"),
        
        // First names (common French names)
        ("first_name", r"\b(?:Jean|Marie|Pierre|Paul|Jacques|Nicolas|François|Christophe|David|Thomas|Vincent|Julien|Alexandre|Laurent|Mathieu|Cédric|Sébastien|Benoît|Guillaume|Romain|Arnaud|Jérôme|Yann|Kévin|Jonathan|Damien|Anthony|Renaud|Florent|Maxime|Antony|Bruno|Philippe|Éric|Stéphane|Nicolas|Cyril|Boris|Grégory|Loïc|Sylvain|Olivier|Fabrice|Karim|Youssef|Mohamed|Ahmed|Mehdi|Redouane|Driss|Samir|Mustapha|Ali|Amine|Sophie|Cécile|Élodie|Laetitia|Julie|Émilie|Claire|Coralie|Mélanie|Audrey|Sandrine|Nathalie|Christelle|Aurélie|Marine|Lydie|Céline|Amélie|Manon|Léa|Chloé|Camélia|Justine|Élodie|Marion|Anaïs|Ophélie|Clémence|Pauline|Adeline|Émeline|Lucie|Anaë|Maëlle|Léane|Noémie|Jade|Inès|Louise|Emma|Alice|Chloé|Lina|Mia|Anna|Jade|Léa|Manon|Inès|Zoé|Camille|Lucie|Léna|Jeanne|Juliette|Rose|Agathe|Capucine|Constance|Lou|Maëlys|Jana|Lina|Mia|Noa|Sacha|Sarah|Zoé)\b"),
        
        // Last names (common French last names)
        ("last_name", r"\b(?:Martin|Bernard|Dubois|Thomas|Robert|Richard|Petit|Durand|Leroy|Moreau|Simon|Laurent|Lefèvre|Michel|Garcia|David|Berthelot|Roux|Vasseur|Blanc|Guérin|Chevalier|François|Girard|Bonnet|Dupont|Lambert|Fontaine|Rousseau|Vincent|Müller|Lopez|Moreau|Fournier|Garnier|Perez|Rousseau|Blanc|Guillaume|Mercier|Diaz|Schmitt|Fernandez|Dumont|Rogers|Legrand|Bauer|Morel|Gauthier|André|Chevallier|Arnaud|Adam|Boyer|Giraud|Perrin|Moulin|Lemoine|Gallois|Denis|Lacroix|Colin|Legendre|Renaud|Duval|Bigot|Martinez|Gerard|Rmy|Aubert|Gomes|Fournier|Sanchez|Dufour|Brun|Lefebvre|Merci|Houston|Nguyen|Wong|Kim|Lee|Park|Wang|Zhang|Li|Chen|Yang|Wu|Liu|Zhao|Huang|Zhou|Xu|Sun|Ma|Hu|Lin|He|Gao|Xiao|Zheng|Liang|Xu|Han|Deng|Feng|Cao|Peng|Zhu|Wei|Tian|Yuan|Hua|Fu|Yu|Xia|Zeng|Xie|Zhong|Tan|Fan|Kuang|Liao|Yan|Zhao|Qin|Jiang|Shi|Cui|Lu|Yao|Zong|Tang)\b"),
        
        // Addresses (French format)
        ("address_fr", r"\b\d{1,5}\s+(?:rue|avenue|boulevard|allée|impasse|place|quai|route|chemin|sentier)\s+[A-Za-zÀ-ÿ\s'-]+\b"),
        
        // Postal codes (French)
        ("postal_code_fr", r"\b\d{5}\b"),
        
        // Dates (various formats)
        ("date_ddmmyyyy", r"\b(0[1-9]|[12][0-9]|3[01])/(0[1-9]|1[0-2])/\d{4}\b"),
        ("date_yyyymmdd", r"\b\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])\b"),
        ("date_ddmmyy", r"\b(0[1-9]|[12][0-9]|3[01])/(0[1-9]|1[0-2])/\d{2}\b"),
        
        // French social security number (partially)
        ("ssn_fr", r"\b\d{13,15}\b"),
        
        // Company names (common patterns)
        ("company_name", r"\b(?:SARL|SA|EURL|SAS|SASU|SCI|SNC|GIE)\s+[A-ZÀ-ÿ][A-Za-zÀ-ÿ\s'-]+\b"),
        
        // Generic patterns
        
        // IP addresses
        ("ip_address", r"\b(?:\d{1,3}\.){3}\d{1,3}\b"),
        
        // URLs
        ("url", r"\bhttps?://[^\s]+\b"),
        
        // UUIDs
        ("uuid", r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b"),
        
        // Generic phone numbers (international)
        ("phone_international", r"\b\+\d{1,3}[\s.-]?\d{1,14}(?:[\s.-]?\d{1,13})?\b"),
        
        // Generic ID numbers (sequences of digits)
        ("id_number", r"\b\d{6,20}\b"),
        
        // Credit card with spaces
        ("credit_card_spaced", r"\b(?:\d{4}\s+){3}\d{4}\b"),
        
        // Credit card with dashes
        ("credit_card_dashed", r"\b(?:\d{4}-){3}\d{4}\b"),
        
        // French TVA number
        ("tva_fr", r"\bFR\d{2}\s?\d{9}\b"),
        
        // Generic TVA number
        ("tva", r"\b[A-Za-z]{2}\d{2,12}\b"),
        
        // Account numbers (generic)
        ("account_number", r"\b\d{4,20}\b"),
        
        // Amounts with currency
        ("amount_eur", r"\b\d{1,3}(?:\s?\d{3})*(?:,\d{2})?\s?€\b"),
        ("amount_usd", r"\b\$\d{1,3}(?:\s?\d{3})*(?:,\d{2})?\b"),
        ("amount_generic", r"\b\d{1,3}(?:\s?\d{3})*(?:,\d{2})?\s?(?:€|\$|£|¥|CHF|CAD|AUD)\b"),
    ]
});

/// Test patterns
#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    
    #[test]
    fn test_patterns_compile() {
        for (name, pattern) in ANONYMIZATION_PATTERNS.iter() {
            assert!(Regex::new(pattern).is_ok(), "Pattern '{}' failed to compile", name);
        }
    }
    
    #[test]
    fn test_email_pattern() {
        let pattern = ANONYMIZATION_PATTERNS.iter()
            .find(|(name, _)| *name == "email")
            .map(|(_, p)| p)
            .unwrap();
        
        let re = Regex::new(pattern).unwrap();
        assert!(re.is_match("test@example.com"));
        assert!(re.is_match("user.name+tag@sub.domain.co.uk"));
    }
    
    #[test]
    fn test_french_phone_pattern() {
        let pattern = ANONYMIZATION_PATTERNS.iter()
            .find(|(name, _)| *name == "phone_fr")
            .map(|(_, p)| p)
            .unwrap();
        
        let re = Regex::new(pattern).unwrap();
        assert!(re.is_match("0123456789"));
        assert!(re.is_match("+33123456789"));
    }
}
