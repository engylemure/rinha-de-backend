use serde::Deserialize;

#[derive(Deserialize)]
pub struct PessoaInput {
    pub apelido: String,
    pub nome: String,
    pub nascimento: String,
    pub stack: Option<Vec<String>>,
}



impl From<PessoaInput> for crate::rinha::CreatePessoaRequest {
    fn from(PessoaInput { apelido, nome, nascimento, stack }: PessoaInput) -> Self {
        Self {
            apelido,
            nascimento,
            nome,
            stack: stack.unwrap_or_default()
        }
    }
}