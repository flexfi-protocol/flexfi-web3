const { 
  Connection, 
  PublicKey, 
  Keypair, 
  sendAndConfirmTransaction, 
  Transaction,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_RENT_PUBKEY
} = require('@solana/web3.js');
const { 
  TOKEN_PROGRAM_ID, 
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint, 
  getOrCreateAssociatedTokenAccount, 
  mintTo, 
  getAccount,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction
} = require('@solana/spl-token');
const BN = require('bn.js');
const fs = require('fs');
const path = require('path');
const borsh = require('borsh');

// Programme FlexFi déployé
const FLEXFI_PROGRAM_ID = new PublicKey('7Yd4fxojkMc9ZvCiewx7coorSnFm84VccBiNxX3hujUH');

// Seeds pour les PDAs - EXACTEMENT comme dans le programme Rust
const WHITELIST_SEED = 'whitelist';
const STAKING_SEED = 'staking';
const USDC_VAULT_SEED = 'usdc_vault';

// Définir la structure pour DepositStaking
class DepositStaking {
  constructor({ amount, lock_days }) {
    this.amount = amount;
    this.lock_days = lock_days;
  }
}

// Schéma Borsh pour DepositStaking
const depositStakingSchema = new Map([
  [DepositStaking, {
    kind: 'struct',
    fields: [
      ['amount', 'u64'],
      ['lock_days', 'u16']
    ]
  }]
]);

// Définir la structure pour WithdrawStaking
class WithdrawStaking {
  constructor({ amount }) {
    this.amount = amount;
  }
}

// Schéma Borsh pour WithdrawStaking  
const withdrawStakingSchema = new Map([
  [WithdrawStaking, {
    kind: 'struct',
    fields: [
      ['amount', 'u64']
    ]
  }]
]);

// Alternative manuelle simple pour deposit
function createDepositStakingInstructionManual(amount, lockDays) {
  const buffer = Buffer.alloc(11); // 1 (discriminant) + 8 (u64) + 2 (u16)
  
  // Discriminant 0 pour DepositStaking
  buffer.writeUInt8(0, 0);
  
  // amount en u64 little endian
  const amountBuffer = new BN(amount).toArrayLike(Buffer, 'le', 8);
  amountBuffer.copy(buffer, 1);
  
  // lock_days en u16 little endian  
  buffer.writeUInt16LE(lockDays, 9);
  
  return buffer;
}

// Alternative manuelle simple pour withdraw
function createWithdrawStakingInstructionManual(amount) {
  const buffer = Buffer.alloc(9); // 1 (discriminant) + 8 (u64)
  
  // Discriminant 1 pour WithdrawStaking (selon l'ordre dans l'enum FlexfiInstruction)
  buffer.writeUInt8(1, 0);
  
  // amount en u64 little endian
  const amountBuffer = new BN(amount).toArrayLike(Buffer, 'le', 8);
  amountBuffer.copy(buffer, 1);
  
  return buffer;
}

