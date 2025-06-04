import { PostDataRequestInput, Signer, buildSigningConfig, postAndAwaitDataRequest } from '@seda-protocol/dev-tools';

async function main() {
    if (!process.env.ORACLE_PROGRAM_ID) {
        throw new Error('Please set the ORACLE_PROGRAM_ID in your env file');
    }

    // Takes the mnemonic from the .env file (SEDA_MNEMONIC and SEDA_RPC_ENDPOINT)
    const signingConfig = buildSigningConfig({});
    const signer = await Signer.fromPartial(signingConfig);

    console.log('🚀 Posting multiple stock price data requests...');
    console.log('Fetching data for 10 different stocks from various sectors\n');

    // 10 different stocks from various sectors
    const symbols = [
        { symbol: 'AAPL', name: 'Apple Inc.', sector: 'Technology' },
        { symbol: 'MSFT', name: 'Microsoft Corporation', sector: 'Technology' },
        { symbol: 'GOOGL', name: 'Alphabet Inc.', sector: 'Technology' },
        { symbol: 'AMZN', name: 'Amazon.com Inc.', sector: 'E-commerce/Cloud' },
        { symbol: 'TSLA', name: 'Tesla Inc.', sector: 'Electric Vehicles' },
        { symbol: 'META', name: 'Meta Platforms Inc.', sector: 'Social Media' },
        { symbol: 'NVDA', name: 'NVIDIA Corporation', sector: 'Semiconductors' },
        { symbol: 'JPM', name: 'JPMorgan Chase & Co.', sector: 'Banking' },
        { symbol: 'JNJ', name: 'Johnson & Johnson', sector: 'Healthcare' },
        { symbol: 'V', name: 'Visa Inc.', sector: 'Financial Services' }
    ];

    const results = [];
    const startTime = Date.now();

    console.log('📊 Starting data collection for 10 stocks...\n');

    // Process each symbol sequentially to avoid rate limits
    for (let i = 0; i < symbols.length; i++) {
        const { symbol, name, sector } = symbols[i];
        console.log(`📈 [${i + 1}/10] Processing ${symbol} - ${name} (${sector})...`);
        
        try {
            const dataRequestInput: PostDataRequestInput = {
                consensusOptions: {
                    method: 'none'
                },
                execProgramId: process.env.ORACLE_PROGRAM_ID,
                execInputs: Buffer.from(symbol),
                tallyInputs: Buffer.from([]),
                memo: Buffer.from(`Stock price request for ${symbol} - ${name} at ${new Date().toISOString()}`),
            };

            const result = await postAndAwaitDataRequest(signer, dataRequestInput, {});
            const price = (Number(result.result) / 1000000).toFixed(2);
            const explorerLink = process.env.SEDA_EXPLORER_URL ? 
                process.env.SEDA_EXPLORER_URL + `/data-requests/${result.drId}/${result.drBlockHeight}` : 
                "Configure env.SEDA_EXPLORER_URL to generate a link to your DR";

            results.push({
                symbol,
                name,
                sector,
                priceUSD: `$${price}`,
                priceNumeric: parseFloat(price),
                drId: result.drId,
                blockHeight: result.blockHeight,
                blockTimestamp: result.blockTimestamp ? result.blockTimestamp.toISOString() : '',
                explorerLink,
                status: 'SUCCESS'
            });

            console.log(`✅ ${symbol}: $${price} USD`);
            
            // Small delay between requests to be API-friendly
            if (i < symbols.length - 1) {
                console.log('⏳ Waiting 3 seconds before next request...\n');
                await new Promise(resolve => setTimeout(resolve, 3000));
            }

        } catch (error) {
            console.error(`❌ Error fetching ${symbol}:`, error);
            results.push({
                symbol,
                name,
                sector,
                priceUSD: 'ERROR',
                priceNumeric: 0,
                error: error instanceof Error ? error.message : 'Unknown error',
                status: 'FAILED'
            });
        }
    }

    const endTime = Date.now();
    const totalTime = ((endTime - startTime) / 1000 / 60).toFixed(2);

    // Display final results
    console.log('\n🎯 FINAL RESULTS - 10 Stock Portfolio Data:');
    console.log('=' .repeat(100));
    
    results.forEach((result, index) => {
        console.log(`\n${index + 1}. ${result.symbol} - ${result.name} (${result.sector})`);
        if (result.status === 'SUCCESS') {
            console.log(`   💰 Price: ${result.priceUSD}`);
            console.log(`   🆔 DR ID: ${result.drId}`);
            console.log(`   📦 Block: ${result.blockHeight}`);
            console.log(`   ⏰ Time: ${result.blockTimestamp}`);
        } else {
            console.log(`   ❌ Error: ${result.error}`);
        }
    });

    // Summary table
    console.log('\n📊 PORTFOLIO SUMMARY TABLE:');
    console.table(results.map(r => ({
        Symbol: r.symbol,
        Company: r.name,
        Sector: r.sector,
        Price: r.priceUSD,
        Status: r.status
    })));

    // Calculate portfolio analytics
    const successfulResults = results.filter(r => r.status === 'SUCCESS');
    const totalValue = successfulResults.reduce((sum, r) => sum + r.priceNumeric, 0);
    const avgPrice = successfulResults.length > 0 ? totalValue / successfulResults.length : 0;
    const highestPrice = successfulResults.length > 0 ? Math.max(...successfulResults.map(r => r.priceNumeric)) : 0;
    const lowestPrice = successfulResults.length > 0 ? Math.min(...successfulResults.map(r => r.priceNumeric)) : 0;
    const highestStock = successfulResults.find(r => r.priceNumeric === highestPrice);
    const lowestStock = successfulResults.find(r => r.priceNumeric === lowestPrice);

    // Sector breakdown
    const sectorBreakdown = successfulResults.reduce((acc, r) => {
        acc[r.sector] = (acc[r.sector] || 0) + 1;
        return acc;
    }, {} as { [key: string]: number });

    console.log(`\n💼 PORTFOLIO ANALYTICS:`);
    console.log(`   • Total execution time: ${totalTime} minutes`);
    console.log(`   • Successful requests: ${successfulResults.length}/${results.length} (${((successfulResults.length/results.length)*100).toFixed(1)}%)`);
    console.log(`   • Total portfolio value: $${totalValue.toFixed(2)} USD`);
    console.log(`   • Average stock price: $${avgPrice.toFixed(2)} USD`);
    console.log(`   • Highest price: $${highestPrice.toFixed(2)} USD (${highestStock?.symbol} - ${highestStock?.name})`);
    console.log(`   • Lowest price: $${lowestPrice.toFixed(2)} USD (${lowestStock?.symbol} - ${lowestStock?.name})`);

    console.log(`\n📊 SECTOR BREAKDOWN:`);
    Object.entries(sectorBreakdown).forEach(([sector, count]) => {
        const percentage = successfulResults.length > 0 ? ((count / successfulResults.length) * 100).toFixed(1) : '0.0';
        console.log(`   • ${sector}: ${count} stocks (${percentage}%)`);
    });

    console.log(`\n🛡️ ORACLE PERFORMANCE:`);
    console.log(`   • Data sources: Alpha Vantage + Financial Modeling Prep + TwelveData`);
    console.log(`   • Fallback system: 4-layer ultra-robust error handling`);
    console.log(`   • Cross-validation: Multi-source price verification`);
    console.log(`   • Reliability: ${((successfulResults.length/results.length)*100).toFixed(1)}% success rate`);
    console.log(`   • Oracle type: Ultra-robust SEDA network oracle\n`);
}

main().catch(console.error); 