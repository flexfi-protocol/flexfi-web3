// 3-test-score-simple.js
const { 
  Connection, 
  PublicKey, 
  Keypair, 
  sendAndConfirmTransaction, 
  Transaction,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY
} = require('@solana/web3.js');
const BN = require('bn.js');

// Programme FlexFi déployé
const FLEXFI_PROGRAM_ID = new PublicKey('7Yd4fxojkMc9ZvCiewx7coorSnFm84VccBiNxX3hujUH');

// Seeds pour les PDAs
const WHITELIST_SEED = 'whitelist';
const SCORE_SEED = 'score';

// ANALYSE BASÉE SUR LES TESTS QUI MARCHENT :
// - Le test de staking utilise des discriminants simples (0, 1)
// - DepositStaking = 0, WithdrawStaking = 1
// - Ces instructions correspondent aux premières variantes de l'enum
//
// EN REGARDANT LE PROCESSOR.RS :
// Les instructions semblent être traitées dans l'ordre de l'enum FlexfiInstruction
// InitializeScore, UpdateScore, GetScore sont probablement à des positions séquentielles

// Créer des instructions avec discriminants simples (comme staking)
function createSimpleInstruction(discriminant) {
  const buffer = Buffer.alloc(1);
  buffer.writeUInt8(discriminant, 0);
  return buffer;
}

function createUpdateScoreInstruction(discriminant, change) {
  const buffer = Buffer.alloc(3); // 1 byte discriminant + 2 bytes i16
  buffer.writeUInt8(discriminant, 0);
  buffer.writeInt16LE(change, 1);
  return buffer;
}

// Classe pour tester le système de score
class ScoreTester {
  constructor(connection, programId) {
    this.connection = connection;
    this.programId = programId;
  }

  async setupTestWallet() {
    console.log('🔧 Setup du wallet de test pour le score');

    // Utiliser le wallet whitelisté
    const WHITELISTED_PRIVATE_KEY = [
      174,95,224,249,206,157,168,36,74,81,125,89,80,32,106,171,64,175,198,95,195,42,134,238,197,14,27,149,243,105,69,46,17,85,103,72,94,139,195,84,123,35,39,39,67,168,233,90,90,150,250,11,191,253,204,211,25,208,211,82,162,1,193,108
    ];
    
    const testKeypair = Keypair.fromSecretKey(Uint8Array.from(WHITELISTED_PRIVATE_KEY));
    console.log(`   🔑 Wallet whitelisté: ${testKeypair.publicKey.toBase58()}`);

    // Vérifier le solde SOL
    const balance = await this.connection.getBalance(testKeypair.publicKey);
    console.log(`   Solde SOL: ${balance / 1e9} SOL`);

    return testKeypair;
  }

  async calculateScorePDAs(userKeypair) {
    console.log('🔍 Calcul des PDAs de score...');

    // User Status PDA (pour whitelist)
    const [userStatusAccount] = await PublicKey.findProgramAddress(
      [Buffer.from(WHITELIST_SEED), userKeypair.publicKey.toBuffer()],
      this.programId
    );

    // Score Account PDA
    const [scoreAccount] = await PublicKey.findProgramAddress(
      [
        Buffer.from(SCORE_SEED),
        userKeypair.publicKey.toBuffer()
      ],
      this.programId
    );

    console.log(`   User Status PDA: ${userStatusAccount.toBase58()}`);
    console.log(`   Score Account: ${scoreAccount.toBase58()}`);

    return {
      userStatusAccount,
      scoreAccount
    };
  }

