import anchor from '@project-serum/anchor';
import { readFileSync } from 'fs';
import { Connection } from '@solana/web3.js';
import dotenv from 'dotenv';

dotenv.config();

const initializer = async () => {
  const { SystemProgram } = anchor.web3;
  const connection = new Connection(process.env.RPC_URL_SOLANA_DEVNET);

  const secretKey = JSON.parse(process.env.SOLANA_SECRET_KEY);
  const payer = Keypair.fromSecretKey(new Uint8Array(secretKey));
  
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