import { arrayify, hexZeroPad } from '@ethersproject/bytes';
import { Connection, Keypair, PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { buildVersionedTransaction, ExecutorPDADeriver, oappIDPDA, EndpointProgram, EventPDADeriver, SetConfigType, UlnProgram } from '@layerzerolabs/lz-solana-sdk-v2';
import * as anchor from '@project-serum/anchor';
import { readFileSync } from 'fs';
import { EndpointId } from '@layerzerolabs/lz-definitions';
import dotenv from 'dotenv';

dotenv.config();

const connection = new Connection(process.env.RPC_URL_SOLANA_DEVNET);

const remotePeers = {
    [EndpointId.SEPOLIA_V2_TESTNET]: process.env.SEPOLIA_CONTRACT_ADDRESS, // EVM counter addr
};

const endpointProgram = new EndpointProgram.Endpoint(new PublicKey('76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6')); // endpoint program id
const ulnProgram = new UlnProgram.Uln(new PublicKey('7a4WjyR8VZ7yZz5XJAKm39BUGn5iT9CKcv2pmG9tdXVH')) // uln program id, mainnet and testnet are the same
const executorProgram = new PublicKey('6doghB248px58JSSwG4qejQ46kFMW4AMj7vzJnWZHNZn') // executor program id, mainnet and testnet are the same

const secretKey = JSON.parse(process.env.SOLANA_SECRET_KEY);
const wallet = Keypair.fromSecretKey(new Uint8Array(secretKey));

const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(wallet), { commitment: 'confirmed' });
anchor.setProvider(provider);

let idl;
try {
    idl = JSON.parse(readFileSync('./target/idl/factory_contract.json', 'utf8'));
} catch (error) {
    console.error("Failed to load IDL:", error);
    process.exit(1);
}

const program = new anchor.Program(idl, new PublicKey(process.env.SOLANA_CONTRACT_ADDRESS), provider);

const COUNT_SEED = 'Count';
const counterId = 0;

(async () => {
    try {
        const [countPDA, countBump] = await oappIDPDA(program.programId, COUNT_SEED, counterId);
        await initCounter(connection, wallet, wallet, countPDA, endpointProgram);

        // Uncomment and modify as needed
        for (const [remoteStr, remotePeer] of Object.entries(remotePeers)) {
            const remotePeerBytes = arrayify(hexZeroPad(remotePeer, 32))

            const remote = parseInt(remoteStr);
            await setPeers(connection, wallet, remote, remotePeerBytes);
            await initSendLibrary(connection, wallet, remote);
            await initReceiveLibrary(connection, wallet, remote)
            await initOappNonce(connection, wallet, remote, remotePeerBytes)
            await setSendLibrary(connection, wallet, remote)
            await setReceiveLibrary(connection, wallet, remote)
            await initUlnConfig(connection, wallet, wallet, remote)
            await setOappExecutor(connection, wallet, remote)

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

async function initSendLibrary(connection, admin, remote) {
    const [id] = await oappIDPDA(program.programId, COUNT_SEED, counterId);
    const ix = await endpointProgram.initSendLibrary(connection, admin.publicKey, id, remote);
    if (ix == null) {
        return Promise.resolve();
    }
    sendAndConfirm(connection, [admin], [ix]);
}

async function initReceiveLibrary(connection, admin, remote) {
    const [id] = await oappIDPDA(program.programId, COUNT_SEED, counterId);
    const ix = await endpointProgram.initReceiveLibrary(connection, admin.publicKey, id, remote)
    if (ix == null) {
        return Promise.resolve()
    }
    sendAndConfirm(connection, [admin], [ix])
}

async function initOappNonce(
    connection,
    admin,
    remote,
    remotePeer
) {
    const [id] = await oappIDPDA(program.programId, COUNT_SEED, counterId);
    const ix = await endpointProgram.initOAppNonce(connection, admin.publicKey, remote, id, remotePeer);
    if (ix === null) return Promise.resolve();
    const current = false;
    try {
        const nonce = await endpointProgram.getNonce(connection, id, remote, remotePeer);
        if (nonce) {
            console.log('nonce already set');
            return Promise.resolve();
        }
    } catch (e) {
        /*nonce not init*/
    }
    sendAndConfirm(connection, [admin], [ix]);
}


async function setSendLibrary(connection, admin, remote) {
    const [id] = await oappIDPDA(program.programId, COUNT_SEED, counterId);
    const sendLib = await endpointProgram.getSendLibrary(connection, id, remote)
    const current = sendLib ? sendLib.msgLib.toBase58() : ''
    const [expectedSendLib] = ulnProgram.deriver.messageLib()
    const expected = expectedSendLib.toBase58()
    if (current === expected) {
        return Promise.resolve()
    }
    const ix = await endpointProgram.setSendLibrary(admin.publicKey, id, ulnProgram.program, remote)
    sendAndConfirm(connection, [admin], [ix])
}

async function setReceiveLibrary(connection, admin, remote) {
    const [id] = await oappIDPDA(program.programId, COUNT_SEED, counterId);
    const receiveLib = await endpointProgram.getReceiveLibrary(connection, id, remote);
    const current = receiveLib ? receiveLib.msgLib.toBase58() : '';
    const [expectedMessageLib] = ulnProgram.deriver.messageLib();
    const expected = expectedMessageLib.toBase58();
    if (current === expected) {
        return Promise.resolve();
    }
    const ix = await endpointProgram.setReceiveLibrary(admin.publicKey, id, ulnProgram.program, remote);
    sendAndConfirm(connection, [admin], [ix]);
}

async function initUlnConfig(
    connection,
    payer,
    admin,
    remote
) {
    const [id] = await oappIDPDA(program.programId, COUNT_SEED, counterId);

    const current = await ulnProgram.getSendConfigState(connection, id, remote);
    if (!current) {
        const ix = await endpointProgram.initOappConfig(admin.publicKey, ulnProgram, payer.publicKey, id, remote);
        await sendAndConfirm(connection, [admin], [ix]);
    }
}

async function setOappExecutor(connection, admin, remote) {
    const [id] = await oappIDPDA(program.programId, COUNT_SEED, counterId);
    const defaultOutboundMaxMessageSize = 10000;

    const [executorPda] = new ExecutorPDADeriver(executorProgram).config();
    const expected = {
        maxMessageSize: defaultOutboundMaxMessageSize,
        executor: executorPda,
    };

    const current = (await ulnProgram.getSendConfigState(connection, id, remote))?.executor;
    const ix = await endpointProgram.setOappConfig(connection, admin.publicKey, id, ulnProgram.program, remote, {
        configType: SetConfigType.EXECUTOR,
        value: expected,
    });
    if (
        current &&
        current.executor.toBase58() === expected.executor.toBase58() &&
        current.maxMessageSize === expected.maxMessageSize
    ) {
        return Promise.resolve();
    }
    await sendAndConfirm(connection, [admin], [ix]);
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