  async testInstructionDiscriminants(userKeypair, pdas) {
    console.log('\n🧪 Test des discriminants pour InitializeScore...');
    
    const { userStatusAccount, scoreAccount } = pdas;
    
    // Tester différents discriminants potentiels
    // Basé sur l'ordre probable dans l'enum FlexfiInstruction
    const discriminantsToTest = [
      { value: 3, description: "InitializeScore (position 3 dans l'enum)" },
      { value: 6, description: "InitializeScore si d'autres instructions avant" },
      { value: 4, description: "Alternative 1" },
      { value: 5, description: "Alternative 2" },
      { value: 7, description: "Alternative 3" },
      { value: 8, description: "Alternative 4" },
    ];

    for (const discriminant of discriminantsToTest) {
      console.log(`\n🔍 Test discriminant ${discriminant.value}: ${discriminant.description}`);
      
      const instructionData = createSimpleInstruction(discriminant.value);
      console.log(`   Data: ${instructionData.toString('hex')}`);
      
      const initScoreIx = {
        keys: [
          { pubkey: scoreAccount, isSigner: false, isWritable: true },
          { pubkey: userKeypair.publicKey, isSigner: true, isWritable: true },
          { pubkey: userStatusAccount, isSigner: false, isWritable: false },
          { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
          { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false }
        ],
        programId: this.programId,
        data: instructionData
      };

      try {
        const simulation = await this.connection.simulateTransaction(
          new Transaction().add(initScoreIx),
          [userKeypair]
        );

        if (simulation.value.err) {
          console.log(`   ❌ Discriminant ${discriminant.value} échoué:`, simulation.value.err);
          if (simulation.value.logs) {
            const errorLog = simulation.value.logs.find(log => 
              log.includes('Error') || log.includes('failed')
            );
            if (errorLog) console.log(`      ${errorLog}`);
          }
        } else {
          console.log(`   ✅ Discriminant ${discriminant.value} réussi !`);
          console.log('   Logs:');
          simulation.value.logs?.forEach(log => console.log(`      ${log}`));
          return discriminant.value;
        }
      } catch (error) {
        console.log(`   ❌ Discriminant ${discriminant.value} erreur:`, error.message);
      }
    }
    
    return null;
  }

  async initializeScore(userKeypair, pdas, discriminant) {
    console.log(`\n💯 Initialisation du score avec discriminant ${discriminant}...`);

    const { userStatusAccount, scoreAccount } = pdas;
    const instructionData = createSimpleInstruction(discriminant);

    const initScoreIx = {
      keys: [
        { pubkey: scoreAccount, isSigner: false, isWritable: true },
        { pubkey: userKeypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: userStatusAccount, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false }
      ],
      programId: this.programId,
      data: instructionData
    };

    console.log('🚀 Envoi de la transaction...');
    const tx = new Transaction().add(initScoreIx);
    const signature = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [userKeypair],
      { skipPreflight: false, commitment: 'confirmed' }
    );

    console.log(`✅ Score initialisé !`);
    console.log(`   Signature: ${signature}`);
    console.log(`   Explorer: https://solscan.io/tx/${signature}?cluster=devnet`);

    return signature;
  }

  async testGetScore(userKeypair, pdas, initDiscriminant) {
    console.log(`\n📊 Test de consultation du score...`);

    const { scoreAccount } = pdas;
    
    // GetScore est probablement le discriminant suivant InitializeScore
    const getScoreDiscriminants = [
      initDiscriminant + 2, // Si UpdateScore est au milieu
      initDiscriminant + 1, // Si GetScore suit directement
      5, // Valeur fixe possible
      8, // Autre valeur possible
    ];

    for (const discriminant of getScoreDiscriminants) {
      console.log(`\n🔍 Test GetScore avec discriminant ${discriminant}`);
      
      const instructionData = createSimpleInstruction(discriminant);
      console.log(`   Data: ${instructionData.toString('hex')}`);
      
      const getScoreIx = {
        keys: [
          { pubkey: scoreAccount, isSigner: false, isWritable: false },
          { pubkey: userKeypair.publicKey, isSigner: false, isWritable: false }
        ],
        programId: this.programId,
        data: instructionData
      };

      try {
        const simulation = await this.connection.simulateTransaction(
          new Transaction().add(getScoreIx),
          [userKeypair]
        );

        if (simulation.value.err) {
          console.log(`   ❌ GetScore ${discriminant} échoué:`, simulation.value.err);
        } else {
          console.log(`   ✅ GetScore ${discriminant} réussi !`);
          console.log('   Logs:');
          simulation.value.logs?.forEach(log => {
            console.log(`      ${log}`);
            // Chercher le score dans les logs
            if (log.includes('User score:') || log.includes('score')) {
              const match = log.match(/(\d+)/);
              if (match) {
                console.log(`   🎯 Score trouvé: ${match[1]} points`);
              }
            }
          });
          return discriminant;
        }
      } catch (error) {
        console.log(`   ❌ GetScore ${discriminant} erreur:`, error.message);
      }
    }
    
    return null;
  }

  async verifyScore(pdas) {
    console.log('\n🔍 Vérification du compte de score...');

    const { scoreAccount } = pdas;
    const scoreInfo = await this.connection.getAccountInfo(scoreAccount);
    
    if (scoreInfo && scoreInfo.owner.equals(this.programId)) {
      console.log('✅ Compte de score créé');
      console.log(`   Propriétaire: ${scoreInfo.owner.toBase58()}`);
      console.log(`   Taille: ${scoreInfo.data.length} bytes`);
      console.log(`   Rent-exempt: ${scoreInfo.lamports} lamports`);
      console.log(`   Données (hex): ${scoreInfo.data.toString('hex')}`);

      // Essayer de décoder les premières données (structure basique)
      if (scoreInfo.data.length >= 34) {
        try {
          const owner = new PublicKey(scoreInfo.data.slice(0, 32));
          const score = scoreInfo.data.readUInt16LE(32);
          console.log(`   📊 Owner décodé: ${owner.toBase58()}`);
          console.log(`   📊 Score décodé: ${score} points`);
        } catch (error) {
          console.log(`   ⚠️ Erreur décodage: ${error.message}`);
        }
      }
      return true;
    } else {
      console.log('❌ Compte de score non trouvé');
      return false;
    }
  }
}

