### 

#### If you get this error while running 
```bash
npx hardhat lz:oapp:init:solana --oapp-config ./layerzero.config.ts --solana-secret-key ""
```
```bash
Could not find a default Solana RPC URL for eid <EVM EID> 
```

#### Update devnet-solana package, add EVM RPC of home chain
```bash
node_modules/@layerzerolabs/devtools-solana/dist/index.js
```
```bash
    case lzDefinitions.EndpointId.SOLANA_V2_TESTNET:
    case lzDefinitions.EndpointId.SOLANA_TESTNET:
      return "https://api.devnet.solana.com";
    case lzDefinitions.EndpointId.YOUR_HOMECHAIN_V2_TESTNET:
      return "RPC_URL";
  ```


#### OAPP Config commands

Run these commands with secret key converted with bs58

```bash
 npx hardhat lz:oapp:init:solana --oapp-config ./layerzero.config.ts --solana-secret-key ""

npx hardhat lz:oapp:wire --oapp-config ./layerzero.config.ts --solana-program-id A4zv2BfhBBet6b545PGtHzncj16if43zdCDjfKFpkhNs --solana-secret-key ""
```

#### Use this to convert privateKey to Base58
```bash

import bs58 from 'bs58';

const privateKeyArray = [];

const privateKeyBuffer = Buffer.from(privateKeyArray);

const base58PrivateKey = bs58.encode(privateKeyBuffer);
```