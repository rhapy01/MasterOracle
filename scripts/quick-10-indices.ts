import { PostDataRequestInput, Signer, buildSigningConfig, postAndAwaitDataRequest } from '@seda-protocol/dev-tools';

async function fetchIndexPrice(signer: any, symbol: string, name: string, category: string) {
    console.log(`📊 Fetching ${symbol} - ${name} (${category})...`);
    
    try {
        const dataRequestInput: PostDataRequestInput = {
            consensusOptions: {
                method: 'none'
            },
            execProgramId: process.env.ORACLE_PROGRAM_ID!,
            execInputs: Buffer.from(symbol),
            tallyInputs: Buffer.from([]),
            memo: Buffer.from(`Index price for ${symbol} at ${new Date().toISOString()}`),
        };

        const result = await postAndAwaitDataRequest(signer, dataRequestInput, {});
        const price = (Number(result.result) / 1000000).toFixed(2);
        
        console.log(`✅ ${symbol}: $${price} USD`);
        
        return {
            symbol,
            name,
            category,
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
            category,
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

    console.log('🚀 Quick 10-Index Price Collection');
    console.log('📊 Fetching major market indices from different sectors\n');

    const indices = [
        { symbol: 'SPY', name: 'SPDR S&P 500 ETF', category: 'Broad Market' },
        { symbol: 'QQQ', name: 'Invesco QQQ - NASDAQ 100', category: 'Technology' },
        { symbol: 'DIA', name: 'SPDR Dow Jones Industrial', category: 'Blue Chip' },
        { symbol: 'IWM', name: 'iShares Russell 2000', category: 'Small Cap' },
        { symbol: 'VTI', name: 'Vanguard Total Stock Market', category: 'Total Market' },
        { symbol: 'EFA', name: 'iShares MSCI EAFE', category: 'International' },
        { symbol: 'XLK', name: 'Technology Select Sector', category: 'Tech Sector' },
        { symbol: 'XLF', name: 'Financial Select Sector', category: 'Financial Sector' },
        { symbol: 'XLE', name: 'Energy Select Sector', category: 'Energy Sector' },
        { symbol: 'VYM', name: 'Vanguard High Dividend Yield', category: 'Dividend Focus' }
    ];

    const results = [];
    const startTime = Date.now();

    // Fetch each index price
    for (let i = 0; i < indices.length; i++) {
        const index = indices[i];
        console.log(`\n[${i + 1}/10] Processing ${index.symbol}...`);
        
        const result = await fetchIndexPrice(signer, index.symbol, index.name, index.category);
        results.push(result);
        
        // Small delay between requests
        if (i < indices.length - 1) {
            console.log('⏳ Waiting 2 seconds...');
            await new Promise(resolve => setTimeout(resolve, 2000));
        }
    }

    const endTime = Date.now();
    const totalTime = ((endTime - startTime) / 1000 / 60).toFixed(2);

    // Summary
    console.log('\n🎯 FINAL RESULTS - 10 Market Indices Portfolio:');
    console.log('='.repeat(70));

    const successful = results.filter(r => r.status === 'SUCCESS');
    const failed = results.filter(r => r.status === 'FAILED');

    console.log('\n📊 INDICES SUMMARY:');
    console.table(results.map(r => ({
        Symbol: r.symbol,
        Index: r.name.length > 25 ? r.name.substring(0, 25) + '...' : r.name,
        Category: r.category,
        Price: r.priceUSD,
        Status: r.status
    })));

    if (successful.length > 0) {
        const totalValue = successful.reduce((sum, r) => sum + r.price, 0);
        const avgPrice = totalValue / successful.length;
        const highest = successful.reduce((max, r) => r.price > max.price ? r : max);
        const lowest = successful.reduce((min, r) => r.price < min.price ? r : min);

        // Category breakdown
        const categoryBreakdown = successful.reduce((acc, r) => {
            acc[r.category] = (acc[r.category] || 0) + 1;
            return acc;
        }, {} as { [key: string]: number });

        console.log('\n💼 INDICES ANALYTICS:');
        console.log(`   • Execution time: ${totalTime} minutes`);
        console.log(`   • Success rate: ${successful.length}/${results.length} (${((successful.length/results.length)*100).toFixed(1)}%)`);
        console.log(`   • Total value: $${totalValue.toFixed(2)} USD`);
        console.log(`   • Average price: $${avgPrice.toFixed(2)} USD`);
        console.log(`   • Highest: $${highest.price.toFixed(2)} (${highest.symbol})`);
        console.log(`   • Lowest: $${lowest.price.toFixed(2)} (${lowest.symbol})`);
        
        console.log('\n📈 TOP 5 INDICES BY PRICE:');
        successful
            .sort((a, b) => b.price - a.price)
            .slice(0, 5)
            .forEach((index, i) => {
                console.log(`   ${i + 1}. ${index.symbol}: $${index.price.toFixed(2)} (${index.name})`);
            });

        console.log('\n🏷️ CATEGORY BREAKDOWN:');
        Object.entries(categoryBreakdown).forEach(([category, count]) => {
            const percentage = ((count / successful.length) * 100).toFixed(1);
            console.log(`   • ${category}: ${count} indices (${percentage}%)`);
        });

        // Market insights
        console.log('\n📊 MARKET INSIGHTS:');
        const broadMarket = successful.filter(r => r.category.includes('Market') || r.symbol === 'SPY');
        const sectorETFs = successful.filter(r => r.category.includes('Sector'));
        
        if (broadMarket.length > 0) {
            const avgBroadMarket = broadMarket.reduce((sum, r) => sum + r.price, 0) / broadMarket.length;
            console.log(`   • Broad Market Average: $${avgBroadMarket.toFixed(2)} (${broadMarket.length} indices)`);
        }
        
        if (sectorETFs.length > 0) {
            const avgSector = sectorETFs.reduce((sum, r) => sum + r.price, 0) / sectorETFs.length;
            console.log(`   • Sector ETFs Average: $${avgSector.toFixed(2)} (${sectorETFs.length} indices)`);
        }
    }

    if (failed.length > 0) {
        console.log(`\n❌ FAILED REQUESTS: ${failed.length}`);
        failed.forEach(f => console.log(`   • ${f.symbol} - ${f.name}`));
    }

    console.log('\n🛡️ Market Intelligence powered by SEDA Network - Ultra-robust multi-source validation');
    console.log('📈 Comprehensive coverage: Broad Market • Technology • Sectors • International • Dividend Focus');
}

main().catch(console.error); 