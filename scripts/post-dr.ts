import { PostDataRequestInput, Signer, buildSigningConfig, postAndAwaitDataRequest } from '@seda-protocol/dev-tools';

async function main() {
    if (!process.env.ORACLE_PROGRAM_ID) {
        throw new Error('Please set the ORACLE_PROGRAM_ID in your env file');
    }

    // Takes the mnemonic from the .env file (SEDA_MNEMONIC and SEDA_RPC_ENDPOINT)
    const signingConfig = buildSigningConfig({});
    const signer = await Signer.fromPartial(signingConfig);

    console.log('🚀 Posting stock/index price data request using Alpha Vantage API...');
    console.log('This may take a little while to complete...\n');

    // Get symbol from command line argument or use default
    const stockSymbol = process.argv[2] || 'AAPL'; // Fetching stock price in USD
    
    // Map of symbols to descriptions for better display
    const symbolDescriptions: { [key: string]: string } = {
        'AAPL': 'Apple Inc. (Individual Stock)',
        'MSFT': 'Microsoft Corporation (Individual Stock)',
        'GOOGL': 'Alphabet Inc. (Individual Stock)',
        'SPY': 'SPDR S&P 500 ETF (Market Index)',
        'QQQ': 'Invesco QQQ ETF - NASDAQ 100 (Tech Index)',
        'DIA': 'SPDR Dow Jones Industrial Average ETF (Dow Index)'
    };

    const description = symbolDescriptions[stockSymbol.toUpperCase()] || `${stockSymbol.toUpperCase()} (Stock/Index)`;

    console.log(`📊 Requesting data for: ${stockSymbol.toUpperCase()}`);
    console.log(`📝 Description: ${description}\n`);

    const dataRequestInput: PostDataRequestInput = {
        consensusOptions: {
            method: 'none'
        },
        execProgramId: process.env.ORACLE_PROGRAM_ID,
        execInputs: Buffer.from(stockSymbol), // Stock symbol (e.g., AAPL, SPY)
        tallyInputs: Buffer.from([]),
        memo: Buffer.from(`Stock price request for ${stockSymbol} at ${new Date().toISOString()}`),
    };

    const result = await postAndAwaitDataRequest(signer, dataRequestInput, {});
    const explorerLink = process.env.SEDA_EXPLORER_URL ? process.env.SEDA_EXPLORER_URL + `/data-requests/${result.drId}/${result.drBlockHeight}` : "Configure env.SEDA_EXPLORER_URL to generate a link to your DR";

    console.log(`\n✅ Data Request Completed for ${stockSymbol.toUpperCase()}!`);
    console.log(`💰 Price: $${(Number(result.result) / 1000000).toFixed(2)} USD`);
    console.log(`📈 Asset Type: ${description}`);
    
    console.table({
        symbol: stockSymbol.toUpperCase(),
        description: description,
        priceUSD: `$${(Number(result.result) / 1000000).toFixed(2)}`,
        drId: result.drId,
        blockHeight: result.drBlockHeight,
        blockTimestamp: result.blockTimestamp ? result.blockTimestamp.toISOString() : '',
        explorerLink
    });
}

main().catch(console.error);