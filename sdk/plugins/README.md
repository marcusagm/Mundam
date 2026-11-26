# SDK declarativo de plugins do Mundam

A v1 descreve campos booleanos próprios para o Inspector, ações de navegação a partir da galeria e telas informativas. A navegação para uma seção do Inspector requer um asset ativo selecionado. Uma tela informativa fica disponível no menu **Plugin screens** após a aprovação do plugin no catálogo. O mesmo manifesto pode ser processado por uma ferramenta de autoria, pelo backend e por outro renderizador. [Schema v1](v1/manifest.schema.json) · [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12)

## Validar um manifesto

Use Rust 1.85 ou posterior e execute os exemplos na raiz do repositório.

```text
cargo run --locked --manifest-path sdk/plugins/Cargo.toml -- validate sdk/plugins/v1/examples/reference-review/manifest.json
cargo run --locked --manifest-path sdk/plugins/Cargo.toml -- validate sdk/plugins/v1/examples/windows-verification/manifest.json --api-version 1.0.0
cargo run --locked --manifest-path sdk/plugins/Cargo.toml -- schema
```

`validate` lê o manifesto e seu ícone local. Um manifesto válido produz JSON com `manifest` normalizado e `compatible`, calculado para a plataforma e a arquitetura do processo da CLI. A versão de API usada por padrão é `1.0.0`. Uma declaração válida para outro alvo produz `compatible: false` e mantém o código de saída zero. Um arquivo inválido gera diagnóstico em stderr e código de saída diferente de zero. O comando `schema` imprime o JSON Schema para integração com editores e outras ferramentas.

## Exemplos

| Exemplo                                                                | Alvo          | Contribuição                                               |
| ---------------------------------------------------------------------- | ------------- | ---------------------------------------------------------- |
| [Reference review](v1/examples/reference-review/manifest.json)         | `all/all`     | Campo de revisão e atalhos para abrir a seção no Inspector |
| [Windows verification](v1/examples/windows-verification/manifest.json) | `windows/x64` | Campo próprio exibido somente para leitura                 |

## Regras de autoria

- Declare `manifestVersion: 1`, um ID estável sob o namespace de `publisher` e uma versão de plugin em SemVer. Informe o nome, a descrição, a licença, as palavras-chave e o ícone no manifesto.
- Use pares de alvo. Plataformas canônicas são `all`, `windows`, `mac` e `linux`. Arquiteturas canônicas são `all`, `x64`, `arm64` e `arm` (32 bits). Basta um par completo corresponder. O curinga se aplica apenas ao seu eixo em executores conhecidos.
- Declare uma faixa estável de API como `>=1.0.0 <2.0.0`. O limite inferior é inclusivo e o superior exclusivo. A versão da API segue um ciclo próprio.
- Coloque o ícone estático PNG em `assets/<nome>.png`. Use um nome ASCII minúsculo iniciado por letra, com até 32 caracteres no nome-base. Cada dimensão pode ter de 16 a 256 pixels e o arquivo pode ter até 256 KiB.
- Escreva textos de exibição em uma linha. O arquivo de manifesto pode ter até 64 KiB. Informe de 1 a 12 palavras-chave com até 32 caracteres cada. A CLI deduplica termos após converter para minúsculas e normalizar Unicode em NFC, preservando a primeira grafia.
- Dê a campos e seções IDs locais estáveis, escritos em minúsculas ASCII e iniciados por letra. Cada campo booleano tem `default` e `schemaVersion` positivo. Cada editor vincula um campo existente do próprio manifesto. Use `when.selectionCount: 1` e `readOnly: true` para uma exibição sem edição.
- Defina `viewportActionLabel` em uma seção do Inspector para incluir uma ação em **Plugin actions** na barra da galeria. Use um texto de uma linha com até 80 caracteres, como `Show review in Inspector`. O Mundam qualifica a seção pelo ID do plugin, exibe o nome do plugin junto ao rótulo e usa a ação para abrir o Inspector, expandir a seção e mover o foco até o cabeçalho. A ação herda o contexto de seleção e as permissões da seção.
- Defina `sidebarLinkLabel` no mesmo tipo de seção para incluir um link sob o nome do plugin na área **Plugins** da sidebar. Use um texto de uma linha com até 80 caracteres, como `Open review details`. Os grupos aparecem assim que o plugin é habilitado, e o Mundam disponibiliza a navegação com uma seleção individual ativa.
- Defina `titleBarActionLabel` para incluir uma ação no menu **Title bar plugin actions** da galeria. O rótulo também usa uma linha de até 80 caracteres. O Mundam fornece o ícone do menu, exibe o nome do plugin ao lado da ação e abre sua seção do Inspector. As três contribuições de navegação podem ser declaradas independentemente.
- Declare de uma a quatro telas em `contributes.screens`, cada uma com `id` local estável, `title` de até 80 caracteres e 1 a 12 blocos. Um bloco com `kind: "paragraph"` aceita um `text` de até 240 caracteres. Um bloco com `kind: "list"` aceita de 1 a 12 itens de até 120 caracteres em `items`. O Mundam renderiza esses valores como texto literal. Um plugin que oferece apenas telas pode omitir `assetFields` e `inspectorSections` e declarar `permissions: []`. [Manifesto de referência](v1/examples/reference-review/manifest.json)
- Declare `selection.read` e `assetData.own.read` em `permissions` para usar o inspector. Editores graváveis também exigem `assetData.own.write` nessa lista. As permissões opcionais usam `optionalPermissions` e devem ser distintas das obrigatórias. Toda permissão depende de consentimento e validação do host.