async function main() {
  console.log('🚀 TEST SYSTÈME DE SCORE FlexFi - VERSION SIMPLE');
  console.log('================================================\n');

  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  const tester = new ScoreTester(connection, FLEXFI_PROGRAM_ID);

  try {
    // ÉTAPE 1: Setup du wallet de test
    console.log('🔵 ÉTAPE 1: Setup du wallet de test');
    const userKeypair = await tester.setupTestWallet();

    // ÉTAPE 2: Calculer les PDAs
    console.log('\n🔵 ÉTAPE 2: Calcul des PDAs de score');
    const pdas = await tester.calculateScorePDAs(userKeypair);

    // ÉTAPE 3: Vérifier si le score existe déjà
    console.log('\n🔵 ÉTAPE 3: Vérification préalable');
    let scoreExists = await tester.verifyScore(pdas);

    if (!scoreExists) {
      // ÉTAPE 4: Trouver le bon discriminant pour InitializeScore
      console.log('\n🔵 ÉTAPE 4: Recherche du discriminant InitializeScore');
      const initDiscriminant = await tester.testInstructionDiscriminants(userKeypair, pdas);
      
      if (initDiscriminant !== null) {
        console.log(`\n✅ Discriminant InitializeScore trouvé: ${initDiscriminant}`);
        
        // ÉTAPE 5: Initialiser le score
        console.log('\n🔵 ÉTAPE 5: Initialisation du score');
        await tester.initializeScore(userKeypair, pdas, initDiscriminant);
        
        // ÉTAPE 6: Vérification du compte créé
        console.log('\n🔵 ÉTAPE 6: Vérification du compte créé');
        scoreExists = await tester.verifyScore(pdas);
        
        // ÉTAPE 7: Test de consultation du score
        console.log('\n🔵 ÉTAPE 7: Test consultation du score');
        await tester.testGetScore(userKeypair, pdas, initDiscriminant);
      } else {
        console.log('\n❌ Aucun discriminant ne fonctionne pour InitializeScore');
        console.log('\n💡 POSSIBILITÉS:');
        console.log('1. L\'instruction nécessite des comptes supplémentaires');
        console.log('2. Le discriminant est en dehors de la plage testée');
        console.log('3. L\'instruction utilise un format Borsh complet');
        console.log('4. Il y a une erreur dans l\'ordre des accounts');
      }
    } else {
      console.log('📝 Score déjà existant, test de consultation...');
      
      // ÉTAPE 4: Test de consultation sur score existant
      console.log('\n🔵 ÉTAPE 4: Test consultation du score existant');
      await tester.testGetScore(userKeypair, pdas, 3); // Essaye avec discriminant 3
    }

    // RÉSUMÉ
    console.log('\n🎉 TEST TERMINÉ');
    console.log('===============');
    console.log(`✅ Wallet: ${userKeypair.publicKey.toBase58()}`);
    console.log(`✅ Score Account: ${pdas.scoreAccount.toBase58()}`);
    console.log(`✅ Status: ${scoreExists ? 'Existant ou créé' : 'Échec création'}`);

  } catch (error) {
    console.error('\n💥 ERREUR DANS LE TEST:', error);
    
    if (error.logs) {
      console.log('Logs blockchain:', error.logs);
    }
    
    console.log('\n🔧 SUGGESTIONS DE DEBUG:');
    console.log('========================');
    console.log('1. Vérifier si tous les comptes sont corrects');
    console.log('2. S\'assurer que le wallet est whitelisté');
    console.log('3. Vérifier l\'ordre des comptes dans l\'instruction');
    console.log('4. Examiner si des seeds supplémentaires sont nécessaires');
    
    process.exit(1);
  }
}

// Notes pour comprendre le système
console.log('📚 INFORMATIONS SUR LE TEST:');
console.log('============================');
console.log('Ce test utilise la même approche que le staking qui fonctionne:');
console.log('- Discriminants simples (1 byte)');
console.log('- Test séquentiel de différentes valeurs');
console.log('- Simulation avant envoi pour éviter les erreurs');
console.log('');
console.log('Instructions attendues (basé sur FlexfiInstruction enum):');
console.log('- InitializeScore: Crée le compte de score');
console.log('- UpdateScore: Modifie le score (+/- points)');
console.log('- GetScore: Lit le score actuel');

main().catch(error => {
  console.error('💥 Erreur fatale:', error);
  process.exit(1);
});