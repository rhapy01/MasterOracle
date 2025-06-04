import { PostDataRequestInput, Signer, buildSigningConfig, postAndAwaitDataRequest } from '@seda-protocol/dev-tools';

async function fetchStockPrice(signer: any, symbol: string, name: string, sector: string) {
    console.log(`📈 Fetching ${symbol} - ${name} (${sector})...`);
    
    try {
        const dataRequestInput: PostDataRequestInput = {
            consensusOptions: {
                method: 'none'
            },
            execProgramId: process.env.ORACLE_PROGRAM_ID!,
            execInputs: Buffer.from(symbol),
            tallyInputs: Buffer.from([]),
            memo: Buffer.from(`Stock price for ${symbol} at ${new Date().toISOString()}`),
        };

        const result = await postAndAwaitDataRequest(signer, dataRequestInput, {});
        const price = (Number(result.result) / 1000000).toFixed(2);
        
        console.log(`✅ ${symbol}: $${price} USD`);
        
        return {
            symbol,
            name,
            sector,
            price: parseFloat(price),
            priceUSD: `$${price}`,
            drId: result.drId,
            status: 'SUCCESS'
        };
    } catch (error) {
        console.error(`❌ ${symbol} failed:`, error);
        return {
            symbol,
            name,
            sector,
            price: 0,
            priceUSD: 'ERROR',
            drId: 'N/A',
            status: 'FAILED'
        };
    }
}

async function main() {
    if (!process.env.ORACLE_PROGRAM_ID) {
        throw new Error('Please set the ORACLE_PROGRAM_ID in your env file');
    }

    const signingConfig = buildSigningConfig({});
    const signer = await Signer.fromPartial(signingConfig);

    console.log('🚀 Quick 10-Stock Price Collection');
    console.log('📊 Fetching popular stocks from different sectors\n');

    const stocks = [
        { symbol: 'AAPL', name: 'Apple Inc.', sector: 'Technology' },
        { symbol: 'MSFT', name: 'Microsoft Corporation', sector: 'Technology' },
        { symbol: 'GOOGL', name: 'Alphabet Inc.', sector: 'Technology' },
        { symbol: 'AMZN', name: 'Amazon.com Inc.', sector: 'E-commerce' },
        { symbol: 'TSLA', name: 'Tesla Inc.', sector: 'Electric Vehicles' },
        { symbol: 'META', name: 'Meta Platforms Inc.', sector: 'Social Media' },
        { symbol: 'NVDA', name: 'NVIDIA Corporation', sector: 'Semiconductors' },
        { symbol: 'JPM', name: 'JPMorgan Chase & Co.', sector: 'Banking' },
        { symbol: 'JNJ', name: 'Johnson & Johnson', sector: 'Healthcare' },
        { symbol: 'V', name: 'Visa Inc.', sector: 'Financial Services' }
    ];

    const results = [];
    const startTime = Date.now();

    // Fetch each stock price
    for (let i = 0; i < stocks.length; i++) {
        const stock = stocks[i];
        console.log(`\n[${i + 1}/10] Processing ${stock.symbol}...`);
        
        const result = await fetchStockPrice(signer, stock.symbol, stock.name, stock.sector);
        results.push(result);
        
        // Small delay between requests
        if (i < stocks.length - 1) {
            console.log('⏳ Waiting 2 seconds...');
            await new Promise(resolve => setTimeout(resolve, 2000));
        }
    }

    const endTime = Date.now();
    const totalTime = ((endTime - startTime) / 1000 / 60).toFixed(2);

    // Summary
    console.log('\n🎯 FINAL RESULTS - 10 Stock Portfolio:');
    console.log('='.repeat(60));

    const successful = results.filter(r => r.status === 'SUCCESS');
    const failed = results.filter(r => r.status === 'FAILED');

    console.log('\n📊 PORTFOLIO SUMMARY:');
    console.table(results.map(r => ({
        Symbol: r.symbol,
        Company: r.name.length > 20 ? r.name.substring(0, 20) + '...' : r.name,
        Sector: r.sector,
        Price: r.priceUSD,
        Status: r.status
    })));

    if (successful.length > 0) {
        const totalValue = successful.reduce((sum, r) => sum + r.price, 0);
        const avgPrice = totalValue / successful.length;
        const highest = successful.reduce((max, r) => r.price > max.price ? r : max);
        const lowest = successful.reduce((min, r) => r.price < min.price ? r : min);

        console.log('\n💼 PORTFOLIO ANALYTICS:');
        console.log(`   • Execution time: ${totalTime} minutes`);
        console.log(`   • Success rate: ${successful.length}/${results.length} (${((successful.length/results.length)*100).toFixed(1)}%)`);
        console.log(`   • Total value: $${totalValue.toFixed(2)} USD`);
        console.log(`   • Average price: $${avgPrice.toFixed(2)} USD`);
        console.log(`   • Highest: $${highest.price.toFixed(2)} (${highest.symbol})`);
        console.log(`   • Lowest: $${lowest.price.toFixed(2)} (${lowest.symbol})`);
        
        console.log('\n📈 TOP 5 BY PRICE:');
        successful
            .sort((a, b) => b.price - a.price)
            .slice(0, 5)
            .forEach((stock, i) => {
                console.log(`   ${i + 1}. ${stock.symbol}: $${stock.price.toFixed(2)} (${stock.name})`);
            });
    }

    if (failed.length > 0) {
        console.log(`\n❌ FAILED REQUESTS: ${failed.length}`);
        failed.forEach(f => console.log(`   • ${f.symbol} - ${f.name}`));
    }

    console.log('\n🛡️ Oracle powered by SEDA Network - Ultra-robust multi-source validation');
}

main().catch(console.error); 