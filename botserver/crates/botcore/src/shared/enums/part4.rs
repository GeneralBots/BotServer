use super::*;

impl ToSql<SmallInt, Pg> for LlmProvider {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let v = *self as i16;
        out.write_all(&v.to_be_bytes())?;
        Ok(serialize::IsNull::No)
    }
}
impl FromSql<SmallInt, Pg> for LlmProvider {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let value = <i16 as FromSql<SmallInt, Pg>>::from_sql(bytes)?;
        match value {
            0 => Ok(Self::OpenAi),
            1 => Ok(Self::Anthropic),
            2 => Ok(Self::AzureOpenAi),
            3 => Ok(Self::AzureClaude),
            4 => Ok(Self::Google),
            5 => Ok(Self::Local),
            6 => Ok(Self::Ollama),
            7 => Ok(Self::Groq),
            8 => Ok(Self::Mistral),
            9 => Ok(Self::Cohere),
            _ => Err(format!("Unknown LlmProvider: {}", value).into()),
        }
    }
}
impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAi => write!(f, "openai"),
            Self::Anthropic => write!(f, "anthropic"),
            Self::AzureOpenAi => write!(f, "azure_openai"),
            Self::AzureClaude => write!(f, "azure_claude"),
            Self::Google => write!(f, "google"),
            Self::Local => write!(f, "local"),
            Self::Ollama => write!(f, "ollama"),
            Self::Groq => write!(f, "groq"),
            Self::Mistral => write!(f, "mistral"),
            Self::Cohere => write!(f, "cohere"),
        }
    }
}

// ============================================================================
// CONTEXT PROVIDER (Vector DB)
// ============================================================================
