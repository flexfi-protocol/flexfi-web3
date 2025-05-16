// verify-wallet-whitelisted.js - FIXED
// Script to verify that the wallet is whitelisted before tests

const {
  Connection,
  PublicKey,
  Keypair
} = require('@solana/web3.js');

// Correct import for bs58 - FIXED VERSION
const bs58 = require('bs58');

// Configuration
const FLEXFI_PROGRAM_ID = new PublicKey('7Yd4fxojkMc9ZvCiewx7coorSnFm84VccBiNxX3hujUH');
const WHITELIST_SEED = 'whitelist';

// Wallet to verify
const WALLET_PUBLIC_KEY = '2AfToX6b4ncQPXKXGL16VjBAkQTJGazT5ACX7zS2WW4s';
const WALLET_PRIVATE_KEY_BASE58 = '4VCvzzfknrYMaQqTAxsNTgbeaCDPabYP5QvLwqPFQs3Ltow7WuEvodHinM7s9uVGjx6DLeoRYEBN5s4NnPPrwU75';

async function verifyWalletWhitelisted() {
  console.log('🔍 Verifying whitelist for test wallet');
  console.log('==================================================');

  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

  // 1. Verify public key
  console.log(`\n1️⃣ Verifying public key`);
  const publicKey = new PublicKey(WALLET_PUBLIC_KEY);
  console.log(`Public key: ${publicKey.toBase58()}`);

  // 2. Verify private key
  console.log(`\n2️⃣ Verifying private key`);
  try {
    // Option 1: From base58
    const keypairFromBase58 = Keypair.fromSecretKey(bs58.decode(WALLET_PRIVATE_KEY_BASE58));
    console.log(`✅ Valid base58 private key`);
    console.log(`   Derived public key: ${keypairFromBase58.publicKey.toBase58()}`);

    // Verify keys match
    if (keypairFromBase58.publicKey.toBase58() === WALLET_PUBLIC_KEY) {
      console.log(`✅ Public/private key match confirmed`);
    } else {
      console.log(`❌ ERROR: Keys do not match!`);
      console.log(`   Expected: ${WALLET_PUBLIC_KEY}`);
      console.log(`   Obtained:  ${keypairFromBase58.publicKey.toBase58()}`);
      return;
    }

    // Option 2: From array (for comparison)
    const privateKeyArray = Array.from(bs58.decode(WALLET_PRIVATE_KEY_BASE58));
    console.log(`\n   Array format: [${privateKeyArray.slice(0, 10).join(',')}...] (${privateKeyArray.length} bytes)`);

  } catch (error) {
    console.log(`❌ Error parsing private key: ${error.message}`);
    return;
  }

  // 3. Verify on-chain whitelist
  console.log(`\n3️⃣ Verifying on-chain whitelist`);

  const [userStatusAccount] = await PublicKey.findProgramAddress(
    [Buffer.from(WHITELIST_SEED), publicKey.toBuffer()],
    FLEXFI_PROGRAM_ID
  );

  console.log(`User Status PDA: ${userStatusAccount.toBase58()}`);

  const statusInfo = await connection.getAccountInfo(userStatusAccount);

  if (statusInfo && statusInfo.owner.equals(FLEXFI_PROGRAM_ID)) {
    console.log(`✅ WALLET WHITELISTED!`);
    console.log(`   Owner: ${statusInfo.owner.toBase58()}`);
    console.log(`   Size: ${statusInfo.data.length} bytes`);
    console.log(`   Rent-exempt: ${statusInfo.lamports} lamports`);

    // Parse data (if possible)
    if (statusInfo.data.length > 0) {
      console.log(`   Data (hex): ${statusInfo.data.toString('hex')}`);
    }
  } else {
    console.log(`❌ WALLET NOT WHITELISTED`);
    console.log(`   The whitelist account does not exist or does not belong to the FlexFi program`);
    console.log(`   Status: ${statusInfo ? 'Exists but wrong owner' : 'Does not exist'}`);
    if (statusInfo) {
      console.log(`   Current owner: ${statusInfo.owner.toBase58()}`);
    }
    return;
  }

  // 4. Summary
  console.log(`\n🎉 SUMMARY - Wallet ready for tests`);
  console.log(`=====================================`);
  console.log(`✅ Valid public key: ${WALLET_PUBLIC_KEY}`);
  console.log(`✅ Valid and matching private key`);
  console.log(`✅ Wallet whitelisted on-chain`);
  console.log(`✅ Ready for staking tests!`);
  console.log(`\n🚀 You can now run: node test-staking-with-backend.js`);
}

// Test key conversion - FIXED VERSION
async function testKeyConversion() {
  console.log('\n🔄 KEY CONVERSION TEST');
  console.log('===============================');

  try {
    // From base58 to array
    const privateKeyBytes = bs58.decode(WALLET_PRIVATE_KEY_BASE58);
    const privateKeyArray = Array.from(privateKeyBytes);

    console.log(`Base58: ${WALLET_PRIVATE_KEY_BASE58}`);
    console.log(`Array:  [${privateKeyArray.slice(0, 10).join(',')}...] (${privateKeyArray.length} bytes total)`);
    console.log(`Length: ${privateKeyArray.length} bytes`);

    // Round-trip verification
    const backToBase58 = bs58.encode(Uint8Array.from(privateKeyArray));
    console.log(`Round-trip: ${backToBase58 === WALLET_PRIVATE_KEY_BASE58 ? '✅' : '❌'}`);

    // Create keypair both ways
    const keypair1 = Keypair.fromSecretKey(bs58.decode(WALLET_PRIVATE_KEY_BASE58));
    const keypair2 = Keypair.fromSecretKey(Uint8Array.from(privateKeyArray));

    console.log(`Same public key: ${keypair1.publicKey.equals(keypair2.publicKey) ? '✅' : '❌'}`);

    // Display key for tests
    console.log(`\n🔑 FOR TESTS - Full array:`);
    console.log(`const WHITELISTED_PRIVATE_KEY = [${privateKeyArray.join(',')}];`);

  } catch (error) {
    console.log(`❌ Conversion error: ${error.message}`);
  }
}

// Function to add this wallet to the whitelist (if needed)
async function addWalletToWhitelist() {
  console.log('\n🔧 MANUAL WHITELIST ADDITION');
  console.log('==============================');
  console.log('If the wallet is not whitelisted, you can:');
  console.log(`1. Use the backend to add it`);
  console.log(`2. Use this command with solana CLI:`);
  console.log(`   solana-keygen new --outfile ${WALLET_PUBLIC_KEY}.json --no-bip39-passphrase`);
  console.log(`3. Or contact the FlexFi admin to add: ${WALLET_PUBLIC_KEY}`);
}

// Run verification
if (require.main === module) {
  verifyWalletWhitelisted()
    .then(() => testKeyConversion())
    .then(() => addWalletToWhitelist())
    .catch(console.error);
}

module.exports = {
  verifyWalletWhitelisted,
  WALLET_PUBLIC_KEY,
  WALLET_PRIVATE_KEY_BASE58
};
