import anchor from '@project-serum/anchor';
import { readFileSync } from 'fs';

const { SystemProgram } = anchor.web3;

const getInitializedStatus = async () => {
  // Establish connection
  const connection = new anchor.web3.Connection(
    anchor.web3.clusterApiUrl('devnet'), 
    'confirmed'
  );

  // Use your keypair
  const payer = anchor.web3.Keypair.fromSecretKey(
    new Uint8Array([
      133, 78, 3, 104, 72, 181, 47, 108, 156, 11, 166, 164, 245, 243, 34, 1,
      209, 89, 67, 93, 12, 119, 192, 159, 86, 182, 38, 118, 216, 85, 186, 98,
      54, 89, 236, 138, 0, 121, 27, 91, 196, 65, 195, 235, 114, 97, 116, 48, 21,
      70, 214, 184, 199, 241, 216, 168, 93, 191, 111, 218, 41, 53, 3, 73,
    ])
  );
  console.log('Payer public key:', payer.publicKey.toBase58());

  // Program ID
  const programId = new anchor.web3.PublicKey(
    'CyKce9sNf2SHyLZgS9URiu2o1tDs8UeASzpwtH3dpadt'
  );

  // Load IDL for the program
  const idl = JSON.parse(
    readFileSync('./target/idl/factory_contract.json', 'utf8')
  ); 

  // Set up the provider and program
  const provider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(payer), 
    {
      commitment: 'confirmed',
    }
  );
  const program = new anchor.Program(idl, programId, provider);

  // Public key of the factory account
  const factoryAccountPublickey = new anchor.web3.PublicKey('CiV8m96XtcxZpNyCSuptr4MweuHwH6iQBjW8vqybpamd');

  try {
    // Fetch the factory account's data
    const factoryAccountData = await program.account.factory.fetch(factoryAccountPublickey);

    // Retrieve the initialized status
    const initializedStatus = factoryAccountData.initialized;
    console.log('Factory Account Initialized:', initializedStatus);
  } catch (error) {
    console.error('Failed to fetch initialized status:', error);
  }
};

// Call the function to check if the contract is initialized
getInitializedStatus().catch(console.error);