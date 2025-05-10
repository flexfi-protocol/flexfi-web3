FlexFi - Plateforme de BNPL sur Solana
FlexFi est un programme Solana qui implémente une solution de "Buy Now, Pay Later" (BNPL) adossée à des crypto-monnaies. Ce système permet aux utilisateurs de payer en plusieurs fois tout en utilisant leurs actifs crypto comme collatéral.
Architecture du Projet
L'architecture du programme FlexFi est organisée en plusieurs modules qui gèrent chacun un aspect spécifique de la plateforme.
src/
├── core/            # Fonctionnalités de base
│   ├── wallet.rs    # Gestion des portefeuilles utilisateurs 
│   ├── staking.rs   # Gestion du staking de collatéral
│   └── mod.rs       # Exports du module core
│
├── bnpl/            # Logique de Buy Now, Pay Later
│   ├── checker.rs   # Vérification d'éligibilité au BNPL
│   ├── contract.rs  # Création et gestion des contrats BNPL
│   ├── repayment.rs # Logique de remboursement et prélèvement automatique
│   └── mod.rs       # Exports du module BNPL
│
├── card/            # Gestion des cartes virtuelles
│   ├── config.rs    # Configuration et paramètres des cartes
│   ├── manager.rs   # Émission et upgrade des cartes
│   └── mod.rs       # Exports du module card
│
├── nft/             # Fonctionnalités NFT
│   ├── mint.rs      # Création des NFT
│   ├── attach.rs    # Attachement des NFT aux cartes
│   ├── perks.rs     # Avantages liés aux NFT
│   └── mod.rs       # Exports du module NFT
│
├── score/           # Système de crédit scoring
│   ├── contract.rs  # Initialisation et mise à jour du score
│   ├── query.rs     # Consultation du score et statistiques
│   └── mod.rs       # Exports du module score
│
├── yield_module/    # Gestion des rendements
│   ├── router.rs    # Routage des rendements selon stratégie
│   ├── tracker.rs   # Suivi et réclamation des rendements
│   └── mod.rs       # Exports du module yield
│
├── state/           # Structures de données pour les comptes
│   ├── wallet.rs    # Structure du compte wallet
│   ├── staking.rs   # Structure du compte staking
│   ├── bnpl.rs      # Structure du contrat BNPL
│   ├── card.rs      # Structure du compte carte
│   ├── nft.rs       # Structure des métadonnées NFT
│   ├── score.rs     # Structure du compte score
│   ├── yield_.rs    # Structure des comptes de rendement
│   └── mod.rs       # Exports des structures de données
│
├── entrypoint.rs    # Point d'entrée du programme Solana
├── processor.rs     # Traitement des instructions
├── instructions.rs  # Définition des instructions
├── error.rs         # Codes d'erreur personnalisés
├── constants.rs     # Constantes utilisées dans le programme
└── lib.rs           # Exports principaux et configuration du programme
Description des composants principaux
Core

wallet.rs : Gère l'initialisation et la désactivation des portefeuilles utilisateurs. Chaque utilisateur doit créer un portefeuille pour accéder aux fonctionnalités FlexFi.
staking.rs : Permet aux utilisateurs de déposer des tokens (USDC) comme collatéral, de les verrouiller pour une période définie, et de les retirer une fois la période terminée.

BNPL (Buy Now, Pay Later)

checker.rs : Vérifie si un utilisateur est éligible pour un prêt BNPL en fonction de son staking, de son score, et d'autres critères.
contract.rs : Crée et gère les contrats BNPL, calcule les frais et échéances, et traite les transactions.
repayment.rs : Gère les remboursements et implémente un système de prélèvement automatique en cas de retard, en utilisant le collatéral de l'utilisateur.

Card

config.rs : Définit les différents types de cartes (Standard, Silver, Gold, Platinum) et leurs caractéristiques (frais, limites, avantages).
manager.rs : Permet la création, la mise à jour et la gestion des cartes virtuelles des utilisateurs.

NFT

mint.rs : Permet de créer des NFT représentant des avantages pour les utilisateurs.
attach.rs : Permet d'attacher des NFT aux cartes pour débloquer des avantages supplémentaires.
perks.rs : Définit et gère les avantages liés aux différents types de NFT (réduction de frais, augmentation de limite, etc.).

Score

contract.rs : Initialise et met à jour le score de crédit des utilisateurs en fonction de leur comportement de paiement.
query.rs : Permet de consulter le score et les statistiques de paiement d'un utilisateur.

Yield Module

