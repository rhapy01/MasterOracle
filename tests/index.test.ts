import { afterEach, describe, it, expect, mock } from "bun:test";
import { file } from "bun";
import { testOracleProgramExecution, testOracleProgramTally } from "@seda-protocol/dev-tools"
import { BigNumber } from 'bignumber.js'

const WASM_PATH = "target/wasm32-wasi/release-wasm/master-oracle.wasm";

const fetchMock = mock();

afterEach(() => {
  fetchMock.mockRestore();
});

describe("stock price data request execution", () => {
  it("should fetch stock prices from Alpha Vantage API", async () => {
    fetchMock.mockImplementation((url) => {
      if (url.includes("alphavantage.co")) {
        // Alpha Vantage response format: {"Global Quote": {"01. symbol": "AAPL", "05. price": "150.25"}}
        return new Response(JSON.stringify({ 
          "Global Quote": {
            "01. symbol": "AAPL",
            "05. price": "150.25",
            "07. latest trading day": "2024-01-15"
          }
        }));
      }

      return new Response('Unknown request');
    });

    const oracleProgram = await file(WASM_PATH).arrayBuffer();

    const vmResult = await testOracleProgramExecution(
      Buffer.from(oracleProgram),
      Buffer.from("AAPL"), // Changed from "ethereum" to "AAPL" (stock symbol)
      fetchMock
    );

    expect(vmResult.exitCode).toBe(0);
    // BigNumber.js is big endian
    const hex = Buffer.from(vmResult.result.toReversed()).toString('hex');
    const result = BigNumber(`0x${hex}`);
    expect(result).toEqual(BigNumber('150250000')); // Updated expected value for $150.25
  });

  it('should tally all results in a single data point', async () => {
    const oracleProgram = await file(WASM_PATH).arrayBuffer();

    // Result from the execution test - updated to match new expected value for $150.25
    let buffer = Buffer.from([0, 228, 247, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    const vmResult = await testOracleProgramTally(Buffer.from(oracleProgram), Buffer.from('tally-inputs'), [{
      exitCode: 0,
      gasUsed: 0,
      inConsensus: true,
      result: buffer,
    }]);

    expect(vmResult.exitCode).toBe(0);
    const hex = Buffer.from(vmResult.result).toString('hex');
    const result = BigNumber(`0x${hex}`);
    expect(result).toEqual(BigNumber('150250000')); // Updated expected value for $150.25
  });
});
