use serde::Deserialize;

#[derive(Deserialize)]
pub struct PessoaInput {
    pub apelido: String,
    pub nome: String,
    pub nascimento: String,
    pub stack: Option<Vec<String>>,
}

#[cfg(not(feature = "without_cache"))]
impl From<PessoaInput> for crate::rinha::CreatePessoaRequest {
    fn from(
        PessoaInput {
            apelido,
            nome,
            nascimento,
            stack,
        }: PessoaInput,
    ) -> Self {
        Self {
            apelido,
            nascimento,
            nome,
            stack: stack.unwrap_or_default(),
        }
    }
}

#[cfg(feature = "without_cache")]
pub use without_cache::*;

#[cfg(feature = "without_cache")]
mod without_cache {
    use super::PessoaInput;
    use chrono::NaiveDate;
    use serde::{Deserialize, Serialize};
    use sqlx::FromRow;
    use uuid::Uuid;

    impl PessoaInput {
        #[inline(always)]
        pub fn validate(&self) -> bool {
            self.apelido.len() <= 32
                && self.nome.len() <= 100
                && NaiveDate::parse_from_str(&self.nascimento, "%Y-%m-%d").is_ok()
                && self
                    .stack
                    .as_ref()
                    .map(|stack| stack.iter().all(|s| s.len() <= 32))
                    .unwrap_or(true)
        }
    }

    #[derive(Serialize, Deserialize, Debug, FromRow)]
    pub struct Pessoa {
        pub id: String,
        pub apelido: String,
        pub nome: String,
        pub nascimento: String,
        pub stack: Option<Vec<String>>,
    }

    impl Pessoa {
        #[inline]
        pub fn from(value: PessoaInput) -> Option<Self> {
            value.validate().then_some(Pessoa {
                id: Uuid::new_v4().to_string(),
                apelido: value.apelido,
                nome: value.nome,
                nascimento: value.nascimento,
                stack: value.stack,
            })
        }
    }
}
