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

    // You can change to any stock symbol or ETF/index
    // Examples: 'AAPL', 'GOOGL', 'MSFT', 'SPY' (S&P 500), 'QQQ' (NASDAQ), 'DIA' (Dow Jones)
    const stockSymbol = 'AAPL'; // Fetching Apple stock price in USD

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
    console.log(`💰 Price: $${(Number(result.result) / 1000000).toFixed(2)} USD\n`);
    
    console.table({
        symbol: stockSymbol,
        priceUSD: `$${(Number(result.result) / 1000000).toFixed(2)}`,
        drId: result.drId,
        blockHeight: result.drBlockHeight,
        blockTimestamp: result.blockTimestamp ? result.blockTimestamp.toISOString() : '',
        explorerLink
    });
}

main().catch(console.error);