use crate::rinha::CreatePessoaRequest;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, FromRow, Row};
use uuid::Uuid;

impl CreatePessoaRequest {
    #[inline(always)]
    pub fn validate(&self) -> bool {
        self.apelido.len() <= 32
            && self.nome.len() <= 100
            && NaiveDate::parse_from_str(&self.nascimento, "%Y-%m-%d").is_ok()
            && self.stack.iter().all(|s| s.len() < 32)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Pessoa {
    pub id: String,
    pub apelido: String,
    pub nome: String,
    pub nascimento: String,
    pub stack: Option<Vec<String>>,
}

impl Pessoa {
    #[inline]
    pub fn from(value: CreatePessoaRequest) -> Option<Self> {
        value.validate().then_some(Pessoa {
            id: Uuid::new_v4().to_string(),
            apelido: value.apelido,
            nome: value.nome,
            nascimento: value.nascimento,
            stack: Some(value.stack),
        })
    }
}

impl FromRow<'_, PgRow> for Pessoa {
    fn from_row(row: &'_ PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            nome: row.try_get("nome")?,
            apelido: row.try_get("apelido")?,
            nascimento: row.try_get("nascimento")?,
            stack: row
                .try_get::<Option<String>, &'_ str>("stack")?
                .map(|stacks| stacks.split(' ').map(|s| s.into()).collect()),
        })
    }
}
