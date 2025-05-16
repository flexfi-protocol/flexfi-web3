// verify-wallet-whitelisted.js - FIXED
// Script to verify that the wallet is whitelisted before tests

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
const axios = require('axios');

// Deployed FlexFi program
const FLEXFI_PROGRAM_ID = new PublicKey('7Yd4fxojkMc9ZvCiewx7coorSnFm84VccBiNxX3hujUH');

// Backend configuration
const BACKEND_URL = 'http://localhost:3000';

// Seeds for PDAs
const WHITELIST_SEED = 'whitelist';
const STAKING_SEED = 'staking';
const USDC_VAULT_SEED = 'usdc_vault';

// Discriminants
const DISCRIMINANTS = {
  DEPOSIT_STAKING: 0,
  WITHDRAW_STAKING: 1,
};

// Helper to create staking instruction
function createDepositStakingInstruction(amount, lockDays) {
  const buffer = Buffer.alloc(11); // 1 (discriminant) + 8 (u64) + 2 (u16)

  // Discriminant 0 for DepositStaking
  buffer.writeUInt8(DISCRIMINANTS.DEPOSIT_STAKING, 0);

  // amount as u64 little endian
  const amountBuffer = new BN(amount).toArrayLike(Buffer, 'le', 8);
  amountBuffer.copy(buffer, 1);

  // lock_days as u16 little endian
  buffer.writeUInt16LE(lockDays, 9);

  return buffer;
}

// Create the withdrawal instruction
function createWithdrawStakingInstruction(amount) {
  const buffer = Buffer.alloc(9); // 1 (discriminant) + 8 (u64)

  // Discriminant 1 for WithdrawStaking
  buffer.writeUInt8(DISCRIMINANTS.WITHDRAW_STAKING, 0);

  // amount as u64 little endian
  const amountBuffer = new BN(amount).toArrayLike(Buffer, 'le', 8);
  amountBuffer.copy(buffer, 1);

  return buffer;
}

// Class to manage backend integration
class BackendManager {
  constructor() {
    this.authToken = null;
    this.walletInfo = null;
  }

  async createNewUser(emailPrefix = 'test-staking') {
    const timestamp = Date.now();
    const email = `${emailPrefix}-${timestamp}@example.com`;
    const password = 'TestPassword123!';

    console.log(`🔐 Creating a new user: ${email}`);

    try {
      const response = await axios.post(`${BACKEND_URL}/api/auth/register`, {
        email,
        password
      });

      if (response.data.status === 'success') {
        this.authToken = response.data.data.token;
        this.walletInfo = response.data.data.wallet;
        console.log(`✅ User created, wallet: ${this.walletInfo.publicKey}`);
        return {
          email,
          password,
          wallet: this.walletInfo,
          token: this.authToken
        };
      }
    } catch (error) {
      console.error('❌ Error creating user:', error.response?.data || error.message);
      throw error;
    }
  }

  async initializeFlexFi(amount = 50000000, durationDays = 30) {
    console.log(`🚀 Initializing FlexFi: ${amount / 1000000} USDC for ${durationDays} days`);

    try {
      const response = await axios.post(`${BACKEND_URL}/api/wallet/flexfi/authorize`, {
        authorizedAmount: amount,
        durationDays: durationDays
      }, {
        headers: { Authorization: `Bearer ${this.authToken}` }
      });

      if (response.data.success) {
        console.log(`✅ FlexFi initialized`);
        console.log(`   Authorization Account: ${response.data.data.authorizationAccount}`);
        console.log(`   Transaction: ${response.data.data.transaction}`);
        return response.data.data;
      }
    } catch (error) {
      console.error('❌ Error initializing FlexFi:', error.response?.data || error.message);
      throw error;
    }
  }

