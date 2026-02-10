# 2026-02-10_13:48-database-write-optimization.md

## Contexto
Durante a análise do subsistema de banco de dados (`src-tauri/src/database.rs`) e do indexador (`src-tauri/src/indexer/mod.rs`), foi identificada uma oportunidade crítica de melhoria de performance: o indexador realizava inserções individuais para cada arquivo descoberto. No SQLite, cada operação de escrita (`INSERT` ou `UPDATE`) fora de uma transação explícita resulta em um commit no disco, o que é extremamente lento em operações de massa.

## Objetivos
- [x] Implementar suporte a transações para operações em lote no banco de dados.
- [x] Refatorar o indexador para agrupar inserções de metadados.
- [x] Garantir a integridade dos dados e o tratamento de conflitos (`ON CONFLICT`).
- [x] Validar a segurança de tipos e o gerenciamento de conexões assíncronas no Rust/SQLx.

## Plano de Implementação

### Fase 1: Refatoração do Banco de Dados (`database.rs`)
- **Helper Interno**: Criação do método `save_image_internal` que aceita uma referência mutável para `SqliteConnection`. Isso permite que a mesma lógica seja usada tanto por conexões individuais quanto por transações.
- **Método de Lote**: Implementação de `save_images_batch`, que recebe um vetor de metadados e executa o loop de salvamento dentro de um bloco `BEGIN ... COMMIT`.
- **Compatibilidade**: Atualização do `save_image` público para adquirir uma conexão do pool e delegar ao helper interno.

### Fase 2: Otimização do Indexador (`indexer/mod.rs`)
- **Worker Loop**: Alteração da lógica no worker do indexador para coletar itens em um lote (baseado no `chunk_size` já calculado) e chamar `save_images_batch`.
- **Coleta de Dados**: Uso de `batch.drain(..).collect()` para transferir a posse dos dados de forma eficiente para o método de banco de dados.

### Fase 3: Hardening e Tipagem
- **Gerenciamento de Executor**: Ajuste para usar referências mutáveis (`&mut SqliteConnection`) em vez de tipos genéricos de `Executor` onde a complexidade de lifetimes do Rust/SQLx causava erros de compilação.
- **Verificação**: Execução de `cargo check` para garantir que as mudanças não introduziram regressões de segurança de memória ou tipos.

## Resultados Obtidos
- **Performance**: Redução drástica no overhead de IO de disco durante a indexação inicial.
- **Robustez**: Manutenção da lógica de "adoção" de arquivos (reconhecimento de movimentação de arquivos entre pastas) mesmo durante o processamento em lote.
- **Código Limpo**: Eliminação de duplicação de lógica de persistência de imagem.

## Próximos Passos
- Monitorar a performance em bibliotecas com mais de 50.000 itens.
- Avaliar a migração para macros `sqlx::query!` para verificação em tempo de compilação (necessário configurar `offline mode` ou banco de desenvolvimento).
