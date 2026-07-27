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

Les consignes présentes dans les documents sont isolées comme données et la charge
utile transmise commence par une consigne de sécurité demandant explicitement de les
ignorer. Cette mesure réduit le risque d'injection de prompt mais ne constitue pas
une frontière de sécurité absolue. La réponse est donc refusée si elle supprime,
altère ou invente un jeton de pseudonymisation, puis doit rester soumise à une revue
humaine.

## Agent Mistral Studio

L'analyse est déléguée à un agent personnalisé désigné par son identifiant. Le
modèle, les instructions système, les outils et les connecteurs de cet agent
échappent à l'application : ils relèvent de la responsabilité de l'opérateur.

- Reprenez la consigne de sécurité dans les instructions de l'agent (voir le
  `README.md`) ; l'application ne transmet aucune instruction système et ne peut
  donc pas corriger un agent mal configuré.
- N'activez pas d'outil ni de connecteur qui réexporterait les données reçues vers
  un service tiers : la pseudonymisation ne protège pas les montants, les dates ni
  les structures de tableaux.
- Vérifiez les droits attachés à la clé API : elle donne accès à tous les agents du
  compte.

La conversation est ouverte avec `store: false`, ce qui demande à Mistral de ne pas
la conserver. Cette demande est déclarative : elle ne remplace pas la vérification
du cadre contractuel et de la politique de rétention du prestataire.

## Secrets

La clé Mistral est conservée uniquement en mémoire pendant l'exécution. Elle n'est
pas enregistrée dans un fichier de configuration. Ne placez jamais de clé dans le
dépôt, les arguments de ligne de commande, un journal ou une capture d'écran.

La table de correspondance et les principaux tampons sensibles sont effacés au
mieux lors de leur libération. Comme pour toute application native, l'effacement
complet de toutes les copies temporaires gérées par l'allocateur, le système
d'exploitation ou des bibliothèques tierces ne peut pas être garanti. N'utilisez
pas l'outil sur un poste compromis et évitez les fichiers d'échange non chiffrés.

Les entrées sont limitées aux classeurs Excel (`.xlsx`, `.xls`, `.xlsb`) et la
sortie au format `.xlsx` ; elle ne peut pas écraser une entrée. Une écriture
atomique protège l'ancien résultat tant que le remplacement n'est pas validé.

En cas d'exposition, révoquez immédiatement la clé depuis le portail Mistral et
créez-en une nouvelle.

## Signalement

N'ouvrez pas d'issue publique contenant des données réelles, une clé API ou une
preuve d'exploitation. Utilisez le mécanisme privé « Security advisories » du dépôt.