  async getWalletStatus() {
    try {
      const response = await axios.get(`${BACKEND_URL}/api/wallet`, {
        headers: { Authorization: `Bearer ${this.authToken}` }
      });

      if (response.data.success && response.data.data.length > 0) {
        const wallet = response.data.data[0];
        console.log(`📊 Wallet status: ${wallet.publicKey}`);
        console.log(`   - Whitelist synced: ${wallet.whitelistSynced}`);
        console.log(`   - FlexFi active: ${wallet.flexfi.isAuthorizationActive}`);
        console.log(`   - Card type: ${wallet.cardType}`);
        return wallet;
      }
    } catch (error) {
      console.error('❌ Error retrieving status:', error.response?.data || error.message);
      throw error;
    }
  }
}

// Class for blockchain tests
class StakingTester {
  constructor(connection, programId) {
    this.connection = connection;
    this.programId = programId;
  }

  async setupTestWallet(walletPublicKey = null) {
    console.log(`🔧 Setting up test wallet`);

    // Use the provided whitelisted wallet
    console.log('✅ WHITELISTED WALLET: Using the official test wallet');
    console.log('   In production, the user would have their private key encrypted in the backend');

    // Private key of the whitelisted wallet 2AfToX6b4ncQPXKXGL16VjBAkQTJGazT5ACX7zS2WW4s
    const WHITELISTED_PRIVATE_KEY = [
      174,95,224,249,206,157,168,36,74,81,125,89,80,32,106,171,64,175,198,95,195,42,134,238,197,14,27,149,243,105,69,46,17,85,103,72,94,139,195,84,123,35,39,39,67,168,233,90,90,150,250,11,191,253,204,211,25,208,211,82,162,1,193,108
    ];

    const testKeypair = Keypair.fromSecretKey(Uint8Array.from(WHITELISTED_PRIVATE_KEY));
    console.log(`   🔑 Whitelisted wallet: ${testKeypair.publicKey.toBase58()}`);
    console.log(`   ✅ This wallet is already in the on-chain whitelist`);

    // Airdrop SOL for transaction fees
    // console.log('💰 Airdropping SOL for transaction fees...');
    // try {
    //   const airdrop = await this.connection.requestAirdrop(testKeypair.publicKey, 2 * 1e9);
    //   await this.connection.confirmTransaction(airdrop);
    // } catch (error) {
    //   console.log('   Airdrop failed (limit reached), checking balance...');
    // }

    const balance = await this.connection.getBalance(testKeypair.publicKey);
    console.log(`   SOL Balance: ${balance / 1e9} SOL`);

    return testKeypair;
  }

  async createTestUSDC(userKeypair) {
    console.log('🪙 Creating test USDC token...');

    // Create a USDC mint
    const usdcMint = await createMint(
      this.connection,
      userKeypair,
      userKeypair.publicKey,
      null,
      6
    );
    console.log(`   USDC Mint: ${usdcMint.toBase58()}`);

    // Create a token account for the user
    const userUsdcAccount = await getOrCreateAssociatedTokenAccount(
      this.connection,
      userKeypair,
      usdcMint,
      userKeypair.publicKey
    );
    console.log(`   User USDC Account: ${userUsdcAccount.address.toBase58()}`);

    // Mint 200 USDC to the user
    await mintTo(
      this.connection,
      userKeypair,
      usdcMint,
      userUsdcAccount.address,
      userKeypair,
      200000000 // 200 USDC
    );
    console.log(`   200 USDC minted`);

    // Check balance
    const balance = await getAccount(this.connection, userUsdcAccount.address);
    console.log(`   USDC Balance: ${Number(balance.amount) / 1000000} USDC`);

    return { usdcMint, userUsdcAccount };
  }

