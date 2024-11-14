import anchor from '@project-serum/anchor';
import { readFileSync } from 'fs';
import { Connection } from '@solana/web3.js';


const initializer = async () => {
  const { SystemProgram } = anchor.web3;
  const connection = new Connection('https://api.devnet.solana.com');


  const payer = anchor.web3.Keypair.fromSecretKey(
    new Uint8Array([
      133, 78, 3, 104, 72, 181, 47, 108, 156, 11, 166, 164, 245, 243, 34, 1, 209, 89,
      67, 93, 12, 119, 192, 159, 86, 182, 38, 118, 216, 85, 186, 98, 54, 89,
      236, 138, 0, 121, 27, 91, 196, 65, 195, 235, 114, 97, 116, 48, 21, 70,
      214, 184, 199, 241, 216, 168, 93, 191, 111, 218, 41, 53, 3, 73
    ])
  );
  

  anchor.setProvider(new anchor.AnchorProvider(connection, new anchor.Wallet(payer), { commitment: 'confirmed' }));

  const programId = new anchor.web3.PublicKey('EjTQazH7zvwvBFDkbJRnpvQfjuQBqjHTdbYE25iaxZoJ');
  const idl = JSON.parse(readFileSync('../target/idl/factory_contract.json', 'utf8'));
  const program = new anchor.Program(idl, programId);
  // Generate a new keypair for the factory account
  const factoryAccount = anchor.web3.Keypair.generate();

  const tx = await program.rpc.initialize({
    accounts: {
      factory: factoryAccount.publicKey,
      payer: payer.publicKey,
      systemProgram: SystemProgram.programId,
    },
    signers: [payer, factoryAccount],
  });

  console.log("Factory account initialized with public key:", factoryAccount.publicKey.toBase58());
};

initializer();