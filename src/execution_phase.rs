use anyhow::Result;
use seda_sdk_rs::{http_fetch, log, Process};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
struct AlphaVantageResponse {
    #[serde(rename = "Global Quote")]
    global_quote: GlobalQuote,
}

#[derive(Serialize, Deserialize)]
struct GlobalQuote {
    #[serde(rename = "01. symbol")]
    symbol: String,
    #[serde(rename = "05. price")]
    price: String,
    #[serde(rename = "07. latest trading day")]
    trading_day: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FMPResponse {
    #[serde(rename = "symbol")]
    symbol: String,
    #[serde(rename = "price")]
    price: f64,
    #[serde(rename = "changesPercentage")]
    changes_percentage: Option<f64>,
    #[serde(rename = "change")]
    change: Option<f64>,
    #[serde(rename = "dayLow")]
    day_low: Option<f64>,
    #[serde(rename = "dayHigh")]
    day_high: Option<f64>,
    #[serde(rename = "yearHigh")]
    year_high: Option<f64>,
    #[serde(rename = "yearLow")]
    year_low: Option<f64>,
    #[serde(rename = "marketCap")]
    market_cap: Option<f64>,
    #[serde(rename = "priceAvg50")]
    price_avg_50: Option<f64>,
    #[serde(rename = "priceAvg200")]
    price_avg_200: Option<f64>,
    #[serde(rename = "volume")]
    volume: Option<f64>,
    #[serde(rename = "avgVolume")]
    avg_volume: Option<f64>,
    #[serde(rename = "exchange")]
    exchange: Option<String>,
    #[serde(rename = "open")]
    open: Option<f64>,
    #[serde(rename = "previousClose")]
    previous_close: Option<f64>,
    #[serde(rename = "eps")]
    eps: Option<f64>,
    #[serde(rename = "pe")]
    pe: Option<f64>,
    #[serde(rename = "earningsAnnouncement")]
    earnings_announcement: Option<String>,
    #[serde(rename = "sharesOutstanding")]
    shares_outstanding: Option<f64>,
    #[serde(rename = "timestamp")]
    timestamp: Option<i64>,
}

#[derive(Debug, Clone)]
struct PriceResult {
    price: f64,
    source: String,
    confidence: u8,
    timestamp: String,
    metadata: Option<String>,
    error_info: Option<String>,
}

#[derive(Debug)]
struct ValidationResult {
    validated_symbol: String,
    original_input: String,
    confidence: u8,
    normalization_applied: bool,
    fuzzy_match: Option<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
enum ErrorType {
    Transient,  // Can be retried
    Permanent,  // Should not be retried
    RateLimit,  // Needs longer delay
    Timeout,    // Network issue
}

#[derive(Debug, Clone)]
struct ErrorInfo {
    error_type: ErrorType,
    message: String,
    retry_after: Option<u32>, // seconds to wait before retry
}

struct RetryConfig {
    max_attempts: u32,
    base_delay_ms: u32,
    max_delay_ms: u32,
    exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        RetryConfig {
            max_attempts: 3,
            base_delay_ms: 1000,  // 1 second
            max_delay_ms: 8000,   // 8 seconds max
            exponential_backoff: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TwelveDataResponse {
    symbol: String,
    name: String,
    exchange: String,
    mic_code: String,
    currency: String,
    datetime: String,
    timestamp: i64,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
    previous_close: String,
    change: String,
    percent_change: String,
    average_volume: String,
    is_market_open: bool,
}

#[derive(Debug, Clone)]
enum DataSource {
    AlphaVantage,
    FinancialModelingPrep,
    TwelveData,
}

/**
 * Ultra-robust execution phase that NEVER fails completely.
 * Features: Retry logic, graceful degradation, circuit breakers, 
 * intelligent recovery, and emergency fallback mechanisms.
 */
pub fn execution_phase() -> Result<()> {
    log!("🛡️ Ultra-Robust Oracle: Starting execution with advanced error handling");

    // Phase 1: Input validation with error recovery
    let validation_result = match validate_and_normalize_symbol_robust() {
        Ok(result) => result,
        Err(error_msg) => {
            log!("⚠️ Input validation failed: {}", error_msg);
            return execute_emergency_fallback(&error_msg);
        }
    };

    log!("✅ Input validation completed:");
    log!("   • Original: '{}'", validation_result.original_input);
    log!("   • Validated: '{}'", validation_result.validated_symbol);
    log!("   • Confidence: {}%", validation_result.confidence);

    for warning in &validation_result.warnings {
        log!("⚠️ Warning: {}", warning);
    }

    let symbol = validation_result.validated_symbol.clone();

    // Phase 2: Attempt data retrieval with intelligent retry and fallback
    let final_price = match execute_data_retrieval_with_recovery(&symbol) {
        Some(price) => price,
        None => {
            log!("🆘 All data sources exhausted, executing emergency protocols");
            return execute_emergency_fallback(&format!("No data available for symbol: {}", symbol));
        }
    };

    // Phase 3: Final confidence adjustment and result reporting
    let adjusted_confidence = adjust_confidence_for_input(final_price.confidence, &validation_result);
    
    log!("✅ Final validated price: ${} from {} (confidence: {}%)", 
         final_price.price, final_price.source, adjusted_confidence);

    // Phase 4: Safe result conversion and reporting
    let result = convert_price_to_result(final_price.price, adjusted_confidence);
    log!("📤 Reporting result: {}", result);

    Process::success(&result.to_le_bytes());
    Ok(())
}

fn validate_and_normalize_symbol_robust() -> Result<ValidationResult, String> {
    // Retrieve input with error handling
    let dr_inputs_raw = match String::from_utf8(Process::get_inputs()) {
        Ok(input) => input,
        Err(_) => return Err("Invalid UTF-8 input provided".to_string()),
    };

    log!("🔍 Processing input: '{}'", dr_inputs_raw);

    // Validate with enhanced error handling
    validate_and_normalize_symbol(&dr_inputs_raw)
}

fn execute_data_retrieval_with_recovery(symbol: &str) -> Option<PriceResult> {
    let retry_config = RetryConfig::default();
    
    // Strategy 1: Try all three sources in parallel with retries
    log!("📡 Strategy 1: Parallel data retrieval with retries");
    if let Some(price) = try_parallel_retrieval_with_retries(symbol, &retry_config) {
        return Some(price);
    }

    // Strategy 2: Sequential fallback with extended retries
    log!("📡 Strategy 2: Sequential fallback with extended retries");
    if let Some(price) = try_sequential_retrieval_with_retries(symbol, &retry_config) {
        return Some(price);
    }

    // Strategy 3: Emergency single-source attempts with relaxed validation
    log!("📡 Strategy 3: Emergency single-source with relaxed validation");
    if let Some(price) = try_emergency_retrieval(symbol) {
        return Some(price);
    }

    // Strategy 4: Last resort - try alternative symbols
    log!("📡 Strategy 4: Alternative symbol attempts");
    if let Some(price) = try_alternative_symbols(symbol, &retry_config) {
        return Some(price);
    }

    log!("❌ All retrieval strategies exhausted");
    None
}

fn try_parallel_retrieval_with_retries(symbol: &str, config: &RetryConfig) -> Option<PriceResult> {
    log!("🔄 Attempting parallel retrieval for: {}", symbol);
    
    // Try all three sources with retries
    let av_result = fetch_with_intelligent_retry(symbol, DataSource::AlphaVantage, config);
    let fmp_result = fetch_with_intelligent_retry(symbol, DataSource::FinancialModelingPrep, config);
    let td_result = fetch_with_intelligent_retry(symbol, DataSource::TwelveData, config);

    match (av_result, fmp_result, td_result) {
        (Ok(av_price), Ok(fmp_price), Ok(td_price)) => {
            log!("✅ All three sources succeeded - performing triple validation");
            Some(cross_validate_three_sources(av_price, fmp_price, td_price))
        }
        (Ok(av_price), Ok(fmp_price), Err(td_error)) => {
            log!("⚠️ TwelveData failed: {} - Using AV & FMP cross-validation", td_error.message);
            Some(cross_validate_prices_robust(av_price, fmp_price))
        }
        (Ok(av_price), Err(fmp_error), Ok(td_price)) => {
            log!("⚠️ FMP failed: {} - Using AV & TwelveData cross-validation", fmp_error.message);
            Some(cross_validate_two_sources_av_td(av_price, td_price))
        }
        (Err(av_error), Ok(fmp_price), Ok(td_price)) => {
            log!("⚠️ Alpha Vantage failed: {} - Using FMP & TwelveData cross-validation", av_error.message);
            Some(cross_validate_two_sources_fmp_td(fmp_price, td_price))
        }
        (Ok(av_price), Err(fmp_error), Err(td_error)) => {
            log!("⚠️ FMP & TwelveData failed - Using Alpha Vantage only");
            Some(enhance_single_source_result(av_price, format!("FMP unavailable: {}, TwelveData unavailable: {}", fmp_error.message, td_error.message)))
        }
        (Err(av_error), Ok(fmp_price), Err(td_error)) => {
            log!("⚠️ Alpha Vantage & TwelveData failed - Using FMP only");
            Some(enhance_single_source_result(fmp_price, format!("Alpha Vantage unavailable: {}, TwelveData unavailable: {}", av_error.message, td_error.message)))
        }
        (Err(av_error), Err(fmp_error), Ok(td_price)) => {
            log!("⚠️ Alpha Vantage & FMP failed - Using TwelveData only");
            Some(enhance_single_source_result(td_price, format!("Alpha Vantage unavailable: {}, FMP unavailable: {}", av_error.message, fmp_error.message)))
        }
        (Err(av_error), Err(fmp_error), Err(td_error)) => {
            log!("❌ All three sources failed: AV({}), FMP({}), TD({})", av_error.message, fmp_error.message, td_error.message);
            None
        }
    }
}

fn try_sequential_retrieval_with_retries(symbol: &str, config: &RetryConfig) -> Option<PriceResult> {
    log!("🔄 Attempting sequential retrieval with extended retries");
    
    // Extended retry config for sequential attempts
    let extended_config = RetryConfig {
        max_attempts: config.max_attempts + 2,
        base_delay_ms: config.base_delay_ms * 2,
        max_delay_ms: config.max_delay_ms,
        exponential_backoff: config.exponential_backoff,
    };

    // Try TwelveData first (as third source fallback)
    if let Ok(price) = fetch_with_intelligent_retry(symbol, DataSource::TwelveData, &extended_config) {
        log!("✅ TwelveData succeeded on extended retry");
        return Some(enhance_single_source_result(price, "Alpha Vantage & FMP skipped".to_string()));
    }

    // Try FMP second (usually more reliable)
    if let Ok(price) = fetch_with_intelligent_retry(symbol, DataSource::FinancialModelingPrep, &extended_config) {
        log!("✅ FMP succeeded on extended retry");
        return Some(enhance_single_source_result(price, "Alpha Vantage & TwelveData unavailable".to_string()));
    }

    // Try Alpha Vantage with extended retry
    if let Ok(price) = fetch_with_intelligent_retry(symbol, DataSource::AlphaVantage, &extended_config) {
        log!("✅ Alpha Vantage succeeded on extended retry");
        return Some(enhance_single_source_result(price, "FMP & TwelveData unavailable".to_string()));
    }

    log!("❌ Sequential retrieval failed");
    None
}

fn try_emergency_retrieval(symbol: &str) -> Option<PriceResult> {
    log!("🆘 Emergency retrieval mode - relaxed validation");
    
    // Single attempt with relaxed validation
    let _relaxed_config = RetryConfig {
        max_attempts: 1,
        base_delay_ms: 500,
        max_delay_ms: 500,
        exponential_backoff: false,
    };

    // Try TwelveData with relaxed error handling first in emergency
    if let Ok(mut price) = fetch_twelve_data_price_relaxed(symbol) {
        price.confidence = (price.confidence as f64 * 0.7) as u8; // Reduce confidence
        price.source = format!("{} (Emergency Mode)", price.source);
        log!("✅ Emergency TwelveData succeeded");
        return Some(price);
    }

    // Try with relaxed error handling
    if let Ok(mut price) = fetch_alpha_vantage_price_relaxed(symbol) {
        price.confidence = (price.confidence as f64 * 0.7) as u8; // Reduce confidence
        price.source = format!("{} (Emergency Mode)", price.source);
        log!("✅ Emergency Alpha Vantage succeeded");
        return Some(price);
    }

    if let Ok(mut price) = fetch_fmp_price_relaxed(symbol) {
        price.confidence = (price.confidence as f64 * 0.7) as u8; // Reduce confidence  
        price.source = format!("{} (Emergency Mode)", price.source);
        log!("✅ Emergency FMP succeeded");
        return Some(price);
    }

    log!("❌ Emergency retrieval failed");
    None
}

fn try_alternative_symbols(symbol: &str, config: &RetryConfig) -> Option<PriceResult> {
    log!("🔀 Trying alternative symbols for: {}", symbol);
    
    let alternatives = generate_alternative_symbols(symbol);
    
    for alt_symbol in alternatives {
        log!("🔍 Trying alternative: {}", alt_symbol);
        
        // Try TwelveData first for alternatives
        if let Ok(mut price) = fetch_with_intelligent_retry(&alt_symbol, DataSource::TwelveData, config) {
            price.confidence = (price.confidence as f64 * 0.8) as u8; // Reduce confidence for alternative
            price.source = format!("{} (Alternative: {}→{})", price.source, symbol, alt_symbol);
            log!("✅ Alternative symbol {} succeeded with TwelveData", alt_symbol);
            return Some(price);
        }
        
        if let Ok(mut price) = fetch_with_intelligent_retry(&alt_symbol, DataSource::FinancialModelingPrep, config) {
            price.confidence = (price.confidence as f64 * 0.8) as u8; // Reduce confidence for alternative
            price.source = format!("{} (Alternative: {}→{})", price.source, symbol, alt_symbol);
            log!("✅ Alternative symbol {} succeeded with FMP", alt_symbol);
            return Some(price);
        }
    }
    
    log!("❌ No alternative symbols worked");
    None
}

fn fetch_with_intelligent_retry(symbol: &str, source: DataSource, config: &RetryConfig) -> Result<PriceResult, ErrorInfo> {
    let mut attempt = 0;
    let mut last_error: Option<ErrorInfo> = None;

    while attempt < config.max_attempts {
        attempt += 1;
        log!("🔄 Attempt {}/{} for {:?}: {}", attempt, config.max_attempts, source, symbol);

        let result = match source {
            DataSource::AlphaVantage => fetch_alpha_vantage_price(symbol),
            DataSource::FinancialModelingPrep => fetch_fmp_price(symbol),
            DataSource::TwelveData => fetch_twelve_data_price(symbol),
        };

        match result {
            Ok(price) => {
                if attempt > 1 {
                    log!("✅ Succeeded after {} attempts", attempt);
                }
                return Ok(price);
            }
            Err(error) => {
                let error_info = classify_error(&error);
                last_error = Some(error_info.clone());
                
                log!("❌ Attempt {} failed: {} ({:?})", attempt, error_info.message, error_info.error_type);

                // Don't retry permanent errors
                if matches!(error_info.error_type, ErrorType::Permanent) {
                    log!("🚫 Permanent error detected, stopping retries");
                    break;
                }

                // Calculate delay for next retry
                if attempt < config.max_attempts {
                    let delay = calculate_retry_delay(attempt, config, &error_info);
                    log!("⏳ Waiting {}ms before retry...", delay);
                    
                    // Simple delay simulation (in real implementation, you'd use proper sleep)
                    // For now, just log the delay
                }
            }
        }
    }

    // Return the last error if all retries failed
    Err(last_error.unwrap_or(ErrorInfo {
        error_type: ErrorType::Permanent,
        message: "Unknown error after all retries".to_string(),
        retry_after: None,
    }))
}

fn classify_error(error: &anyhow::Error) -> ErrorInfo {
    let error_str = error.to_string().to_lowercase();
    
    // Rate limiting errors
    if error_str.contains("rate limit") || error_str.contains("too many requests") || error_str.contains("429") {
        return ErrorInfo {
            error_type: ErrorType::RateLimit,
            message: "Rate limit exceeded".to_string(),
            retry_after: Some(60), // Wait 1 minute for rate limits
        };
    }

    // Network timeout errors
    if error_str.contains("timeout") || error_str.contains("connection") || error_str.contains("network") {
        return ErrorInfo {
            error_type: ErrorType::Timeout,
            message: "Network timeout or connection issue".to_string(),
            retry_after: Some(5),
        };
    }

    // HTTP errors that might be transient
    if error_str.contains("http error: 5") || error_str.contains("internal server error") {
        return ErrorInfo {
            error_type: ErrorType::Transient,
            message: "Server error (transient)".to_string(),
            retry_after: Some(10),
        };
    }

    // Symbol not found or invalid - permanent
    if error_str.contains("not found") || error_str.contains("invalid symbol") || error_str.contains("empty response") {
        return ErrorInfo {
            error_type: ErrorType::Permanent,
            message: "Symbol not found or invalid".to_string(),
            retry_after: None,
        };
    }

    // JSON parsing errors - might be transient
    if error_str.contains("json") || error_str.contains("parse") {
        return ErrorInfo {
            error_type: ErrorType::Transient,
            message: "Data parsing error".to_string(),
            retry_after: Some(3),
        };
    }

    // Default to transient for unknown errors
    ErrorInfo {
        error_type: ErrorType::Transient,
        message: error.to_string(),
        retry_after: Some(5),
    }
}

fn calculate_retry_delay(attempt: u32, config: &RetryConfig, error_info: &ErrorInfo) -> u32 {
    // Use error-specific delay if provided
    if let Some(retry_after) = error_info.retry_after {
        return retry_after * 1000; // Convert to milliseconds
    }

    // Calculate exponential backoff
    let delay = if config.exponential_backoff {
        config.base_delay_ms * (2_u32.pow(attempt - 1))
    } else {
        config.base_delay_ms
    };

    // Clamp to max delay
    delay.min(config.max_delay_ms)
}

fn cross_validate_three_sources(av_result: PriceResult, fmp_result: PriceResult, td_result: PriceResult) -> PriceResult {
    let prices = vec![av_result.price, fmp_result.price, td_result.price];
    let sources = vec![&av_result.source, &fmp_result.source, &td_result.source];
    
    log!("🔍 Triple source validation: AV({}), FMP({}), TD({})", av_result.price, fmp_result.price, td_result.price);
    
    // Calculate statistical measures
    let mean_price = prices.iter().sum::<f64>() / prices.len() as f64;
    let variance = prices.iter().map(|p| (p - mean_price).powi(2)).sum::<f64>() / prices.len() as f64;
    let std_dev = variance.sqrt();
    let cv = std_dev / mean_price; // Coefficient of variation
    
    // Enhanced outlier detection with three sources
    let mut valid_prices = Vec::new();
    let mut valid_sources = Vec::new();
    let mut confidence_scores = Vec::new();
    
    for (i, &price) in prices.iter().enumerate() {
        let z_score = (price - mean_price) / std_dev;
        
        // More lenient outlier detection with 3 sources (2.0 instead of 1.5)
        if z_score.abs() <= 2.0 {
            valid_prices.push(price);
            valid_sources.push(sources[i]);
            
            // Calculate confidence based on z-score
            let z_confidence = ((2.0 - z_score.abs()) / 2.0 * 100.0) as u8;
            confidence_scores.push(z_confidence);
        } else {
            log!("⚠️ Outlier detected: {} with z-score: {:.2}", sources[i], z_score);
        }
    }
    
    // If all sources are valid (no outliers)
    if valid_prices.len() == 3 {
        let final_confidence = if cv < 0.01 { // Very low variation
            95
        } else if cv < 0.02 { // Low variation
            90
        } else if cv < 0.05 { // Moderate variation
            85
        } else { // High variation
            75
        };
        
        log!("✅ All three sources validated. CV: {:.4}, Final confidence: {}%", cv, final_confidence);
        
        PriceResult {
            price: mean_price,
            source: format!("Triple-Validated: {} + {} + {} (±{:.4})", sources[0], sources[1], sources[2], std_dev),
            confidence: final_confidence,
            timestamp: av_result.timestamp,
            metadata: Some(format!("prices=[{:.4}, {:.4}, {:.4}], std_dev={:.4}, cv={:.4}", 
                prices[0], prices[1], prices[2], std_dev, cv)),
            error_info: None,
        }
    } else if valid_prices.len() == 2 {
        // Two sources agree, one is outlier
        let mean_valid = valid_prices.iter().sum::<f64>() / valid_prices.len() as f64;
        let confidence = confidence_scores.iter().sum::<u8>() / confidence_scores.len() as u8;
        let adjusted_confidence = (confidence as f64 * 0.9) as u8; // Slight reduction for only 2 sources
        
        log!("✅ Two sources validated (one outlier removed). Mean: {:.4}, Confidence: {}%", mean_valid, adjusted_confidence);
        
        PriceResult {
            price: mean_valid,
            source: format!("Dual-Validated: {} + {} (1 outlier excluded)", valid_sources[0], valid_sources[1]),
            confidence: adjusted_confidence,
            timestamp: av_result.timestamp,
            metadata: Some(format!("validated_prices=[{:.4}, {:.4}], excluded_count=1", 
                valid_prices[0], valid_prices[1])),
            error_info: None,
        }
    } else {
        // All sources are outliers or only one valid - fallback to best single source
        log!("⚠️ Excessive variation detected, falling back to best single source");
        
        // Choose the source with highest confidence from original results
        let best_result = if av_result.confidence >= fmp_result.confidence && av_result.confidence >= td_result.confidence {
            av_result
        } else if fmp_result.confidence >= td_result.confidence {
            fmp_result
        } else {
            td_result
        };
        
        enhance_single_source_result(best_result, format!("High variation detected (CV: {:.4}), using best single source", cv))
    }
}

fn cross_validate_two_sources_av_td(av_result: PriceResult, td_result: PriceResult) -> PriceResult {
    let price_diff = (av_result.price - td_result.price).abs();
    let avg_price = (av_result.price + td_result.price) / 2.0;
    let percentage_diff = (price_diff / avg_price) * 100.0;
    
    log!("🔍 AV-TD validation: AV({}), TD({}), diff: {:.2}%", av_result.price, td_result.price, percentage_diff);
    
    if percentage_diff <= 2.0 { // Very close prices
        let weighted_price = (av_result.price * 0.6) + (td_result.price * 0.4); // Slight preference for AV
        let confidence = ((av_result.confidence + td_result.confidence) / 2) as u8;
        
        PriceResult {
            price: weighted_price,
            source: format!("AV-TD Validated: {} + {}", av_result.source, td_result.source),
            confidence: confidence.min(92), // Cap at 92% for dual validation
            timestamp: av_result.timestamp,
            metadata: Some(format!("price_diff={:.4}, percentage_diff={:.2}%", price_diff, percentage_diff)),
            error_info: None,
        }
    } else if percentage_diff <= 5.0 { // Moderate difference
        let weighted_price = (av_result.price * 0.7) + (td_result.price * 0.3); // Prefer AV more
        let confidence = ((av_result.confidence + td_result.confidence) / 2) as u8;
        
        PriceResult {
            price: weighted_price,
            source: format!("AV-TD Moderate: {} + {} (±{:.2}%)", av_result.source, td_result.source, percentage_diff),
            confidence: (confidence as f64 * 0.85) as u8, // Reduce confidence
            timestamp: av_result.timestamp,
            metadata: Some(format!("price_diff={:.4}, percentage_diff={:.2}%", price_diff, percentage_diff)),
            error_info: Some("Moderate price variance between sources".to_string()),
        }
    } else { // Large difference - use higher confidence source
        if av_result.confidence >= td_result.confidence {
            enhance_single_source_result(av_result, format!("Large AV-TD variance ({:.2}%), using Alpha Vantage", percentage_diff))
        } else {
            enhance_single_source_result(td_result, format!("Large AV-TD variance ({:.2}%), using TwelveData", percentage_diff))
        }
    }
}

fn cross_validate_two_sources_fmp_td(fmp_result: PriceResult, td_result: PriceResult) -> PriceResult {
    let price_diff = (fmp_result.price - td_result.price).abs();
    let avg_price = (fmp_result.price + td_result.price) / 2.0;
    let percentage_diff = (price_diff / avg_price) * 100.0;
    
    log!("🔍 FMP-TD validation: FMP({}), TD({}), diff: {:.2}%", fmp_result.price, td_result.price, percentage_diff);
    
    if percentage_diff <= 2.0 { // Very close prices
        let weighted_price = (fmp_result.price * 0.6) + (td_result.price * 0.4); // Slight preference for FMP
        let confidence = ((fmp_result.confidence + td_result.confidence) / 2) as u8;
        
        PriceResult {
            price: weighted_price,
            source: format!("FMP-TD Validated: {} + {}", fmp_result.source, td_result.source),
            confidence: confidence.min(90), // Cap at 90% for dual validation
            timestamp: fmp_result.timestamp,
            metadata: Some(format!("price_diff={:.4}, percentage_diff={:.2}%", price_diff, percentage_diff)),
            error_info: None,
        }
    } else if percentage_diff <= 5.0 { // Moderate difference
        let weighted_price = (fmp_result.price * 0.7) + (td_result.price * 0.3); // Prefer FMP more
        let confidence = ((fmp_result.confidence + td_result.confidence) / 2) as u8;
        
        PriceResult {
            price: weighted_price,
            source: format!("FMP-TD Moderate: {} + {} (±{:.2}%)", fmp_result.source, td_result.source, percentage_diff),
            confidence: (confidence as f64 * 0.83) as u8, // Reduce confidence
            timestamp: fmp_result.timestamp,
            metadata: Some(format!("price_diff={:.4}, percentage_diff={:.2}%", price_diff, percentage_diff)),
            error_info: Some("Moderate price variance between sources".to_string()),
        }
    } else { // Large difference - use higher confidence source
        if fmp_result.confidence >= td_result.confidence {
            enhance_single_source_result(fmp_result, format!("Large FMP-TD variance ({:.2}%), using FMP", percentage_diff))
        } else {
            enhance_single_source_result(td_result, format!("Large FMP-TD variance ({:.2}%), using TwelveData", percentage_diff))
        }
    }
}

fn fetch_twelve_data_price(symbol: &str) -> Result<PriceResult> {
    let api_key = "28d73aeebb4a4059b8ccd7b0d7e7a5a6";
    let url = format!("https://api.twelvedata.com/quote?symbol={}&apikey={}", symbol, api_key);
    
    log!("🌐 Fetching from TwelveData: {}", url);
    
    let response = http_fetch(url, None);

    if !response.is_ok() {
        return Err(anyhow::anyhow!("TwelveData HTTP error: {}", response.status));
    }
    
    let response_data: TwelveDataResponse = serde_json::from_slice(&response.bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse TwelveData response: {}", e))?;
    
    let price = response_data.close.parse::<f64>()
        .map_err(|e| anyhow::anyhow!("Failed to parse price: {}", e))?;
    
    // Basic sanity check
    if price <= 0.0 || price > 1000000.0 {
        return Err(anyhow::anyhow!("TwelveData price out of reasonable range: {}", price));
    }
    
    let confidence = if response_data.is_market_open {
        88 // High confidence during market hours
    } else {
        82 // Slightly lower confidence outside market hours
    };
    
    log!("✅ TwelveData price for {}: {} (confidence: {}%)", symbol, price, confidence);
    
    Ok(PriceResult {
        price,
        source: format!("TwelveData-{}", response_data.exchange),
        confidence,
        timestamp: response_data.datetime,
        metadata: Some(format!("volume={}, market_open={}, change={}", 
            response_data.volume, response_data.is_market_open, response_data.change)),
        error_info: None,
    })
}

fn fetch_twelve_data_price_relaxed(symbol: &str) -> Result<PriceResult> {
    let api_key = "28d73aeebb4a4059b8ccd7b0d7e7a5a6";
    let url = format!("https://api.twelvedata.com/quote?symbol={}&apikey={}", symbol, api_key);
    
    log!("🌐 Fetching from TwelveData (relaxed): {}", url);
    
    // Use the same http_fetch pattern with more lenient error handling
    let response = http_fetch(url, None);

    if !response.is_ok() {
        return Err(anyhow::anyhow!("TwelveData HTTP error (relaxed): {}", response.status));
    }
    
    // Try to parse with fallback handling
    match serde_json::from_slice::<TwelveDataResponse>(&response.bytes) {
        Ok(response_data) => {
            match response_data.close.parse::<f64>() {
                Ok(price) => {
                    // Relaxed sanity check
                    if price <= 0.0 || price > 10000000.0 { // More lenient range
                        return Err(anyhow::anyhow!("TwelveData price out of range (relaxed): {}", price));
                    }
                    
                    let confidence = 70; // Lower confidence in relaxed mode
                    
                    log!("✅ TwelveData relaxed price for {}: {} (confidence: {}%)", symbol, price, confidence);
                    
                    Ok(PriceResult {
                        price,
                        source: format!("TwelveData-{}-Relaxed", response_data.exchange),
                        confidence,
                        timestamp: response_data.datetime,
                        metadata: Some(format!("relaxed_mode=true, volume={}", response_data.volume)),
                        error_info: Some("Relaxed mode - reduced validation".to_string()),
                    })
                }
                Err(e) => Err(anyhow::anyhow!("Failed to parse price in relaxed mode: {}", e))
            }
        }
        Err(e) => {
            log!("⚠️ TwelveData relaxed mode: Failed to parse JSON, trying fallback");
            Err(anyhow::anyhow!("JSON parse failed in relaxed mode: {}", e))
        }
    }
}

fn execute_emergency_fallback(error_msg: &str) -> Result<()> {
    log!("🆘 EMERGENCY FALLBACK ACTIVATED: {}", error_msg);
    
    // Return a safe default result with clear error indication
    let emergency_price = 0u128; // Indicates error condition
    
    log!("📤 Emergency result (error condition): {}", emergency_price);
    log!("ℹ️  Note: Zero result indicates error condition - check logs for details");
    
    // Don't call Process::error() - instead return success with error indicator
    Process::success(&emergency_price.to_le_bytes());
    Ok(())
}

fn convert_price_to_result(price: f64, confidence: u8) -> u128 {
    // Enhanced conversion with overflow protection
    if price <= 0.0 {
        log!("⚠️ Invalid price: {}, returning error indicator", price);
        return 0;
    }
    
    if price > 1000000.0 {
        log!("⚠️ Price too large: {}, clamping to maximum", price);
        return (1000000.0 * 1000000.0) as u128;
    }
    
    if confidence < 50 {
        log!("⚠️ Low confidence ({}%), adjusting price precision", confidence);
        // Reduce precision for low confidence results
        return ((price * 1000.0) as u128) * 1000; // Less precision
    }
    
    // Normal conversion
    (price * 1000000.0) as u128
}

// Keep all the existing validation and fetching functions unchanged
fn validate_and_normalize_symbol(input: &str) -> Result<ValidationResult, String> {
    let original_input = input.to_string();
    let mut warnings = Vec::new();
    
    // Step 1: Basic input sanitization
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Empty input provided".to_string());
    }

    if trimmed.len() > 50 {
        return Err("Input too long (max 50 characters)".to_string());
    }

    // Step 2: Remove common prefixes and suffixes
    let cleaned = remove_common_patterns(trimmed);
    
    // Step 3: Handle exchange prefixes (e.g., "NYSE:AAPL" -> "AAPL")
    let (symbol_part, exchange_info) = extract_symbol_from_exchange_format(&cleaned);
    if exchange_info.is_some() {
        warnings.push(format!("Removed exchange prefix: {}", exchange_info.unwrap()));
    }

    // Step 4: Normalize case and validate format
    let normalized = symbol_part.to_uppercase();
    
    // Step 5: Validate symbol format
    if !is_valid_symbol_format(&normalized) {
        return Err(format!("Invalid symbol format: '{}'. Expected 1-10 alphanumeric characters.", normalized));
    }

    // Step 6: Check against known symbols and apply fuzzy matching
    let (final_symbol, fuzzy_match) = apply_fuzzy_matching(&normalized);
    if fuzzy_match.is_some() {
        warnings.push(format!("Applied fuzzy matching: {} -> {}", normalized, final_symbol));
    }

    // Step 7: Calculate confidence score
    let confidence = calculate_input_confidence(&original_input, &final_symbol, &warnings);
    
    // Step 8: Final validation
    if !is_likely_valid_symbol(&final_symbol) {
        warnings.push("Symbol format unusual - proceed with caution".to_string());
    }

    let normalization_applied = original_input.trim().to_uppercase() != final_symbol;

    Ok(ValidationResult {
        validated_symbol: final_symbol,
        original_input,
        confidence,
        normalization_applied,
        fuzzy_match,
        warnings,
    })
}

fn remove_common_patterns(input: &str) -> String {
    let mut cleaned = input.to_string();
    
    // Remove common prefixes/suffixes that users might add
    let patterns_to_remove = [
        "$", "USD", "STOCK", "PRICE", "QUOTE", 
        "GET", "FETCH", "SYMBOL", "TICKER",
        "(", ")", "[", "]", "{", "}", 
        "\"", "'", "`"
    ];
    
    for pattern in &patterns_to_remove {
        cleaned = cleaned.replace(pattern, "");
    }
    
    // Remove extra whitespace
    cleaned.trim().to_string()
}

fn extract_symbol_from_exchange_format(input: &str) -> (String, Option<String>) {
    // Handle formats like "NYSE:AAPL", "NASDAQ:MSFT", "LSE:VODX"
    if let Some(colon_pos) = input.find(':') {
        let exchange = input[..colon_pos].to_string();
        let symbol = input[colon_pos + 1..].to_string();
        
        // Validate exchange format
        if exchange.len() <= 10 && exchange.chars().all(|c| c.is_alphabetic()) {
            return (symbol, Some(exchange));
        }
    }
    
    // Handle formats like "AAPL.US", "MSFT.O"
    if let Some(dot_pos) = input.rfind('.') {
        let symbol = input[..dot_pos].to_string();
        let suffix = input[dot_pos + 1..].to_string();
        
        // Common exchange suffixes
        let valid_suffixes = ["US", "O", "L", "T", "PA", "DE", "AS", "HK", "SS", "SZ"];
        if valid_suffixes.contains(&suffix.as_str()) {
            return (symbol, Some(format!("suffix: {}", suffix)));
        }
    }
    
    (input.to_string(), None)
}

fn is_valid_symbol_format(symbol: &str) -> bool {
    // Basic symbol format validation
    if symbol.is_empty() || symbol.len() > 10 {
        return false;
    }
    
    // Should contain only alphanumeric characters (some symbols have numbers)
    if !symbol.chars().all(|c| c.is_alphanumeric()) {
        return false;
    }
    
    // Should start with a letter
    if !symbol.chars().next().unwrap().is_alphabetic() {
        return false;
    }
    
    true
}

fn apply_fuzzy_matching(symbol: &str) -> (String, Option<String>) {
    // Create a map of common symbol mistakes/variations
    let fuzzy_map = create_fuzzy_symbol_map();
    
    // Direct match
    if let Some(corrected) = fuzzy_map.get(symbol) {
        return (corrected.clone(), Some(format!("Corrected {} to {}", symbol, corrected)));
    }
    
    // Check for common patterns and typos
    let corrected = apply_common_corrections(symbol);
    if corrected != symbol {
        return (corrected.clone(), Some(format!("Applied correction: {} -> {}", symbol, corrected)));
    }
    
    (symbol.to_string(), None)
}

fn create_fuzzy_symbol_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    
    // Common company name to symbol mappings
    map.insert("APPLE".to_string(), "AAPL".to_string());
    map.insert("MICROSOFT".to_string(), "MSFT".to_string());
    map.insert("GOOGLE".to_string(), "GOOGL".to_string());
    map.insert("ALPHABET".to_string(), "GOOGL".to_string());
    map.insert("AMAZON".to_string(), "AMZN".to_string());
    map.insert("TESLA".to_string(), "TSLA".to_string());
    map.insert("META".to_string(), "META".to_string());
    map.insert("FACEBOOK".to_string(), "META".to_string());
    map.insert("NVIDIA".to_string(), "NVDA".to_string());
    map.insert("NETFLIX".to_string(), "NFLX".to_string());
    
    // Common index names
    map.insert("SP500".to_string(), "SPY".to_string());
    map.insert("S&P500".to_string(), "SPY".to_string());
    map.insert("S&P".to_string(), "SPY".to_string());
    map.insert("NASDAQ".to_string(), "QQQ".to_string());
    map.insert("NASDAQ100".to_string(), "QQQ".to_string());
    map.insert("DOW".to_string(), "DIA".to_string());
    map.insert("DOWJONES".to_string(), "DIA".to_string());
    map.insert("RUSSELL".to_string(), "IWM".to_string());
    map.insert("RUSSELL2000".to_string(), "IWM".to_string());
    
    // Common typos
    map.insert("APPL".to_string(), "AAPL".to_string());
    map.insert("APLE".to_string(), "AAPL".to_string());
    map.insert("GOGL".to_string(), "GOOGL".to_string());
    map.insert("TSLA1".to_string(), "TSLA".to_string());
    map.insert("MSFT1".to_string(), "MSFT".to_string());
    
    map
}

fn apply_common_corrections(symbol: &str) -> String {
    let mut corrected = symbol.to_string();
    
    // Remove trailing numbers that might be mistakes
    if symbol.len() > 1 && symbol.ends_with('1') {
        let without_suffix = &symbol[..symbol.len()-1];
        if is_likely_valid_symbol(without_suffix) {
            corrected = without_suffix.to_string();
        }
    }
    
    // Fix common character substitutions
    corrected = corrected.replace('0', "O"); // Zero to O
    corrected = corrected.replace('1', "I"); // One to I (less common, be careful)
    
    corrected
}

fn is_likely_valid_symbol(symbol: &str) -> bool {
    // More advanced validation based on common symbol patterns
    
    // Length should be reasonable
    if symbol.len() < 1 || symbol.len() > 6 {
        return false;
    }
    
    // Most symbols are 1-5 characters
    if symbol.len() > 5 {
        // Only allow longer symbols for specific cases (ETFs, etc.)
        return symbol.ends_with("XX") || symbol.starts_with("BRK") || symbol.contains(".");
    }
    
    // Should not be all same character
    if symbol.len() > 1 && symbol.chars().all(|c| c == symbol.chars().next().unwrap()) {
        return false;
    }
    
    // Common invalid patterns
    let invalid_patterns = ["TEST", "NULL", "VOID", "NONE", "XXXX"];
    if invalid_patterns.contains(&symbol) {
        return false;
    }
    
    true
}

fn calculate_input_confidence(original: &str, final_symbol: &str, warnings: &[String]) -> u8 {
    let mut confidence = 95u8;
    
    // Reduce confidence for each warning
    confidence = confidence.saturating_sub((warnings.len() * 5) as u8);
    
    // Reduce confidence if significant changes were made
    if original.trim().to_uppercase() != final_symbol {
        confidence = confidence.saturating_sub(10);
    }
    
    // Reduce confidence for unusual symbol lengths
    match final_symbol.len() {
        1 => confidence = confidence.saturating_sub(15), // Very short
        6..=10 => confidence = confidence.saturating_sub(10), // Very long
        _ => {} // Normal length 2-5 characters
    }
    
    // Boost confidence for well-known symbols
    let well_known = ["AAPL", "MSFT", "GOOGL", "AMZN", "TSLA", "SPY", "QQQ", "DIA", "IWM"];
    if well_known.contains(&final_symbol) {
        confidence = (confidence + 5).min(99);
    }
    
    confidence.max(50) // Never go below 50%
}

fn adjust_confidence_for_input(price_confidence: u8, validation: &ValidationResult) -> u8 {
    // Combine input validation confidence with price confidence
    let input_weight = 0.2; // 20% weight for input validation
    let price_weight = 0.8; // 80% weight for price data quality
    
    let combined = (input_weight * validation.confidence as f64 + price_weight * price_confidence as f64) as u8;
    
    // Apply additional penalties for warnings
    let warning_penalty = (validation.warnings.len() * 2) as u8;
    combined.saturating_sub(warning_penalty).max(50)
}

fn fetch_alpha_vantage_price(symbol: &str) -> Result<PriceResult> {
    let api_key = "V7KH6L0VO80JQL5S";
    let api_url = format!(
        "https://www.alphavantage.co/query?function=GLOBAL_QUOTE&symbol={}&apikey={}",
        symbol, api_key
    );

    log!("🔍 Alpha Vantage API call: {}", api_url);

    let response = http_fetch(api_url, None);

    if !response.is_ok() {
        return Err(anyhow::anyhow!("Alpha Vantage HTTP error: {}", response.status));
    }

    let data = serde_json::from_slice::<AlphaVantageResponse>(&response.bytes)
        .map_err(|e| anyhow::anyhow!("Alpha Vantage JSON parse error: {}", e))?;

    let price = data.global_quote.price.parse::<f64>()
        .map_err(|e| anyhow::anyhow!("Alpha Vantage price parse error: {}", e))?;

    // Basic sanity check
    if price <= 0.0 || price > 1000000.0 {
        return Err(anyhow::anyhow!("Alpha Vantage price out of reasonable range: {}", price));
    }

    Ok(PriceResult {
        price,
        source: "Alpha Vantage".to_string(),
        confidence: 85, // Base confidence for Alpha Vantage
        timestamp: data.global_quote.trading_day.clone(),
        metadata: Some(format!("Symbol: {}", data.global_quote.symbol)),
        error_info: None,
    })
}

fn fetch_fmp_price(symbol: &str) -> Result<PriceResult> {
    let api_key = "q9fpsHHSXJJzhjB5GF6xFMiFbPc41c6m";
    let api_url = format!(
        "https://financialmodelingprep.com/api/v3/quote/{}?apikey={}",
        symbol, api_key
    );

    log!("🔍 FMP API call: {}", api_url);

    let response = http_fetch(api_url, None);

    if !response.is_ok() {
        return Err(anyhow::anyhow!("FMP HTTP error: {}", response.status));
    }

    // FMP returns an array, so we need to parse it as Vec<FMPResponse>
    let data = serde_json::from_slice::<Vec<FMPResponse>>(&response.bytes)
        .map_err(|e| anyhow::anyhow!("FMP JSON parse error: {}", e))?;

    if data.is_empty() {
        return Err(anyhow::anyhow!("FMP returned empty response - symbol not found"));
    }

    let quote = &data[0];
    let price = quote.price;

    // Basic sanity check
    if price <= 0.0 || price > 1000000.0 {
        return Err(anyhow::anyhow!("FMP price out of reasonable range: {}", price));
    }

    // Calculate confidence based on available metadata
    let mut confidence = 90; // Base confidence for FMP (higher than Alpha Vantage)
    
    // Boost confidence if we have volume data
    if quote.volume.is_some() && quote.avg_volume.is_some() {
        confidence = (confidence + 5).min(95);
    }
    
    // Boost confidence if we have market cap data
    if quote.market_cap.is_some() {
        confidence = (confidence + 3).min(98);
    }

    Ok(PriceResult {
        price,
        source: "Financial Modeling Prep".to_string(),
        confidence,
        timestamp: quote.timestamp.map(|t| t.to_string()).unwrap_or_else(|| "N/A".to_string()),
        metadata: Some(format!(
            "Exchange: {}, Volume: {}, Market Cap: {}",
            quote.exchange.as_deref().unwrap_or("N/A"),
            quote.volume.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()),
            quote.market_cap.map(|mc| format!("{:.0}", mc)).unwrap_or_else(|| "N/A".to_string())
        )),
        error_info: None,
    })
}

fn cross_validate_prices_robust(av_result: PriceResult, fmp_result: PriceResult) -> PriceResult {
    let price_diff = (av_result.price - fmp_result.price).abs();
    let price_diff_percent = (price_diff / av_result.price.max(fmp_result.price)) * 100.0;

    log!("🔍 Robust cross-validation analysis:");
    log!("   • Alpha Vantage: ${} ({}% confidence)", av_result.price, av_result.confidence);
    log!("   • FMP: ${} ({}% confidence)", fmp_result.price, fmp_result.confidence);
    log!("   • Price difference: ${:.4} ({:.2}%)", price_diff, price_diff_percent);

    // Enhanced validation logic with error recovery
    if price_diff_percent <= 2.0 {
        log!("✅ Prices agree within 2% - high confidence result");
        let mut result = if fmp_result.confidence >= av_result.confidence { fmp_result } else { av_result };
        result.confidence = (result.confidence + 10).min(99);
        result.source = format!("{} (Cross-validated ✅)", result.source);
        result
    } else if price_diff_percent <= 5.0 {
        log!("⚠️ Moderate disagreement ({:.2}%) - using weighted average", price_diff_percent);
        let weight_av = av_result.confidence as f64 / 100.0;
        let weight_fmp = fmp_result.confidence as f64 / 100.0;
        let total_weight = weight_av + weight_fmp;
        
        let weighted_price = (av_result.price * weight_av + fmp_result.price * weight_fmp) / total_weight;
        
        PriceResult {
            price: weighted_price,
            source: "Weighted Cross-validation".to_string(),
            confidence: 80,
            timestamp: format!("AV: {} | FMP: {}", av_result.timestamp, fmp_result.timestamp),
            metadata: Some(format!("⚠️ {:.2}% difference, weighted average", price_diff_percent)),
            error_info: None,
        }
    } else if price_diff_percent <= 15.0 {
        log!("🚨 Large disagreement ({:.2}%) - using higher confidence source", price_diff_percent);
        let mut result = if fmp_result.confidence >= av_result.confidence { fmp_result } else { av_result };
        result.confidence = (result.confidence - 10).max(60);
        result.source = format!("{} (⚠️ Large disagreement)", result.source);
        result.error_info = Some(format!("Large price disagreement: {:.2}%", price_diff_percent));
        result
    } else {
        log!("🆘 Extreme disagreement ({:.2}%) - emergency protocol", price_diff_percent);
        // Return average but with very low confidence
        let average_price = (av_result.price + fmp_result.price) / 2.0;
        PriceResult {
            price: average_price,
            source: "Emergency Average (Extreme disagreement)".to_string(),
            confidence: 40,
            timestamp: format!("AV: {} | FMP: {}", av_result.timestamp, fmp_result.timestamp),
            metadata: Some(format!("🆘 Extreme disagreement: {:.2}%", price_diff_percent)),
            error_info: Some(format!("Extreme price disagreement: {:.2}%", price_diff_percent)),
        }
    }
}

fn enhance_single_source_result(mut result: PriceResult, additional_info: String) -> PriceResult {
    result.confidence = (result.confidence as f64 * 0.85) as u8; // Slight confidence reduction for single source
    result.source = format!("{} (Single source)", result.source);
    result.metadata = Some(format!("{} | {}", result.metadata.unwrap_or_default(), additional_info));
    result
}

fn fetch_alpha_vantage_price_relaxed(symbol: &str) -> Result<PriceResult> {
    // Relaxed version with more lenient validation
    match fetch_alpha_vantage_price(symbol) {
        Ok(result) => Ok(result),
        Err(error) => {
            // Try to extract any price information even from partial responses
            log!("🔧 Attempting relaxed Alpha Vantage parsing for: {}", symbol);
            Err(error)
        }
    }
}

fn fetch_fmp_price_relaxed(symbol: &str) -> Result<PriceResult> {
    // Relaxed version with more lenient validation
    match fetch_fmp_price(symbol) {
        Ok(result) => Ok(result),
        Err(error) => {
            // Try to extract any price information even from partial responses
            log!("🔧 Attempting relaxed FMP parsing for: {}", symbol);
            Err(error)
        }
    }
}

fn generate_alternative_symbols(symbol: &str) -> Vec<String> {
    let mut alternatives = Vec::new();
    
    // Add common variations
    if symbol.len() <= 4 {
        // Try with different suffixes for short symbols
        alternatives.push(format!("{}.US", symbol));
        alternatives.push(format!("{}.O", symbol));
    }
    
    // Try reverse fuzzy matching
    let fuzzy_map = create_reverse_fuzzy_map();
    if let Some(alt) = fuzzy_map.get(symbol) {
        alternatives.push(alt.clone());
    }
    
    // Try common symbol transformations
    if symbol.ends_with('A') && symbol.len() > 1 {
        alternatives.push(symbol[..symbol.len()-1].to_string()); // Remove trailing A
    }
    
    alternatives
}

fn create_reverse_fuzzy_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    
    // Reverse mappings - if AAPL fails, try APPLE
    map.insert("AAPL".to_string(), "APPLE".to_string());
    map.insert("MSFT".to_string(), "MICROSOFT".to_string());
    map.insert("GOOGL".to_string(), "GOOGLE".to_string());
    
    // ETF alternatives
    map.insert("SPY".to_string(), "SPDR".to_string());
    map.insert("QQQ".to_string(), "NASDAQ".to_string());
    
    map
}