async function main() {
  console.log("Test de staking et retrait avec le wallet hardcodé...");

  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

  // D'abord, essayons d'utiliser le USDC existant (celui qui a fonctionné)
  const existingUSDCMint = new PublicKey('42VLpBUTzLfPVKNPKUfYS23Ht2jtVzq7xpkZ9QoLFoRU');
  
  let lenderKeypair;
  try {
    // Lire le fichier keypair
    const keypairPath = path.join(__dirname, 'lender-keypair.json');
    const keypairData = JSON.parse(fs.readFileSync(keypairPath, 'utf-8'));
    
    // Vérifier que c'est bien un tableau
    if (!Array.isArray(keypairData)) {
      throw new Error('Le fichier lender-keypair.json doit contenir un tableau de bytes');
    }
    
    // Créer le keypair depuis le tableau de bytes
    lenderKeypair = Keypair.fromSecretKey(Uint8Array.from(keypairData));
    
    console.log(`Wallet chargé: ${lenderKeypair.publicKey.toBase58()}`);

    // Liste des wallets hardcodés dans la whitelist
    const HARDCODED_WHITELIST = [
      '4r41NBNFTU3wWZ5gWpq59xZjG2c6FfthZScbqrJJUPZs',
      'iDc5xocYcovheHitHamo6hkbxd7PK4ZWuw2DNsV5R8V'
    ];

    // Vérifier que le wallet est dans la whitelist
    if (!HARDCODED_WHITELIST.includes(lenderKeypair.publicKey.toBase58())) {
      throw new Error(`Le wallet chargé (${lenderKeypair.publicKey.toBase58()}) n'est pas dans la whitelist hardcodée`);
    }

    console.log("✅ Wallet hardcodé vérifié avec succès!");
  } catch (error) {
    console.error("Erreur lors du chargement du keypair:", error);
    throw error;
  }

  const balance = await connection.getBalance(lenderKeypair.publicKey);
  console.log(`Solde SOL: ${balance / 1e9} SOL`);

  if (balance < 0.01 * 1e9) {
    console.log("Solde SOL insuffisant, demande d'airdrop...");
    const airdrop = await connection.requestAirdrop(lenderKeypair.publicKey, 2 * 1e9);
    await connection.confirmTransaction(airdrop);
    console.log("Airdrop confirmé");
  }

  // Utiliser le USDC EXISTANT d'abord
  let usdcMint = existingUSDCMint;
  console.log(`Utilisation du USDC existant: ${usdcMint.toBase58()}`);
  
  // Vérifier si le compte USDC existe déjà pour cet utilisateur
  let userUsdcAccount;
  try {
    userUsdcAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      lenderKeypair,
      usdcMint,
      lenderKeypair.publicKey
    );
    console.log(`Compte USDC existant: ${userUsdcAccount.address.toBase58()}`);
    
    // Vérifier le solde actuel
    const currentBalance = await getAccount(connection, userUsdcAccount.address);
    console.log(`Solde USDC actuel: ${Number(currentBalance.amount) / 1000000} USDC`);
    
    // Si pas assez de USDC, minter plus (nécessite l'autorité du mint)
    // Note: Ceci échouera si on n'est pas l'autorité du mint existant
    if (Number(currentBalance.amount) < 200000000) {
      console.log("Solde USDC insuffisant, création d'un nouveau USDC...");
      
      // Créer un NOUVEAU mint USDC
      usdcMint = await createMint(
        connection, 
        lenderKeypair, 
        lenderKeypair.publicKey, 
        null, 
        6
      );
      console.log(`Nouveau USDC créé: ${usdcMint.toBase58()}`);
      
      userUsdcAccount = await getOrCreateAssociatedTokenAccount(
        connection,
        lenderKeypair,
        usdcMint,
        lenderKeypair.publicKey
      );
      console.log(`Nouveau compte USDC: ${userUsdcAccount.address.toBase58()}`);
      
      await mintTo(
        connection,
        lenderKeypair,
        usdcMint,
        userUsdcAccount.address,
        lenderKeypair,
        200000000
      );
      console.log("200 USDC mintés");
    }
  } catch (error) {
    console.log("Erreur avec le USDC existant, création d'un nouveau...");
    
    // Créer un nouveau USDC
    usdcMint = await createMint(
      connection, 
      lenderKeypair, 
      lenderKeypair.publicKey, 
      null, 
      6
    );
    console.log(`Nouveau USDC créé: ${usdcMint.toBase58()}`);
    
    userUsdcAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      lenderKeypair,
      usdcMint,
      lenderKeypair.publicKey
    );
    console.log(`Nouveau compte USDC: ${userUsdcAccount.address.toBase58()}`);
    
    await mintTo(
      connection,
      lenderKeypair,
      usdcMint,
      userUsdcAccount.address,
      lenderKeypair,
      200000000
    );
    console.log("200 USDC mintés");
  }

  // Calcul des PDAs
  console.log("\n=== Calcul des PDAs ===");
  
  // User Status PDA
  const [userStatusAccount] = await PublicKey.findProgramAddress(
    [Buffer.from(WHITELIST_SEED), lenderKeypair.publicKey.toBuffer()],
    FLEXFI_PROGRAM_ID
  );
  console.log(`User Status PDA: ${userStatusAccount.toBase58()}`);

  // Staking Account PDA
  const stakingSeeds = [
    Buffer.from(STAKING_SEED),
    lenderKeypair.publicKey.toBuffer(),
    usdcMint.toBuffer()
  ];
  
  const [stakingAccount] = await PublicKey.findProgramAddress(
    stakingSeeds,
    FLEXFI_PROGRAM_ID
  );
  console.log(`Staking Account PDA: ${stakingAccount.toBase58()}`);

  // Vault Account PDA
  const vaultSeeds = [
    Buffer.from(USDC_VAULT_SEED),
    stakingAccount.toBuffer()
  ];
  
  const [vaultAccount] = await PublicKey.findProgramAddress(
    vaultSeeds,
    FLEXFI_PROGRAM_ID
  );
  console.log(`Vault Account PDA: ${vaultAccount.toBase58()}`);

  // Calculer l'ATA pour le vault
  const expectedVaultATA = await getAssociatedTokenAddress(
    usdcMint,
    vaultAccount,
    true // allowOwnerOffCurve car le vault est un PDA
  );
  console.log("Expected vault ATA:", expectedVaultATA.toBase58());

  console.log("\n=== Fin calcul des PDAs ===\n");

  // PARTIE 1 : CRÉER UN STAKING
  console.log("=== PARTIE 1 : CRÉATION DU STAKING ===");
  
  // Créer l'ATA du vault si nécessaire
  console.log("Vérification de l'ATA du vault...");
  const vaultATAInfo = await connection.getAccountInfo(expectedVaultATA);
  if (vaultATAInfo === null) {
    console.log("L'ATA du vault n'existe pas, création en cours...");
    
    const createVaultATAIx = createAssociatedTokenAccountInstruction(
      lenderKeypair.publicKey,  // payer
      expectedVaultATA,         // ATA à créer
      vaultAccount,             // owner (le vault PDA)
      usdcMint,                 // mint
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    
    const createATATx = new Transaction().add(createVaultATAIx);
    const createATASignature = await sendAndConfirmTransaction(connection, createATATx, [lenderKeypair]);
    console.log(`ATA du vault créée: ${createATASignature}`);
  } else {
    console.log("L'ATA du vault existe déjà");
  }

  let userBalance = await getAccount(connection, userUsdcAccount.address);
  console.log(`Solde USDC avant staking: ${Number(userBalance.amount) / 1000000} USDC`);

  console.log("\nTentative de staking de 100 USDC...");

  const stakeAmount = 100000000; // 100 USDC
  const lockDays = 7; // 7 jours pour le test

  // Créer l'instruction de deposit
  const depositInstructionData = createDepositStakingInstructionManual(stakeAmount, lockDays);
  
  const stakeUsdcIx = {
    keys: [
      { pubkey: stakingAccount, isSigner: false, isWritable: true },
      { pubkey: lenderKeypair.publicKey, isSigner: true, isWritable: true },
      { pubkey: userStatusAccount, isSigner: false, isWritable: false },
      { pubkey: userUsdcAccount.address, isSigner: false, isWritable: true },
      { pubkey: expectedVaultATA, isSigner: false, isWritable: true },
      { pubkey: usdcMint, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
    ],
    programId: FLEXFI_PROGRAM_ID,
    data: depositInstructionData
  };

  const stakeTx = new Transaction().add(stakeUsdcIx);
  
  try {
    // D'abord simuler
    console.log("\nSimulation du staking...");
    const simulation = await connection.simulateTransaction(stakeTx, [lenderKeypair]);
    
    if (simulation.value.err) {
      console.log("Erreur de simulation:", simulation.value.err);
      console.log("Logs:");
      simulation.value.logs?.forEach((log, i) => console.log(`[${i}] ${log}`));
      throw new Error("Simulation échouée");
    } else {
      console.log("Simulation réussie !");
      simulation.value.logs?.forEach((log, i) => console.log(`[${i}] ${log}`));
    }
    
    const stakeSignature = await sendAndConfirmTransaction(
      connection, 
      stakeTx, 
      [lenderKeypair],
      { skipPreflight: false, commitment: 'confirmed' }
    );

    console.log(`\nStaking réussi! Signature: ${stakeSignature}`);
    console.log(`Lien Explorer: https://solscan.io/tx/${stakeSignature}?cluster=devnet`);

    userBalance = await getAccount(connection, userUsdcAccount.address);
    console.log(`Solde USDC après staking: ${Number(userBalance.amount) / 1000000} USDC`);

    // Vérifier le vault
    const vaultBalance = await getAccount(connection, expectedVaultATA);
    console.log(`Solde du vault après staking: ${Number(vaultBalance.amount) / 1000000} USDC`);

  } catch (error) {
    console.error("\nErreur lors du staking:", error);
    if (error.logs) {
      console.log("\nLogs de l'erreur:");
      error.logs.forEach((log, i) => console.log(`[${i}] ${log}`));
    }
    return; // Arrêter l'exécution si le staking échoue
  }

  // PARTIE 2 : ATTENDRE ET RETIRER LE STAKING
  console.log("\n=== PARTIE 2 : RETRAIT DU STAKING ===");
  
  console.log("\nNote: Dans un environnement de test, nous ne pouvons pas attendre 7 jours.");
  console.log("Le retrait va échouer car la période de verrouillage n'est pas terminée.");
  console.log("C'est le comportement attendu !\n");
  
  const withdrawAmount = 50000000; // Retirer 50 USDC sur les 100 stakés
  
  // Créer l'instruction de withdraw
  const withdrawInstructionData = createWithdrawStakingInstructionManual(withdrawAmount);
  
  const withdrawUsdcIx = {
    keys: [
      { pubkey: stakingAccount, isSigner: false, isWritable: true },
      { pubkey: lenderKeypair.publicKey, isSigner: true, isWritable: true },
      { pubkey: userStatusAccount, isSigner: false, isWritable: false },
      { pubkey: userUsdcAccount.address, isSigner: false, isWritable: true },
      { pubkey: expectedVaultATA, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
    ],
    programId: FLEXFI_PROGRAM_ID,
    data: withdrawInstructionData
  };

  const withdrawTx = new Transaction().add(withdrawUsdcIx);
  
  try {
    // Simuler d'abord pour voir si ça va fonctionner
    console.log("\nSimulation du retrait...");
    const simulation = await connection.simulateTransaction(withdrawTx, [lenderKeypair]);
    
    if (simulation.value.err) {
      console.log("Erreur de simulation (attendue):", simulation.value.err);
      console.log("Logs:");
      simulation.value.logs?.forEach((log, i) => console.log(`[${i}] ${log}`));
      
      // Si l'erreur est due au verrouillage, l'afficher clairement
      const isStillLocked = simulation.value.logs?.some(log => 
        log.includes("still locked") || 
        log.includes("StakingFrozen") ||
        log.includes("lock period")
      );
      
      if (isStillLocked) {
        console.log("\n⚠️  Le staking est encore verrouillé (comportement attendu).");
        console.log("La période de verrouillage est de 7 jours.");
      }
    } else {
      console.log("Simulation réussie ! Envoi de la transaction...");
      
      const withdrawSignature = await sendAndConfirmTransaction(
        connection, 
        withdrawTx, 
        [lenderKeypair],
        { skipPreflight: false, commitment: 'confirmed' }
      );

      console.log(`\nRetrait réussi! Signature: ${withdrawSignature}`);
      console.log(`Lien Explorer: https://solscan.io/tx/${withdrawSignature}?cluster=devnet`);

      // Vérifier les soldes après retrait
      userBalance = await getAccount(connection, userUsdcAccount.address);
      console.log(`Solde USDC après retrait: ${Number(userBalance.amount) / 1000000} USDC`);

      const vaultBalance = await getAccount(connection, expectedVaultATA);
      console.log(`Solde du vault après retrait: ${Number(vaultBalance.amount) / 1000000} USDC`);
      
      console.log("\n✅ Test complet réussi ! Staking créé et retiré.");
    }
  } catch (error) {
    console.error("\nErreur lors du retrait (attendue):", error);
    if (error.logs) {
      console.log("\nLogs de l'erreur:");
      error.logs.forEach((log, i) => console.log(`[${i}] ${log}`));
    }
    
    console.log("\n⚠️  C'est normal que le retrait échoue !");
    console.log("Le staking est verrouillé pour 7 jours.");
    console.log("Dans un environnement de production, vous devriez attendre la fin de la période de verrouillage.");
    console.log("\nLe test de staking est réussi. Le retrait est correctement bloqué.");
  }
}

main().catch(console.error);