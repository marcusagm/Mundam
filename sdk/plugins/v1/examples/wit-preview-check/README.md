# Verificação de preview com WIT

Este exemplo gera os bindings Rust a partir do [contrato WIT do SDK](../../wit/preview.wit). O guest recebe a miniatura do asset escolhido pelo host, lê os marcadores RIFF e WebP e devolve os oito bytes ao host de teste. O [world de conformidade](../../wit/preview-check.wit) define esse ponto de entrada. O projeto usa um recurso opaco que o Rust libera ao sair da função, inclusive quando uma leitura falha. [Referência de recursos WIT em Rust](https://component-model.bytecodealliance.org/language-support/using-wit-resources/rust.html)

## Compilar e testar

Use Rust 1.87 ou posterior e Node.js 24. Na pasta deste exemplo, execute os comandos com o mesmo toolchain Rust para instalar o alvo e compilar.

```text
rustup target add wasm32-unknown-unknown
cargo build --locked --release --target wasm32-unknown-unknown
npm ci --ignore-scripts
npm test
```

O teste transforma o módulo Rust em componente e grava `target/wasm32-unknown-unknown/release/preview-check.component.wasm`. O host de teste usa os bindings JavaScript gerados para fornecer a miniatura em memória. A suíte confere a interface de preview importada, as duas leituras, a liberação do recurso, os resultados de erro e a rejeição de uma resposta incompleta. [Instanciação com Jco](https://bytecodealliance.github.io/jco/transpiling.html#instantiation)

A prova nativa do Mundam usa o mesmo componente com a ABI experimental `wit-preview-check-v1`. O ensaio integra o pacote assinado ao broker no processo supervisionado. A qualificação para distribuição e a fila de produção continuam em desenvolvimento. [Formato dos pacotes Mundam](../../package-format.md)

Após gerar o componente, é possível executar os testes do auxiliar independente a partir da raiz do repositório com Rust 1.90 ou posterior.

```text
cargo test --locked --manifest-path sdk/plugins/runtime-probe/Cargo.toml --features component-proof --test component -- --include-ignored --test-threads=1
```