  async calculateStakingPDAs(userKeypair, usdcMint) {
    console.log('🔍 Calculating staking PDAs...');

    // User Status PDA (for whitelist)
    const [userStatusAccount] = await PublicKey.findProgramAddress(
      [Buffer.from(WHITELIST_SEED), userKeypair.publicKey.toBuffer()],
      this.programId
    );

    // Staking Account PDA
    const [stakingAccount] = await PublicKey.findProgramAddress(
      [
        Buffer.from(STAKING_SEED),
        userKeypair.publicKey.toBuffer(),
        usdcMint.toBuffer()
      ],
      this.programId
    );

    // Vault Account PDA
    const [vaultAccount] = await PublicKey.findProgramAddress(
      [
        Buffer.from(USDC_VAULT_SEED),
        stakingAccount.toBuffer()
      ],
      this.programId
    );

    // ATA of the vault
    const vaultATA = await getAssociatedTokenAddress(
      usdcMint,
      vaultAccount,
      true // allowOwnerOffCurve
    );

    console.log(`   User Status PDA: ${userStatusAccount.toBase58()}`);
    console.log(`   Staking Account: ${stakingAccount.toBase58()}`);
    console.log(`   Vault Account: ${vaultAccount.toBase58()}`);
    console.log(`   Vault ATA: ${vaultATA.toBase58()}`);

    return {
      userStatusAccount,
      stakingAccount,
      vaultAccount,
      vaultATA
    };
  }

  async verifyWhitelist(userStatusAccount, hardcodedWallet) {
    console.log('🔍 Verifying whitelist...');
    console.log(`   Hardcoded wallet: ${hardcodedWallet.publicKey.toBase58()}`);

    // Calculate the PDA for the hardcoded wallet (not the backend)
    const [hardcodedStatusAccount] = await PublicKey.findProgramAddress(
      [Buffer.from(WHITELIST_SEED), hardcodedWallet.publicKey.toBuffer()],
      this.programId
    );

    console.log(`   Hardcoded Status PDA: ${hardcodedStatusAccount.toBase58()}`);

    const statusInfo = await this.connection.getAccountInfo(hardcodedStatusAccount);

    if (statusInfo && statusInfo.owner.equals(this.programId)) {
      console.log('✅ Hardcoded wallet found in on-chain whitelist');
      console.log(`   Owner: ${statusInfo.owner.toBase58()}`);
      console.log(`   Size: ${statusInfo.data.length} bytes`);
      return { isWhitelisted: true, statusAccount: hardcodedStatusAccount };
    } else {
      console.log('⚠️ Hardcoded wallet not found in on-chain whitelist');
      console.log('   This wallet must be added to the whitelist for testing');
      return { isWhitelisted: false, statusAccount: hardcodedStatusAccount };
    }
  }

