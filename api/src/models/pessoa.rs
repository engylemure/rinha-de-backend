use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, FromRow, Row};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct PessoaInput {
    pub apelido: String,
    pub nome: String,
    pub nascimento: String,
    pub stack: Option<Vec<String>>,
}

impl PessoaInput {
    pub fn validate(&self) -> bool {
        self.apelido.len() <= 32
            && self.nome.len() <= 100
            && NaiveDate::parse_from_str(&self.nascimento, "%Y-%m-%d").is_ok()
            && self
                .stack
                .as_ref()
                .map(|stacks| stacks.iter().all(|s| s.len() < 32))
                .unwrap_or(true)
    }
}

impl From<PessoaInput> for Option<Pessoa> {
    fn from(value: PessoaInput) -> Self {
        value.validate().then_some(Pessoa {
            id: Uuid::new_v4().to_string(),
            apelido: value.apelido,
            nome: value.nome,
            nascimento: value.nascimento,
            stack: value.stack
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct Pessoa {
    pub id: String,
    pub apelido: String,
    pub nome: String,
    pub nascimento: String,
    pub stack: Option<Vec<String>>,
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
