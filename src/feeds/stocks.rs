use super::{FeedData, FeedFetcher, StockQuote};
use anyhow::Result;
use async_trait::async_trait;
use futures::future::join_all;
use serde::Deserialize;

pub struct StocksFetcher {
    symbols: Vec<String>,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct YahooChartResponse {
    chart: ChartBody,
}

#[derive(Debug, Deserialize)]
struct ChartBody {
    result: Option<Vec<ChartResult>>,
}

#[derive(Debug, Deserialize)]
struct ChartResult {
    meta: ChartMeta,
}

#[derive(Debug, Deserialize)]
struct ChartMeta {
    symbol: String,
    #[serde(rename = "shortName")]
    short_name: Option<String>,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "chartPreviousClose")]
    chart_previous_close: Option<f64>,
}

impl StocksFetcher {
    pub fn new(symbols: Vec<String>) -> Self {
        Self {
            symbols,
            client: reqwest::Client::new(),
        }
    }

    async fn fetch_symbol(&self, symbol: &str) -> Option<StockQuote> {
        let url = format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1d",
            symbol
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .ok()?;

        let data: YahooChartResponse = response.json().await.ok()?;
        let result = data.chart.result?.into_iter().next()?;
        let meta = result.meta;

        let price = meta.regular_market_price.unwrap_or(0.0);
        let prev_close = meta.chart_previous_close.unwrap_or(price);
        let change = price - prev_close;
        let change_percent = if prev_close != 0.0 {
            (change / prev_close) * 100.0
        } else {
            0.0
        };

        Some(StockQuote {
            symbol: meta.symbol,
            name: meta.short_name.unwrap_or_else(|| "Unknown".to_string()),
            price,
            change,
            change_percent,
        })
    }
}

#[async_trait]
impl FeedFetcher for StocksFetcher {
    async fn fetch(&self) -> Result<FeedData> {
        let futures: Vec<_> = self.symbols.iter().map(|s| self.fetch_symbol(s)).collect();
        let results = join_all(futures).await;
        let quotes: Vec<StockQuote> = results.into_iter().flatten().collect();

        Ok(FeedData::Stocks(quotes))
    }
}
