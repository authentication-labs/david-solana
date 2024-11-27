import { writeFile, mkdir } from 'fs/promises';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { Solita } from '@metaplex-foundation/solita';
import { readFile } from 'fs/promises';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function generateTypeScriptSDK() {
    const generatedIdlDir = path.join(__dirname, '..', 'idl');
    const address = 'CyKce9sNf2SHyLZgS9URiu2o1tDs8UeASzpwtH3dpadt';
    const generatedSDKDir = path.join(__dirname, '..', 'src', 'generated', 'factory_contract');
    const idlPath = path.join(__dirname, '..', 'target', 'idl', 'factory_contract.json');
    const idl = JSON.parse(await readFile(idlPath, 'utf8'));

    // Ensure the generated IDL directory exists
    await mkdir(generatedIdlDir, { recursive: true });

    if (idl.metadata?.address == null) {
        idl.metadata = { ...idl.metadata, address };
        await writeFile(path.join(generatedIdlDir, 'factory_contract.json'), JSON.stringify(idl, null, 2));
    }
    const gen = new Solita(idl, { formatCode: true });
    await gen.renderAndWriteTo(generatedSDKDir);
}

(async () => {
    await generateTypeScriptSDK();
})();