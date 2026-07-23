# Politique de sécurité

## Données traitées

Consolid transmet des contenus à un prestataire d'IA externe. L'opérateur doit
vérifier le cadre contractuel, la classification des données, la base légale, la
durée de conservation du prestataire et les règles de son organisation avant usage.

La pseudonymisation n'est pas une garantie d'anonymat :

- les montants, dates, structures de tableaux et éléments non détectés restent
  présents ;
- une personne ou société non précédée d'un libellé reconnu peut ne pas être
  remplacée ;
- une combinaison de données indirectes peut permettre une réidentification.

Testez impérativement l'outil sur un jeu représentatif non sensible avant
production et contrôlez les contenus à transmettre.

## Secrets

La clé Mistral est conservée uniquement en mémoire pendant l'exécution. Elle n'est
pas enregistrée dans un fichier de configuration. Ne placez jamais de clé dans le
dépôt, les arguments de ligne de commande, un journal ou une capture d'écran.

En cas d'exposition, révoquez immédiatement la clé depuis le portail Mistral et
créez-en une nouvelle.

## Signalement

N'ouvrez pas d'issue publique contenant des données réelles, une clé API ou une
preuve d'exploitation. Utilisez le mécanisme privé « Security advisories » du dépôt.
