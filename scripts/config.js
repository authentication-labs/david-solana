import { arrayify, hexZeroPad } from '@ethersproject/bytes';
import { Connection, Keypair, PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { buildVersionedTransaction, oappIDPDA, EndpointProgram, EventPDADeriver, SimpleMessageLibProgram, UlnProgram } from '@layerzerolabs/lz-solana-sdk-v2';
import * as anchor from '@project-serum/anchor';
import { readFileSync } from 'fs';
import { EndpointId } from '@layerzerolabs/lz-definitions';

// Establish connection to the Solana devnet
const connection = new Connection('https://api.devnet.solana.com');

const remotePeers = {
    [EndpointId.OPTIMISM_V2_TESTNET]: '0x632824675C8871A1D4e2e692c121bE4eDC4051e2', // EVM counter addr
};

const endpointProgram = new EndpointProgram.Endpoint(new PublicKey('76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6')); // endpoint program id

const wallet = Keypair.fromSecretKey(new Uint8Array([
    133, 78, 3, 104, 72, 181, 47, 108, 156, 11, 166, 164, 245, 243, 34, 1, 209, 89,
    67, 93, 12, 119, 192, 159, 86, 182, 38, 118, 216, 85, 186, 98, 54, 89,
    236, 138, 0, 121, 27, 91, 196, 65, 195, 235, 114, 97, 116, 48, 21, 70,
    214, 184, 199, 241, 216, 168, 93, 191, 111, 218, 41, 53, 3, 73
]));

const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(wallet), { commitment: 'confirmed' });
anchor.setProvider(provider);

let idl;
try {
    idl = JSON.parse(readFileSync('../target/idl/factory_contract.json', 'utf8'));
} catch (error) {
    console.error("Failed to load IDL:", error);
    process.exit(1);
}

const program = new anchor.Program(idl, new PublicKey('EjTQazH7zvwvBFDkbJRnpvQfjuQBqjHTdbYE25iaxZoJ'), provider);

const COUNT_SEED = 'Count';
const counterId = 0;

(async () => {
    try {
        const [countPDA, countBump] = await oappIDPDA(program.programId, COUNT_SEED, counterId);
        await initCounter(connection, wallet, wallet, countPDA, endpointProgram);

        // Uncomment and modify as needed
        for (const [remoteStr, remotePeer] of Object.entries(remotePeers)) {
            const remote = parseInt(remoteStr);
            await setPeers(connection, wallet, remote, arrayify(hexZeroPad(remotePeer, 32)));
        }
    } catch (error) {
        console.error("Error in main function:", error);
    }
})();

async function initCounter(connection, payer, admin, countPDA, endpoint) {
    try {
        console.log('Initializing count with PDA:', countPDA.toBase58());

        const [oAppRegistry] = await endpoint.deriver.oappRegistry(countPDA);
        const [eventAuthority] = new EventPDADeriver(endpoint.program).eventAuthority();

        let current = false;
        try {
            await program.account.factory.fetch(countPDA);
            current = true;
        } catch (e) {
            console.log('Counter not initialized yet.');
        }

        if (current) {
            console.log('Counter already initialized.');
            return;
        }

        const [lzReceiveTypesAccounts] = await PublicKey.findProgramAddress(
            [Buffer.from('LzReceiveTypes'), countPDA.toBuffer()], program.programId
        );

        const [lzComposeTypesAccounts] = await PublicKey.findProgramAddress(
            [Buffer.from('LzComposeTypes'), countPDA.toBuffer()], program.programId
        );

        console.log('Checking individual account related keys:');
        console.log('Endpoint Program:', endpointProgram);
        console.log('Wallet Public Key:', wallet.publicKey.toBase58());
        console.log('Count PDA:', countPDA.toBase58());
        console.log('oAppRegistry:', oAppRegistry.toBase58());
        console.log('Event Authority:', eventAuthority.toBase58());

        const txInstruction = await program.methods.initCount({
            admin: admin.publicKey,
            endpoint: endpointProgram.programId
        })
            .accounts({
                payer: payer.publicKey,
                factory: countPDA,
                lzReceiveTypesAccounts: lzReceiveTypesAccounts,
                lzComposeTypesAccounts: lzComposeTypesAccounts,
                systemProgram: SystemProgram.programId
            })
            .remainingAccounts([
                { pubkey: endpointProgram.program, isWritable: false, isSigner: false }, // Use the actual PublicKey member
                { pubkey: wallet.publicKey, isWritable: true, isSigner: true },
                { pubkey: countPDA, isWritable: false, isSigner: false }, // pda of oapp
                { pubkey: oAppRegistry, isWritable: true, isSigner: false },
                { pubkey: anchor.web3.SystemProgram.programId, isWritable: false, isSigner: false },
                { pubkey: eventAuthority, isWritable: false, isSigner: false },
                { pubkey: endpointProgram.program, isWritable: false, isSigner: false },
                { pubkey: countPDA, isWritable: true, isSigner: false },
                { pubkey: anchor.web3.SystemProgram.programId, isWritable: false, isSigner: false },
                { pubkey: endpointProgram.program, isWritable: false, isSigner: false },
            ])
            .instruction();

        await sendAndConfirm(connection, [admin], [txInstruction]);
    } catch (error) {
        console.error("Error in initCounter:", error);
    }
}

async function setPeers(
    connection,
    admin,
    dstEid,
    remotePeer
) {
    try {
        const [remoteAddress] = await PublicKey.findProgramAddress(
            [Buffer.from('remote'), admin.publicKey.toBuffer()],
            program.programId
        );

        console.log('Remote Address:', remoteAddress.toBase58());
        console.log('Admin Public Key:', admin.publicKey.toBase58());

        const txInstruction = await program.methods.setRemote({
            dstEid,
            remote: remotePeer
        })
            .accounts({
                admin: admin.publicKey,
                remote: remoteAddress,
                systemProgram: SystemProgram.programId,
            })
            .instruction();

        await sendAndConfirm(connection, [admin], [txInstruction]);
    } catch (error) {
        console.error("Error in setPeers:", error);
    }
}

async function sendAndConfirm(
    connection,
    signers,
    instructions
) {
    try {
        const tx = await buildVersionedTransaction(connection, signers[0].publicKey, instructions, 'confirmed');
        tx.sign(signers);
        const hash = await connection.sendTransaction(tx, { skipPreflight: true });
        const txConfirmedResponse = await connection.confirmTransaction(hash, 'confirmed');
        console.log('Transaction confirmed:', hash);
        console.log('Response:', txConfirmedResponse);
    } catch (error) {
        console.error("Error in sendAndConfirm:", error);
    }
}