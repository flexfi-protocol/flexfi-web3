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

// Créer la fonction d'encodage correcte
function createDepositStakingInstruction(amount, lockDays) {
  // Créer l'objet instruction
  const instruction = new DepositStaking({
    amount: new BN(amount),
    lock_days: lockDays
  });

  // Sérialiser juste les données de l'instruction
  const instructionData = borsh.serialize(depositStakingSchema, instruction);
  
  // Pour un enum Borsh, on a besoin du discriminant + les données
  const buffer = Buffer.alloc(1 + instructionData.length);
  
  // Le discriminant est 0 pour DepositStaking (première variante dans l'enum)
  buffer.writeUInt8(0, 0);
  
  // Copier les données sérialisées
  instructionData.copy(buffer, 1);
  
  return buffer;
}

// Alternative manuelle simple
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

async function main() {
  console.log("Test de staking avec le wallet hardcodé...");

  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

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

  console.log("Création du token USDC...");
  const usdcMint = await createMint(
    connection, 
    lenderKeypair, 
    lenderKeypair.publicKey, 
    null, 
    6
  );
  console.log(`USDC créé: ${usdcMint.toBase58()}`);

  const userUsdcAccount = await getOrCreateAssociatedTokenAccount(
    connection,
    lenderKeypair,
    usdcMint,
    lenderKeypair.publicKey
  );
  console.log(`Compte USDC: ${userUsdcAccount.address.toBase58()}`);

  await mintTo(
    connection,
    lenderKeypair,
    usdcMint,
    userUsdcAccount.address,
    lenderKeypair,
    300000000
  );
  console.log("300 USDC mintés");

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

  // NOUVEAU : Créer l'ATA du vault AVANT l'instruction de staking
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

  console.log("\nTentative de staking de 10 USDC...");

  const amount = 10000000; // 100 USDC
  const lockDays = 30;

  // Créer l'instruction avec le bon discriminant (0 pour DepositStaking)
  const instructionData = createDepositStakingInstructionManual(amount, lockDays);
  console.log("Instruction data (hex):", instructionData.toString('hex'));
  console.log("Instruction data length:", instructionData.length);
  
  // Afficher les composants de l'instruction
  console.log("Discriminant:", instructionData[0]);
  console.log("Amount bytes:", instructionData.slice(1, 9).toString('hex'));
  console.log("Lock days bytes:", instructionData.slice(9, 11).toString('hex'));

  const stakingAccountInfo = await connection.getAccountInfo(stakingAccount);
  if (!stakingAccountInfo) {
    console.log("✅ Le staking PDA n'existe pas encore, il sera créé automatiquement par le programme.");
  } else {
    console.log("⚠️ Le staking PDA existe déjà. Vérifie qu'il est bien initialisé.");
  }

  const stakeUsdcIx = {
    keys: [
      { pubkey: stakingAccount, isSigner: false, isWritable: true },
      { pubkey: lenderKeypair.publicKey, isSigner: true, isWritable: true },
      { pubkey: userStatusAccount, isSigner: false, isWritable: false },
      { pubkey: userUsdcAccount.address, isSigner: false, isWritable: true },
      { pubkey: expectedVaultATA, isSigner: false, isWritable: true }, // Utiliser l'ATA
      { pubkey: usdcMint, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
    ],
    programId: FLEXFI_PROGRAM_ID,
    data: instructionData
  };

  console.log("\nComptes dans l'instruction:");
  stakeUsdcIx.keys.forEach((key, index) => {
    console.log(`${index}: ${key.pubkey.toBase58()} (writable: ${key.isWritable}, signer: ${key.isSigner})`);
  });

  const tx = new Transaction().add(stakeUsdcIx);
  tx.feePayer = lenderKeypair.publicKey;

  try {
    // D'abord simuler pour voir les logs
    console.log("\nSimulation de la transaction...");
    const simulation = await connection.simulateTransaction(tx, [lenderKeypair]);
    
    if (simulation.value.err) {
      console.log("Erreur de simulation:", simulation.value.err);
      console.log("Logs:");
      simulation.value.logs?.forEach((log, i) => console.log(`[${i}] ${log}`));
    } else {
      console.log("Simulation réussie !");
      simulation.value.logs?.forEach((log, i) => console.log(`[${i}] ${log}`));
      
      // Si la simulation est réussie, envoyer la transaction
      const signature = await sendAndConfirmTransaction(
        connection, 
        tx, 
        [lenderKeypair],
        {
          skipPreflight: false,
          commitment: 'confirmed'
        }
      );

      console.log(`\nStaking réussi! Signature: ${signature}`);
      console.log(`Lien Explorer: https://solscan.io/tx/${signature}?cluster=devnet`);

      userBalance = await getAccount(connection, userUsdcAccount.address);
      console.log(`\nSolde USDC après staking: ${Number(userBalance.amount) / 1000000} USDC`);

      // Vérifier le vault
      try {
        const vaultBalance = await getAccount(connection, expectedVaultATA);
        console.log(`Solde du vault: ${Number(vaultBalance.amount) / 1000000} USDC`);
      } catch (error) {
        console.log("Vault token account n'existe pas encore ou erreur lors de la lecture");
      }

      console.log("\n✅ Test réussi ! Le wallet hardcodé peut staker des USDC.");
    }
  } catch (error) {
    console.error("\n❌ Erreur lors du staking:", error);
    
    if (error.logs) {
      console.log("\nLogs de l'erreur:");
      error.logs.forEach((log, i) => console.log(`[${i}] ${log}`));
    }
  }
}

main().catch(console.error);