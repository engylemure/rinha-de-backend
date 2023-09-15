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
    use sql_builder::quote;
    use uuid::Uuid;

    impl PessoaInput {
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
        pub fn insert_query(&self) -> Option<String> {
            sql_builder::SqlBuilder::insert_into("pessoas")
                .fields(&["id", "apelido", "nome", "nascimento", "stack"])
                .values(&[
                    quote(&self.id),
                    quote(&self.apelido),
                    quote(&self.nome),
                    quote(&self.nascimento),
                    quote(format!(
                        "{{{}}}",
                        self.stack
                            .as_ref()
                            .map(|stack| stack.iter().map(quote).collect::<Vec<_>>().join(","))
                            .unwrap_or_default()
                    )),
                ])
                .sql()
                .ok()
        }
    }

    impl From<PessoaInput> for Option<Pessoa> {
        fn from(value: PessoaInput) -> Self {
            value.validate().then_some(Pessoa {
                id: Uuid::new_v4().to_string(),
                apelido: value.apelido,
                nome: value.nome,
                nascimento: value.nascimento,
                stack: value.stack,
            })
        }
    }

    impl From<tokio_postgres::Row> for Pessoa {
        fn from(value: tokio_postgres::Row) -> Self {
            Self {
                id: value.get("id"),
                apelido: value.get("apelido"),
                nome: value.get("nome"),
                nascimento: value.get("nascimento"),
                stack: value.get("stack"),
            }
        }
    }
}
