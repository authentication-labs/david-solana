import anchor from '@project-serum/anchor';
import { readFileSync } from 'fs';

const initializer = async () => {
  const { SystemProgram } = anchor.web3;

  const connection = new anchor.web3.Connection(
    anchor.web3.clusterApiUrl('devnet'),
    'confirmed'
  );

  const payer = anchor.web3.Keypair.fromSecretKey(
    new Uint8Array([14,123,28,102,64,8,100,70,201,98,157,178,125,37,40,79,76,239,71,144,74,126,89,131,146,175,142,18,226,127,62,3,59,137,181,152,126,59,24,180,220,163,197,209,178,127,116,184,23,55,194,161,113,147,73,205,15,26,112,110,255,255,21,87])
  );
  

  anchor.setProvider(new anchor.AnchorProvider(connection, new anchor.Wallet(payer), { commitment: 'confirmed' }));

  const programId = new anchor.web3.PublicKey('CyKce9sNf2SHyLZgS9URiu2o1tDs8UeASzpwtH3dpadt');
  const idl = JSON.parse(readFileSync('./target/idl/factory_contract.json', 'utf8'));
  const program = new anchor.Program(idl, programId);

  // Generate a new keypair for the factory account
  const factoryAccount = anchor.web3.Keypair.generate();
  console.log(await factoryAccount.secretKey)

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