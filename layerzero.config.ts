import { EndpointId } from '@layerzerolabs/lz-definitions';
import dotenv from 'dotenv';

import type { OAppOmniGraphHardhat, OmniPointHardhat } from '@layerzerolabs/toolbox-hardhat';

dotenv.config();

const sepoliaContract: OmniPointHardhat = {
    eid: EndpointId.SEPOLIA_V2_TESTNET,
    address: process.env.SEPOLIA_CONTRACT_ADDRESS || '',
};

const solanaContract: OmniPointHardhat = {
    eid: EndpointId.SOLANA_V2_TESTNET,
    address: "A4zv2BfhBBet6b545PGtHzncj16if43zdCDjfKFpkhNs",
};

const config: OAppOmniGraphHardhat = {
    contracts: [
        {
            contract: sepoliaContract,
        },
        {
            contract: solanaContract,
        },
    ],
    connections: [
        {
            from: sepoliaContract,
            to: solanaContract,
        },
    ],
};

export default config;