router.rs : Gère le routage des rendements générés selon la stratégie choisie par l'utilisateur.
tracker.rs : Suit les rendements générés et permet aux utilisateurs de les réclamer.

State
Contient toutes les structures de données pour les différents types de comptes Solana du programme :

wallet.rs : Structure WalletAccount
staking.rs : Structure StakingAccount et énumération StakingStatus
bnpl.rs : Structure BNPLContractAccount et énumération BNPLStatus
card.rs : Structure CardAccount
nft.rs : Structures NFTMetadataAccount et NFTAttachmentAccount
score.rs : Structure ScoreAccount
yield_.rs : Structure YieldAccount et énumération YieldStrategy

Autres fichiers importants

entrypoint.rs : Point d'entrée standard pour un programme Solana.
processor.rs : Traite les différentes instructions reçues par le programme.
instructions.rs : Définit les instructions que le programme peut exécuter.
error.rs : Définit les codes d'erreur personnalisés.
constants.rs : Contient les constantes utilisées dans tout le programme.

Fonctionnement global

Un utilisateur crée un portefeuille FlexFi
Il stake des tokens USDC comme collatéral
Son score de crédit est initialisé
En fonction de son staking et de son score, il peut accéder à différents types de cartes
Il peut effectuer des achats en utilisant BNPL
Les remboursements sont effectués selon l'échéancier convenu
Son score évolue en fonction de son comportement de remboursement
Le staking génère des rendements qui peuvent être réclamés ou réinvestis

Scénarios de Test
Scénario 1: Intégration d'un nouvel utilisateur
Ce scénario teste le flux d'intégration d'un nouvel utilisateur à la plateforme FlexFi.

Création du wallet

Créer un nouveau wallet avec l'ID backend
Vérifier que le wallet est correctement initialisé


Dépôt de staking

Déposer 10 USDC de staking
Vérifier que le staking est correctement verrouillé pour 30 jours


Initialisation du score

Initialiser le score de crédit
Vérifier que le score initial est de 500


Accès à la carte Standard

Vérifier que l'utilisateur a accès à une carte Standard
Vérifier que les échéances 3x, 4x, 6x sont disponibles
Vérifier que le taux de frais est de 7%


Consultation du score

Consulter le score de crédit
Vérifier que les statistiques de paiement sont correctes



Scénario 2: Premier achat BNPL et remboursement complet
Ce scénario teste le processus d'achat avec BNPL et le remboursement complet d'un prêt.

Préparation

Créer un wallet et staker 50 USDC
Initialiser le score de crédit


Création d'un contrat BNPL

Créer un contrat BNPL de 30 USDC à payer en 3 fois
Vérifier que le montant par échéance est correctement calculé
Vérifier que les frais sont correctement ajoutés (7% pour une carte Standard)


Paiement de la première échéance

Effectuer le paiement de la première échéance
Vérifier que le contrat est mis à jour (paid_installments = 1)
Vérifier que le score est augmenté (+5 points)


Paiement des échéances restantes

Effectuer le paiement des échéances restantes
Vérifier que le contrat est marqué comme complété
Vérifier que le score est augmenté (+5 points par paiement à l'heure, +20 points pour la complétion)


Vérification finale

Vérifier que le score final est de 535 (500 initial + 5*3 paiements + 20 complétion)
Vérifier que les statistiques de paiement sont mises à jour (3 paiements à l'heure, 0 retard)



Scénario 3: Paiement en retard et déstaking automatique
Ce scénario teste le mécanisme de prélèvement automatique en cas de retard de paiement.

Préparation

Créer un wallet et staker 100 USDC
Initialiser le score de crédit
Créer un contrat BNPL de 50 USDC à payer en 4 fois


Simulation d'un paiement manqué

Avancer l'horloge au-delà de la date d'échéance + période de grâce (15 jours)
Vérifier que le compte utilisateur n'a pas de fonds pour le paiement


Déclenchement du déstaking automatique

Appeler process_check_repayment
Vérifier que le système prélève automatiquement depuis le staking
Vérifier qu'une pénalité est appliquée (par exemple, 10%)


Vérification de l'impact sur le score

Vérifier que le score est réduit (-20 points pour un paiement en retard avec déstaking)
Vérifier que les statistiques de paiement sont mises à jour (1 paiement en retard)


Vérification du staking

Vérifier que le montant de staking est réduit du montant de l'échéance + pénalité
Vérifier que le montant restant est correct



Ces scénarios couvrent les principales fonctionnalités du programme FlexFi et permettent de vérifier que les différents composants fonctionnent correctement ensemble.