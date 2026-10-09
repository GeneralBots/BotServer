use super::*;

impl ToSql<SmallInt, Pg> for ContextProvider {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let v = *self as i16;
        out.write_all(&v.to_be_bytes())?;
        Ok(serialize::IsNull::No)
    }
}
impl FromSql<SmallInt, Pg> for ContextProvider {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let value = <i16 as FromSql<SmallInt, Pg>>::from_sql(bytes)?;
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Qdrant),
            2 => Ok(Self::Pinecone),
            3 => Ok(Self::Weaviate),
            4 => Ok(Self::Milvus),
            5 => Ok(Self::PgVector),
            6 => Ok(Self::Elasticsearch),
            _ => Err(format!("Unknown ContextProvider: {}", value).into()),
        }
    }
}

// ============================================================================
// TASK STATUS
// ============================================================================