O schema verifica a estrutura. A biblioteca Rust verifica também a ordem do intervalo de API, referências, duplicidades entre listas, vínculo entre ID e namespace e permissões exigidas pelas contribuições. A CLI usa a mesma biblioteca para validar o documento e os bytes da imagem. Os exemplos são diretórios de autoria que podem ser empacotados e assinados pela CLI e instalados em **Settings → Plugins**. A interface atual exibe os valores padrão declarados. A persistência dos valores por asset tem entrega própria. [Plano de implementação](../../docs/report/2026-11-24-api-and-plugins.md)

Para integrar a validação em Rust, use `parse_manifest` sobre os bytes, `validate_icon` sobre o recurso e `supports` com o alvo do executor e a versão de API. Para arquivos de distribuição, use `package::verify_package` com a política de confiança do host. O Cargo.lock da CLI fixa as dependências de leitura JSON, SemVer, normalização Unicode, decodificação PNG e criptografia.

## Empacotar e verificar

A CLI aceita uma chave Ed25519 em PKCS#8 DER. Para reproduzir o ensaio, gere uma chave descartável em um diretório privado fora do repositório e substitua os caminhos nos exemplos pelos caminhos adequados à sua máquina. Para distribuição, consulte o procedimento de custódia de chaves do editor do pacote.

```text
openssl genpkey -algorithm ED25519 -outform DER -out private-ed25519.der
cargo run --locked --manifest-path sdk/plugins/Cargo.toml -- key-fingerprint private-ed25519.der
cargo run --locked --manifest-path sdk/plugins/Cargo.toml -- pack sdk/plugins/v1/examples/reference-review/manifest.json private-ed25519.der review.mundam-plugin
cargo run --locked --manifest-path sdk/plugins/Cargo.toml -- verify review.mundam-plugin org.example <fingerprint-confiavel>
```

`pack` grava um novo arquivo e recusa substituir um pacote existente. Mantenha a chave privada fora dos arquivos que serão publicados. O destinatário precisa obter o namespace do editor e o fingerprint de sua chave pública por um canal confiável independente do arquivo. `verify` usa esse vínculo para validar o pacote e devolve JSON com o manifesto, a compatibilidade, o fingerprint conferido e o SHA-256 do pacote completo. O verificador pertence à biblioteca independente e pode ser chamado pelo futuro gerenciador de instalações. [Formato do pacote v1](v1/package-format.md)