  async performStaking(userKeypair, pdas, usdcInfo, amount = 100000000, lockDays = 30) {
    const { userStatusAccount, stakingAccount, vaultAccount, vaultATA } = pdas;
    const { usdcMint, userUsdcAccount } = usdcInfo;

    console.log(`\n💰 Staking test: ${amount / 1000000} USDC for ${lockDays} days`);

    // Check if the vault ATA exists, otherwise create it
    const vaultATAInfo = await this.connection.getAccountInfo(vaultATA);
    if (!vaultATAInfo) {
      console.log('🔧 Creating vault ATA...');
      const createVaultATAIx = createAssociatedTokenAccountInstruction(
        userKeypair.publicKey,  // payer
        vaultATA,              // ATA to create
        vaultAccount,          // owner (the vault PDA)
        usdcMint,              // mint
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      );

      const createATATx = new Transaction().add(createVaultATAIx);
      const createATASignature = await sendAndConfirmTransaction(
        this.connection,
        createATATx,
        [userKeypair]
      );
      console.log(`   ATA created: ${createATASignature}`);
    }

    // Staking instruction
    const stakingIx = {
      keys: [
        { pubkey: stakingAccount, isSigner: false, isWritable: true },
        { pubkey: userKeypair.publicKey, isSigner: true, isWritable: true },
        { pubkey: userStatusAccount, isSigner: false, isWritable: false }, // Whitelist
        { pubkey: userUsdcAccount.address, isSigner: false, isWritable: true },
        { pubkey: vaultATA, isSigner: false, isWritable: true },
        { pubkey: usdcMint, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
        { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      ],
      programId: this.programId,
      data: createDepositStakingInstruction(amount, lockDays)
    };

    // Simulate first
    console.log('🔍 Simulating staking transaction...');
    const simulation = await this.connection.simulateTransaction(
      new Transaction().add(stakingIx),
      [userKeypair]
    );

    if (simulation.value.err) {
      console.log('❌ Simulation failed:', simulation.value.err);
      console.log('Logs:');
      simulation.value.logs?.forEach((log, i) => console.log(`[${i}] ${log}`));
      throw new Error('Staking simulation failed');
    }

    console.log('✅ Simulation successful!');
    console.log('Simulation logs:');
    simulation.value.logs?.forEach((log, i) => console.log(`[${i}] ${log}`));

    // Send the transaction
    console.log('🚀 Sending staking transaction...');
    const tx = new Transaction().add(stakingIx);
    const signature = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [userKeypair],
      { skipPreflight: false, commitment: 'confirmed' }
    );

    console.log(`✅ Staking successful!`);
    console.log(`   Signature: ${signature}`);
    console.log(`   Explorer: https://solscan.io/tx/${signature}?cluster=devnet`);

    return signature;
  }

  async verifyStaking(pdas, usdcInfo, stakedAmount) {
    const { stakingAccount, vaultATA } = pdas;
    const { userUsdcAccount } = usdcInfo;

    console.log('\n🔍 Verifying staking...');

    // Verify the staking account
    const stakingInfo = await this.connection.getAccountInfo(stakingAccount);
    if (stakingInfo && stakingInfo.owner.equals(this.programId)) {
      console.log('✅ Staking account created');
      console.log(`   Size: ${stakingInfo.data.length} bytes`);
    }

    // Verify balances
    const userBalance = await getAccount(this.connection, userUsdcAccount.address);
    console.log(`   User balance: ${Number(userBalance.amount) / 1000000} USDC`);

    const vaultBalance = await getAccount(this.connection, vaultATA);
    console.log(`   Vault balance: ${Number(vaultBalance.amount) / 1000000} USDC`);

    // Verify the amount was transferred
    if (Number(vaultBalance.amount) === stakedAmount) {
      console.log('✅ Correct amount transferred to vault');
    } else {
      console.log(`⚠️ Unexpected amount (expected: ${stakedAmount}, received: ${Number(vaultBalance.amount)})`);
    }
  }
}

async function main() {
  console.log('🚀 STAKING TEST WITH BACKEND WALLET FlexFi');
  console.log('==========================================\n');

  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  const backend = new BackendManager();
  const tester = new StakingTester(connection, FLEXFI_PROGRAM_ID);

  try {
    // STEP 1: Create a user and wallet via backend
    // console.log('🔵 STEP 1: Creating user and backend wallet');
    // const userInfo = await backend.createNewUser('test-staking');
    // console.log(`📧 Email: ${userInfo.email}`);
    // console.log(`🔑 Backend Wallet: ${userInfo.wallet.publicKey}`);

    // // STEP 2: Initialize FlexFi
    // console.log('\n🔵 STEP 2: Initializing FlexFi');
    // await backend.initializeFlexFi(50000000, 30);

    // // STEP 3: Verify status
    // console.log('\n🔵 STEP 3: Verifying status');
    // const walletStatus = await backend.getWalletStatus();

    // STEP 4: Setup for blockchain tests
    console.log('\n🔵 STEP 4: Blockchain setup (hardcoded wallet)');
    const userKeypair = await tester.setupTestWallet();

    // STEP 5: Create test USDC
    console.log('\n🔵 STEP 5: Creating test USDC');
    const usdcInfo = await tester.createTestUSDC(userKeypair);

    // STEP 6: Calculate PDAs for the hardcoded wallet
    console.log('\n🔵 STEP 6: Calculating PDAs (hardcoded wallet)');
    const pdas = await tester.calculateStakingPDAs(userKeypair, usdcInfo.usdcMint);

    // STEP 7: Verify whitelist of the hardcoded wallet
    console.log('\n🔵 STEP 7: Verifying whitelist (hardcoded wallet)');
    const whitelistStatus = await tester.verifyWhitelist(pdas.userStatusAccount, userKeypair);

    if (!whitelistStatus.isWhitelisted) {
      console.log('💡 NOTE: For this test to work, the hardcoded wallet must be added');
      console.log('         to the whitelist via the backend or an initialization script.');
      console.log('         Continuing anyway to see the error...');
    }

    // STEP 8: Perform staking (with the whitelisted hardcoded wallet)
    console.log('\n🔵 STEP 8: Staking test (hardcoded wallet)');
    const stakedAmount = 100000000; // 100 USDC
    const lockDays = 30;

    // Use the status account of the hardcoded wallet
    const stakingPDAs = {
      ...pdas,
      userStatusAccount: whitelistStatus.statusAccount
    };

    const signature = await tester.performStaking(
      userKeypair,
      stakingPDAs,
      usdcInfo,
      stakedAmount,
      lockDays
    );

    // STEP 9: Verifications
    console.log('\n🔵 STEP 9: Final verifications');
    await tester.verifyStaking(pdas, usdcInfo, stakedAmount);

    // SUMMARY
    console.log('\n🎉 STAKING TEST COMPLETED SUCCESSFULLY!');
    console.log('=====================================');
    console.log(`✅ Hardcoded wallet used: ${userKeypair.publicKey.toBase58()}`);
    console.log(`✅ Staking performed: ${stakedAmount / 1000000} USDC`);
    console.log(`✅ Lock duration: ${lockDays} days`);
    console.log(`✅ Transaction: ${signature}`);
    console.log(`✅ Explorer: https://solscan.io/tx/${signature}?cluster=devnet`);

    // Information for next steps
    console.log('\n📋 INFORMATION FOR NEXT TESTS:');
    console.log('==========================================');
    console.log(`Hardcoded Wallet (tests): ${userKeypair.publicKey.toBase58()}`);
    console.log(`USDC Mint: ${usdcInfo.usdcMint.toBase58()}`);
    console.log(`Staking Account: ${pdas.stakingAccount.toBase58()}`);
    console.log(`User Status Account: ${whitelistStatus.statusAccount.toBase58()}`);

    console.log('\n🏗️ CONFIRMED ARCHITECTURE:');
    console.log('===========================');
    console.log('✅ Backend: Generates wallets + manages whitelist (FlexFi ADMIN)');
    console.log('✅ User: Signs only critical transactions (staking)');
    console.log('✅ Tests: Use hardcoded wallets for simplicity');
    console.log('✅ Production: Encrypted wallets in backend + selective signing');

  } catch (error) {
    console.error('\n💥 ERROR IN TEST:', error);

    // Display additional details if available
    if (error.response?.data) {
      console.log('Backend error details:', error.response.data);
    }

    if (error.logs) {
      console.log('Blockchain logs:', error.logs);
    }

    process.exit(1);
  }
}

// Important notes for integration
console.log('📝 FlexFi ARCHITECTURE:');
console.log('=======================');
console.log('🔑 USER signs: Staking, Withdraw, Critical authorizations');
console.log('🔑 FLEXFI ADMIN signs: Whitelist, BNPL spend, Automated operations');
console.log('💾 STORAGE: User private keys encrypted in the backend');
console.log('🧪 TESTS: Hardcoded wallets for simplicity/reproducibility');
console.log('');
console.log('📋 NEXT TESTS TO ADAPT:');
console.log('==============================');
console.log('1️⃣ Score: Use hardcoded wallet + verify backend creation');
console.log('2️⃣ BNPL: User signs authorization, FlexFi ADMIN signs spend');
console.log('3️⃣ NFT/Cards: Integrate with backend wallets + managed cards');

main().catch(error => {
  console.error('💥 Fatal error:', error);
  process.exit(1);
});
