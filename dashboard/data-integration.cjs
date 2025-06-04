// Data Integration Script for SEDA Oracle Dashboard
// This script bridges the oracle data with the visualization dashboard

const fs = require('fs');
const path = require('path');

class OracleDataIntegrator {
    constructor() {
        this.dataFile = path.join(__dirname, 'oracle-data.json');
        this.initializeDataStore();
    }

    initializeDataStore() {
        const initialData = {
            stocks: [],
            indices: [],
            lastUpdated: new Date().toISOString(),
            totalRequests: 0,
            successfulRequests: 0,
            failedRequests: 0
        };

        if (!fs.existsSync(this.dataFile)) {
            this.saveData(initialData);
        }
    }

    loadData() {
        try {
            const data = fs.readFileSync(this.dataFile, 'utf8');
            return JSON.parse(data);
        } catch (error) {
            console.error('Error loading data:', error);
            return null;
        }
    }

    saveData(data) {
        try {
            fs.writeFileSync(this.dataFile, JSON.stringify(data, null, 2));
            return true;
        } catch (error) {
            console.error('Error saving data:', error);
            return false;
        }
    }

    addStockData(stockData) {
        const data = this.loadData();
        if (!data) return false;

        // Add or update stock data
        const existingIndex = data.stocks.findIndex(s => s.symbol === stockData.symbol);
        if (existingIndex >= 0) {
            data.stocks[existingIndex] = { ...stockData, lastUpdated: new Date().toISOString() };
        } else {
            data.stocks.push({ ...stockData, lastUpdated: new Date().toISOString() });
        }

        data.totalRequests++;
        if (stockData.status === 'SUCCESS') {
            data.successfulRequests++;
        } else {
            data.failedRequests++;
        }

        data.lastUpdated = new Date().toISOString();
        return this.saveData(data);
    }

    addIndexData(indexData) {
        const data = this.loadData();
        if (!data) return false;

        // Add or update index data
        const existingIndex = data.indices.findIndex(i => i.symbol === indexData.symbol);
        if (existingIndex >= 0) {
            data.indices[existingIndex] = { ...indexData, lastUpdated: new Date().toISOString() };
        } else {
            data.indices.push({ ...indexData, lastUpdated: new Date().toISOString() });
        }

        data.totalRequests++;
        if (indexData.status === 'SUCCESS') {
            data.successfulRequests++;
        } else {
            data.failedRequests++;
        }

        data.lastUpdated = new Date().toISOString();
        return this.saveData(data);
    }

    getMarketSummary() {
        const data = this.loadData();
        if (!data) return null;

        const stocksTotal = data.stocks.reduce((sum, stock) => 
            stock.status === 'SUCCESS' ? sum + (stock.price || 0) : sum, 0);
        
        const indicesTotal = data.indices.reduce((sum, index) => 
            index.status === 'SUCCESS' ? sum + (index.price || 0) : sum, 0);

        const successRate = data.totalRequests > 0 ? 
            ((data.successfulRequests / data.totalRequests) * 100).toFixed(1) : '100.0';

        return {
            totalStocks: data.stocks.length,
            totalIndices: data.indices.length,
            totalValue: stocksTotal + indicesTotal,
            successRate: successRate + '%',
            lastUpdated: data.lastUpdated,
            stocksValue: stocksTotal,
            indicesValue: indicesTotal
        };
    }

    generateDashboardData() {
        const data = this.loadData();
        if (!data) return null;

        const summary = this.getMarketSummary();
        
        return {
            summary,
            stocks: data.stocks,
            indices: data.indices,
            charts: {
                stockPrices: data.stocks.filter(s => s.status === 'SUCCESS').map(s => ({
                    label: s.symbol,
                    value: s.price || 0
                })),
                sectorBreakdown: this.getSectorBreakdown(data.stocks),
                indexCategories: this.getCategoryBreakdown(data.indices)
            }
        };
    }

    getSectorBreakdown(stocks) {
        const sectors = {};
        stocks.filter(s => s.status === 'SUCCESS').forEach(stock => {
            if (!sectors[stock.sector]) {
                sectors[stock.sector] = { total: 0, count: 0 };
            }
            sectors[stock.sector].total += stock.price || 0;
            sectors[stock.sector].count += 1;
        });

        return Object.keys(sectors).map(sector => ({
            label: sector,
            value: (sectors[sector].total / sectors[sector].count).toFixed(2),
            count: sectors[sector].count
        }));
    }

    getCategoryBreakdown(indices) {
        const categories = {};
        indices.filter(i => i.status === 'SUCCESS').forEach(index => {
            if (!categories[index.category]) {
                categories[index.category] = { total: 0, count: 0 };
            }
            categories[index.category].total += index.price || 0;
            categories[index.category].count += 1;
        });

        return Object.keys(categories).map(category => ({
            label: category,
            value: (categories[category].total / categories[category].count).toFixed(2),
            count: categories[category].count
        }));
    }

