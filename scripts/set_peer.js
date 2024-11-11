const anchor = require('@project-serum/anchor');
const { PublicKey, SystemProgram } = require('@solana/web3.js');
const { readFileSync } = require('fs');

// Your provided keypair Uint8Array
const rawKeypair = new Uint8Array([
  133, 78, 3, 104, 72, 181, 47, 108, 156, 11, 166, 164, 245, 243, 34, 1, 209, 89,
  67, 93, 12, 119, 192, 159, 86, 182, 38, 118, 216, 85, 186, 98, 54, 89,
  236, 138, 0, 121, 27, 91, 196, 65, 195, 235, 114, 97, 116, 48, 21, 70,
  214, 184, 199, 241, 216, 168, 93, 191, 111, 218, 41, 53, 3, 73
]);

async function main() {
  // Convert the raw Uint8Array to a Keypair
  const wallet = anchor.web3.Keypair.fromSecretKey(rawKeypair);
  const connection = new anchor.web3.Connection(anchor.web3.clusterApiUrl('devnet'), 'confirmed');
  console.log("Wallet public key:", wallet.publicKey.toBase58());
  // Initialize provider with the custom wallet
  const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(wallet), { commitment: 'confirmed' });
  anchor.setProvider(provider);

  // Load the program
  const idl = JSON.parse(readFileSync('./target/idl/factory_contract.json', 'utf8'));
  const program = new anchor.Program(idl, new PublicKey('CyKce9sNf2SHyLZgS9URiu2o1tDs8UeASzpwtH3dpadt'), provider);

  // Convert remoteHex to a byte array
  const remoteHex = '0000000000000000000000002286266daef92c9e123f03ee92fe0293da2ea8f8';
  const dstEid = 40267;

  const remoteBytes = Buffer.from(remoteHex, 'hex');

  // Compute the PDA for the remote account based on some seeds
  const [remoteAddress, bump] = await PublicKey.findProgramAddress(
    [Buffer.from('some_seed'), wallet.publicKey.toBuffer()], // Adjust seed appropriately
    program.programId
  );

  try {
    // Prepare the transaction
    const tx = await program.methods.setRemote({
      dstEid,
      remote: remoteBytes
    })
      .accounts({
        admin: wallet.publicKey,
        remote: remoteAddress,
        systemProgram: SystemProgram.programId,
      })
      .signers([wallet])
      .rpc();

    console.log("Transaction signature:", tx);
  } catch(err) {
    console.error("Error in sending transaction:", err);
  }
}

main().catch(err => {
  console.error("Caught error:", err);
});