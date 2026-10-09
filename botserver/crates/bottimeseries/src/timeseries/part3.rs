use super::*;

impl TimeSeriesClient {
    pub async fn new(config: TimeSeriesConfig) -> Result<Self, TimeSeriesError> {
        let http_client = create_tls_client(Some(30));

    let (write_sender, write_receiver) = mpsc::channel::<MetricPoint>(10000);
    let write_buffer = Arc::new(RwLock::new(Vec::with_capacity(config.batch_size)));

    let client = Self {
        config: config.clone(),
        http_client: http_client.clone(),
        write_sender,
    };

    let buffer_clone = write_buffer;
    let config_clone = config.clone();
        tokio::spawn(async move {
            Self::background_writer(write_receiver, buffer_clone, http_client, config_clone).await;
        });

        Ok(client)
    }

    async fn background_writer(
        mut receiver: mpsc::Receiver<MetricPoint>,
        buffer: Arc<RwLock<Vec<MetricPoint>>>,
        http_client: reqwest::Client,
        config: TimeSeriesConfig,
    ) {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_millis(config.flush_interval_ms));

        loop {
            tokio::select! {
                Some(point) = receiver.recv() => {
                    let mut buf = buffer.write().await;
                    buf.push(point);

                    if buf.len() >= config.batch_size {
                        let points: Vec<MetricPoint> = buf.drain(..).collect();
                        drop(buf);
                        if let Err(e) = Self::flush_points(&http_client, &config, &points).await {
                            log::error!("Failed to flush metrics: {}", e);
                        }
                    }
                }
                _ = interval.tick() => {
                    let mut buf = buffer.write().await;
                    if !buf.is_empty() {
                        let points: Vec<MetricPoint> = buf.drain(..).collect();
                        drop(buf);
                        if let Err(e) = Self::flush_points(&http_client, &config, &points).await {
                            log::error!("Failed to flush metrics: {}", e);
                        }
                    }
                }
            }
        }
    }

    async fn flush_points(
        http_client: &reqwest::Client,
        config: &TimeSeriesConfig,
        points: &[MetricPoint],
    ) -> Result<(), TimeSeriesError> {
        if points.is_empty() {
            return Ok(());
        }

        let line_protocol: String = points
            .iter()
            .map(|p| p.to_line_protocol())
            .collect::<Vec<_>>()
            .join("\n");

        let url = format!(
            "{}/api/v2/write?org={}&bucket={}&precision=ns",
            config.url, config.org, config.bucket
        );

        let response = http_client
            .post(&url)
            .header("Authorization", format!("Token {}", config.token))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(line_protocol)
            .send()
            .await
            .map_err(|e| TimeSeriesError::WriteError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TimeSeriesError::WriteError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        log::debug!("Flushed {} metric points to InfluxDB", points.len());
        Ok(())
    }

    pub async fn write_point(&self, point: MetricPoint) -> Result<(), TimeSeriesError> {
        self.write_sender
            .send(point)
            .await
            .map_err(|e| TimeSeriesError::WriteError(e.to_string()))
    }

    pub async fn write_points(&self, points: Vec<MetricPoint>) -> Result<(), TimeSeriesError> {
        for point in points {
            self.write_point(point).await?;
        }
        Ok(())
    }

    pub async fn query(&self, flux_query: &str) -> Result<QueryResult, TimeSeriesError> {
        let url = format!("{}/api/v2/query?org={}", self.config.url, self.config.org);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Token {}", self.config.token))
            .header("Accept", "application/csv")
            .header("Content-Type", "application/vnd.flux")
            .body(flux_query.to_string())
            .send()
            .await
            .map_err(|e| TimeSeriesError::QueryError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TimeSeriesError::QueryError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let csv_data = response
            .text()
            .await
            .map_err(|e| TimeSeriesError::QueryError(e.to_string()))?;

        Self::parse_csv_result(&csv_data)
    }

    pub(super) fn parse_csv_result(csv_data: &str) -> Result<QueryResult, TimeSeriesError> {
        let mut result = QueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
        };

        let mut lines = csv_data.lines().peekable();

        while let Some(line) = lines.peek() {
            if line.starts_with('#') || line.is_empty() {
                lines.next();
            } else {
                break;
            }
        }

        if let Some(header_line) = lines.next() {
            result.columns = header_line
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        for line in lines {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let values: Vec<serde_json::Value> = line
                .split(',')
                .map(|s| {
                    let trimmed = s.trim();

                    if let Ok(n) = trimmed.parse::<i64>() {
                        serde_json::Value::Number(n.into())
                    } else if let Ok(n) = trimmed.parse::<f64>() {
                        serde_json::json!(n)
                    } else if trimmed == "true" {
                        serde_json::Value::Bool(true)
                    } else if trimmed == "false" {
                        serde_json::Value::Bool(false)
                    } else {
                        serde_json::Value::String(trimmed.to_string())
                    }
                })
                .collect();

            result.rows.push(values);
        }

        Ok(result)
    }

    pub async fn query_range(
        &self,
        measurement: &str,
        start: &str,
        stop: Option<&str>,
        aggregation_window: Option<&str>,
    ) -> Result<QueryResult, TimeSeriesError> {
        let stop_clause = stop.map_or("now()".to_string(), |s| format!("\"{}\"", s));
        let window = aggregation_window.unwrap_or("1m");

        let flux = format!(
            r#"from(bucket: "{}")
  |> range(start: {}, stop: {})
  |> filter(fn: (r) => r._measurement == "{}")
  |> aggregateWindow(every: {}, fn: mean, createEmpty: false)
  |> yield(name: "mean")"#,
            self.config.bucket, start, stop_clause, measurement, window
        );

        self.query(&flux).await
    }

    pub async fn query_last(&self, measurement: &str) -> Result<QueryResult, TimeSeriesError> {
        let flux = format!(
            r#"from(bucket: "{}")
  |> range(start: -1h)
  |> filter(fn: (r) => r._measurement == "{}")
  |> last()"#,
            self.config.bucket, measurement
        );

        self.query(&flux).await
    }

    pub async fn query_stats(
        &self,
        measurement: &str,
        start: &str,
    ) -> Result<QueryResult, TimeSeriesError> {
        let flux = format!(
            r#"from(bucket: "{}")
  |> range(start: {})
  |> filter(fn: (r) => r._measurement == "{}")
  |> group()
  |> reduce(
      identity: {{count: 0.0, sum: 0.0, min: 0.0, max: 0.0}},
      fn: (r, accumulator) => ({{
          count: accumulator.count + 1.0,
          sum: accumulator.sum + r._value,
          min: if accumulator.count == 0.0 then r._value else if r._value < accumulator.min then r._value else accumulator.min,
          max: if accumulator.count == 0.0 then r._value else if r._value > accumulator.max then r._value else accumulator.max
      }})
  )"#,
            self.config.bucket, start, measurement
        );

        self.query(&flux).await
    }

    pub async fn health_check(&self) -> Result<bool, TimeSeriesError> {
        let url = format!("{}/health", self.config.url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| TimeSeriesError::ConnectionError(e.to_string()))?;

        Ok(response.status().is_success())
    }
}