    exportToJSON() {
        const dashboardData = this.generateDashboardData();
        const exportFile = path.join(__dirname, 'dashboard-export.json');
        
        try {
            fs.writeFileSync(exportFile, JSON.stringify(dashboardData, null, 2));
            console.log(`✅ Dashboard data exported to: ${exportFile}`);
            return exportFile;
        } catch (error) {
            console.error('❌ Export failed:', error);
            return null;
        }
    }

    // Method to simulate adding data from oracle results
    simulateOracleData() {
        console.log('🔄 Simulating Oracle Data Integration...');
        
        // Sample stock data (replace with actual oracle results)
        const sampleStocks = [
            { symbol: 'AAPL', name: 'Apple Inc.', sector: 'Technology', price: 203.27, status: 'SUCCESS', drId: 'sample-dr-id-1' },
            { symbol: 'MSFT', name: 'Microsoft Corporation', sector: 'Technology', price: 462.97, status: 'SUCCESS', drId: 'sample-dr-id-2' },
            { symbol: 'GOOGL', name: 'Alphabet Inc.', sector: 'Technology', price: 166.18, status: 'SUCCESS', drId: 'sample-dr-id-3' },
            { symbol: 'AMZN', name: 'Amazon.com Inc.', sector: 'E-commerce', price: 205.71, status: 'SUCCESS', drId: 'sample-dr-id-4' },
            { symbol: 'TSLA', name: 'Tesla Inc.', sector: 'Electric Vehicles', price: 344.27, status: 'SUCCESS', drId: 'sample-dr-id-5' },
            { symbol: 'META', name: 'Meta Platforms Inc.', sector: 'Social Media', price: 289.45, status: 'SUCCESS', drId: 'sample-dr-id-6' },
            { symbol: 'NVDA', name: 'NVIDIA Corporation', sector: 'Semiconductors', price: 892.13, status: 'SUCCESS', drId: 'sample-dr-id-7' },
            { symbol: 'JPM', name: 'JPMorgan Chase & Co.', sector: 'Banking', price: 234.56, status: 'SUCCESS', drId: 'sample-dr-id-8' },
            { symbol: 'JNJ', name: 'Johnson & Johnson', sector: 'Healthcare', price: 178.92, status: 'SUCCESS', drId: 'sample-dr-id-9' },
            { symbol: 'V', name: 'Visa Inc.', sector: 'Financial Services', price: 312.84, status: 'SUCCESS', drId: 'sample-dr-id-10' }
        ];

        // Sample index data
        const sampleIndices = [
            { symbol: 'SPY', name: 'SPDR S&P 500 ETF', category: 'Broad Market', price: 596.09, status: 'SUCCESS', drId: 'sample-dr-id-11' },
            { symbol: 'QQQ', name: 'Invesco QQQ - NASDAQ 100', category: 'Technology', price: 387.45, status: 'SUCCESS', drId: 'sample-dr-id-12' },
            { symbol: 'DIA', name: 'SPDR Dow Jones Industrial', category: 'Blue Chip', price: 423.78, status: 'SUCCESS', drId: 'sample-dr-id-13' },
            { symbol: 'IWM', name: 'iShares Russell 2000', category: 'Small Cap', price: 234.12, status: 'SUCCESS', drId: 'sample-dr-id-14' },
            { symbol: 'VTI', name: 'Vanguard Total Stock Market', category: 'Total Market', price: 289.67, status: 'SUCCESS', drId: 'sample-dr-id-15' },
            { symbol: 'EFA', name: 'iShares MSCI EAFE', category: 'International', price: 87.34, status: 'SUCCESS', drId: 'sample-dr-id-16' },
            { symbol: 'XLK', name: 'Technology Select Sector', category: 'Tech Sector', price: 198.45, status: 'SUCCESS', drId: 'sample-dr-id-17' },
            { symbol: 'XLF', name: 'Financial Select Sector', category: 'Financial Sector', price: 45.67, status: 'SUCCESS', drId: 'sample-dr-id-18' },
            { symbol: 'XLE', name: 'Energy Select Sector', category: 'Energy Sector', price: 89.23, status: 'SUCCESS', drId: 'sample-dr-id-19' },
            { symbol: 'VYM', name: 'Vanguard High Dividend Yield', category: 'Dividend Focus', price: 123.89, status: 'SUCCESS', drId: 'sample-dr-id-20' }
        ];

        // Add sample data
        sampleStocks.forEach(stock => this.addStockData(stock));
        sampleIndices.forEach(index => this.addIndexData(index));

        console.log('✅ Sample data integration completed!');
        
        // Export dashboard data
        this.exportToJSON();
        
        // Display summary
        const summary = this.getMarketSummary();
        console.log('\n📊 Market Summary:');
        console.log(`   • Total Stocks: ${summary.totalStocks}`);
        console.log(`   • Total Indices: ${summary.totalIndices}`);
        console.log(`   • Total Value: $${summary.totalValue.toFixed(2)}`);
        console.log(`   • Success Rate: ${summary.successRate}`);
        console.log(`   • Last Updated: ${summary.lastUpdated}`);
    }
}

// Export for use in other modules
module.exports = OracleDataIntegrator;

// If run directly, simulate data integration
if (require.main === module) {
    const integrator = new OracleDataIntegrator();
    integrator.simulateOracleData();
} 