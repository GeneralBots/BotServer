use langchain_rust::{
    chain::{Chain, SQLDatabaseChainBuilder, options::ChainCallOptions},
    llm::openai::OpenAI,
    tools::{postgres::PostgreSQLEngine, SQLDatabaseBuilder},
    prompt::PromptTemplate,
};

pub struct ChartGenerator {
    sql_chain: SQLDatabaseChainBuilder,
    llm: OpenAI,
}

impl ChartGenerator {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let llm = OpenAI::default();
        let engine = PostgreSQLEngine::new(database_url).await?;
        let db = SQLDatabaseBuilder::new(engine).build().await?;

        let sql_chain = SQLDatabaseChainBuilder::new()
            .llm(llm.clone())
            .top_k(4)
            .database(db);

        Ok(Self {
            sql_chain,
            llm,
        })
    }

    pub async fn generate_chart(
        &self,
        question: &str,
        chart_type: &str
    ) -> Result<ChartResponse, Box<dyn std::error::Error>> {
        // Step 1: Generate SQL using LangChain
        let sql_result = self.generate_sql(question).await?;

        // Step 2: Execute SQL and get data
        let data = self.execute_sql(&sql_result).await?;

        // Step 3: Generate chart configuration using LLM
        let chart_config = self.generate_chart_config(&data, chart_type).await?;

        // Step 4: Generate and render chart
        let chart_image = self.render_chart(&chart_config).await?;

        Ok(ChartResponse {
            sql_query: sql_result,
            data,
            chart_image,
            chart_config,
        })
    }

    async fn generate_sql(&self, question: &str) -> Result<String, Box<dyn std::error::Error>> {
        let chain = self.sql_chain
            .clone()
            .build()
            .expect("Failed to build SQL chain");

        let input_variables = chain.prompt_builder().query(question).build();
        let result = chain.invoke(input_variables).await?;

        Ok(result.to_string())
    }

    async fn execute_sql(&self, query: &str) -> Result<Value, Box<dyn std::error::Error>> {
        // Execute the generated SQL and return structured data
        // Implementation depends on your database setup
        Ok(Value::Null)
    }

    async fn generate_chart_config(&self, data: &Value, chart_type: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Given this data: {} and chart type: {}, generate a billboard.js configuration JSON. \
            Focus on creating meaningful visualizations for this business data.",
            data, chart_type
        );

        let message = HumanMessage::new(prompt);
        let result = self.llm.invoke(&[message]).await?;

        serde_json::from_str(&result.generation)
            .map_err(|e| e.into())
    }

    async fn render_chart(&self, config: &Value) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Use headless browser to render chart and capture as image
        // This would integrate with your browser automation setup
        Ok(vec![])
    }
}
