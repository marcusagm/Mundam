# Operações de arquivos e pastas na biblioteca

As edições iniciadas pelo usuário usam `library_operation`. A camada Tauri delega
ao handler de aplicação em `feature/library/operations.rs`, que envia uma intenção
tipada ao Ledger. Os eventos do watcher continuam usando os comandos de observação
existentes: uma observação de rename não deve mover o arquivo novamente.

## Diagnóstico do rename anterior

O frontend separava o caminho original exclusivamente por `/`. Para um caminho
Windows com `\`, isso podia produzir um destino relativo e deslocar o arquivo para
o diretório de execução. Além disso, o comando `rename_file` chamava o rename do
sistema sem impedir substituições e dependia do watcher para reconstruir a
identidade no catálogo. Esses riscos foram removidos do fluxo de rename; isso não
comprova, por si só, qual deles causou uma perda histórica específica.

## Funcionalidades

- Menu de contexto das pastas: criar subpasta, renomear, mover, excluir, escolher
  cor, abrir no explorador e copiar caminho.
- Menu dos assets: renomear preservando extensão e mover uma seleção para uma
  pasta indexada. Também é possível arrastar assets sobre uma pasta.
- Cor personalizada, herança dinâmica da pasta pai e restauração da cor padrão.
  A migração `20261105020000_folder_colors.sql` mantém a preferência por pasta.
- Cor usa o mesmo submenu `ColorPicker` das tags, com **Use Parent Color** e reset.
  O rename ocorre inline na árvore, usando `editingId` e `onRename` do `TreeView`.
- **Use Parent Color** e **Recursive View** reutilizam `ContextMenuCheckboxItem`,
  que compõe o `Checkbox` existente. **Reset Color** só aparece para uma cor
  personalizada ativa, tanto em pastas quanto em tags. O reset das tags é uma
  intenção explícita que limpa a cor sem ativar herança.
- Criar subpasta usa um nome disponível (`New Folder`, `New Folder (1)`, ...) e abre
  o editor inline após a criação. O seletor de destino usa `TreeView`, excluindo a
  própria pasta e seus descendentes; o botão de confirmação exige uma seleção.
- O comando do explorador usa **Reveal in OS**, como o menu de assets.
- Arrastar uma pasta para outra move a subárvore completa, sempre para dentro do
  destino. O contrato de DnD distingue `FOLDER` de `ASSET`; ciclos e self-drop são
  rejeitados antes do IPC e também no backend.
- Raízes vinculadas também podem ser renomeadas e movidas para dentro de outra
  pasta. O handler de aplicação remove registros antigos do watcher após o commit
  e volta a observar o novo caminho quando ele permanece uma raiz. Se o destino
  já está dentro de outra raiz monitorada, essa observação recursiva passa a cobrir
  a pasta movida. Falhas de reconfiguração retornam um aviso e são repetidas automaticamente,
  sem apresentar a alteração já concluída como uma falha de escrita.
- A exclusão recuperável continua restrita às subpastas; raízes oferecem desvincular.

O IPC exige tanto o registro em `generate_handler!` quanto a permissão
`allow-library-operation` em `permissions/main.toml` e sua inclusão em
`capabilities/default.json`. A ausência da capability bloqueia todas as operações
com `library_operation not allowed`. Mudanças nesses arquivos exigem reiniciar o
processo Tauri, pois o hot reload do frontend não atualiza permissões do binário.

Exemplo de chamada:

```typescript
await executeLibraryOperation({
    kind: 'moveAssets',
    assetIds: ['asset-uuid'],
    destinationId: 'folder-uuid'
});
```

## Integridade dos dados

O adaptador abre uma transação `BEGIN IMMEDIATE`, valida os destinos e prepara as
mutações. Os caminhos são resolvidos pelo ID registrado; o frontend não monta
caminhos nem envia um destino arbitrário. Nomes com separadores, componentes de
travessia e nomes reservados do Windows são rejeitados. Pastas não podem ser
movidas para si mesmas nem para descendentes.

Os movimentos usam primitivas nativas com exclusão de destino: `MoveFileExW`
sem substituição no Windows, `renameat2(RENAME_NOREPLACE)` no Linux e
`renamex_np(RENAME_EXCL)` no macOS. Assim, um destino criado entre a validação e o
movimento também é protegido. Links simbólicos não são movidos diretamente.

As observações são reconciliadas com o catálogo e a identidade nativa dos arquivos.
Eventos dentro de uma pasta recém-criada ou renomeada não são descartados por
uma janela de supressão por ancestral. O gate do Ledger serializa as mutações
coordenadas e as escritas de observação. O rename de assets preserva ID, notas,
avaliação e associações; uma colisão não apaga outro registro para liberar caminho.
As alterações de subárvores usam a hierarquia e componentes de caminho, evitando
`LIKE`, seus curingas `%`/`_` e a dependência de separadores `/` no Windows.

Se uma etapa de disco ou o commit falhar, os movimentos concluídos são revertidos
em ordem inversa. Se a compensação também falhar, o erro informa onde os arquivos
permanecem. Eventos de atualização são emitidos após o commit; falhas de refresh
não transformam uma operação já concluída em uma falha de escrita.

### Exclusão e recuperação

A exclusão de uma subpasta move seu conteúdo integral para
`<app-local-data>/deleted-folders/<uuid>-<nome>`, antes de confirmar a remoção dos
registros. O aviso de sucesso oferece **Undo** e permanece disponível até ser
dispensado. A migração `20261105030000_folder_recovery.sql` registra um snapshot
transacional da estrutura, cores e dados de assets, incluindo IDs, notas, ratings,
favoritos e associações de tags. Undo move os arquivos de volta e restaura esse
catálogo na mesma transação coordenada; se o local estiver ocupado ou o pai tiver
sido removido ou movido, ele recusa a restauração e preserva a cópia para nova
tentativa. A notificação de erro oferece **Try Undo Again**. Índices de duplicatas,
jobs e thumbnails são regenerados, sem reativar leases antigos de processamento.

O acesso direto está somente em **Settings → Trash → Open Folder Recovery**, com
uma indicação de uso avançado. Alterações manuais podem inviabilizar o Undo.
Essa recuperação é separada da retenção automática da lixeira de assets. A nova
permissão `allow-folder-recovery-directory` autoriza a resolução do diretório;
nenhuma pasta é aberta automaticamente durante a exclusão.

Exclusões anteriores à migração que não têm snapshot continuam recuperáveis apenas
como arquivos pelo explorador. Desvincular uma raiz continua mantendo
os originais no local de origem. Thumbnails e previews dos registros removidos
são limpos após o commit, sem tratar falhas de limpeza como perda dos originais.

### Limites operacionais

- Movimentos entre volumes são recusados pelo sistema operacional sem apagar a
  origem. Isso também vale para exclusão recuperável se o diretório de dados do
  aplicativo estiver em outro volume.
- O journal durável permite recuperar interrupções no boot antes de iniciar os
  sensores. Situações cuja propriedade ou identidade não pode ser provada são
  preservadas para reparo, em vez de sobrescrever caminhos ou apagar arquivos.
- macOS e Linux têm implementação própria; a validação inicial foi feita no Windows
  e complementada no macOS (veja abaixo).

## Correções encontradas no macOS — histórico anterior à checagem

- **Copiar caminho sem efeito:** o menu de pastas usa `writeText` do plugin
  `clipboard-manager`, cujo conjunto `clipboard-manager:default` não concede
  escrita. A capability `default.json` agora inclui
  `clipboard-manager:allow-write-text`. O Windows funcionava porque a falha não era
  visível no fluxo testado; a permissão é necessária em todas as plataformas.
- **Asset copiado para pasta nova não era indexado:** o `LedgerPathGuard` ignora
  eventos do watcher para um caminho marcado **e todos os seus descendentes** por
  15 segundos. A criação de pasta marcava o diretório novo, então arquivos copiados
  para ele nesse intervalo eram descartados e nenhum novo evento os reapresentava.
  A criação não marca mais o caminho: o registro já existe no banco antes de o
  evento de diretório chegar ao indexer (`handle_directory_discovered` o ignora).
  Movimentos e renames continuam marcados. O teste
  `created_folder_does_not_suppress_files_copied_into_it` cobre esse caso.
- **Arquivo copiado logo após criar/renomear a pasta:** o fluxo normal cria
  “New Folder” e em seguida a renomeia no editor inline; o rename marca origem e
  destino, e `is_marked` considera ancestrais, então qualquer arquivo novo dentro da
  pasta era ignorado por 15 s (a reindexação no boot o encontrava). O criador de
  arquivos agora usa `LedgerPathGuard::is_exactly_marked` (apenas o caminho exato) em
  `handle_file_discovered`. Arquivos que já foram movidos junto com uma subárvore
  continuam sem duplicar porque `handle_file_creation` consulta o banco antes de
  criar. Eventos de remoção e rename seguem usando a supressão por ancestral. O
  teste `exact_check_ignores_marked_ancestors` cobre a diferença entre os dois
  métodos. Na prática, o problema aparecia apenas quando o arquivo era copiado em
  poucos segundos; após a janela de 15 s tudo funcionava, e o scan do boot sempre
  recuperava o arquivo.
- **Esc mantinha “New Folder”:** a pasta é criada em disco antes da edição inline.
  `useFolderTreeActions` memoriza a pasta pendente; cancelar com Esc executa
  `deleteFolder` (recuperável) para descartá-la, enquanto confirmar, mesmo mantendo
  o nome padrão, a preserva. Um `blur` posterior ao fechamento do editor é ignorado
  para não confirmar uma edição obsoleta.
- **Testes Rust no macOS:** `cargo test --lib` falhava com
  `symbol _EMBED_INFO_PLIST is already defined` porque `lib.rs` e
  `thumbnail_ipc_tests.rs` expandiam `tauri::generate_context!` no mesmo binário de
  teste. A expansão agora existe só em `application_context::<R>()` (`lib.rs`),
  reutilizada pelo teste com `MockRuntime`, mantendo a validação da ACL real.
- **Testes específicos do Windows:** dois testes assumiam Windows e falhavam no macOS
  sem indicar bug do produto. `test_is_subpath` verifica separadores `\\`, letras de
  unidade e comparação sem diferenciar maiúsculas, e agora é `#[cfg(windows)]`, com
  `test_is_subpath_unix` cobrindo fronteira de componente e diferenciação de
  maiúsculas. O teste de IPC de thumbnails usava a origem `http://tauri.localhost`;
  fora do Windows a origem local é `tauri://localhost`, então a ACL rejeitava a
  janela `main`. A origem agora é escolhida por plataforma (`LOCAL_APP_URL`), o que
  também faz as rejeições da splashscreen validarem a ACL, e não apenas a origem
  errada.

Validação dessa revisão no macOS: `cargo test --lib` passou por completo
(240 testes). Os relatos de supressão e cancelamento acima descrevem a solução
anterior; os fluxos atuais estão detalhados na seção final deste documento.

## Melhorias adicionais avaliadas

Abrir no explorador, copiar caminho e resetar a cor foram incluídos por serem
úteis no fluxo de organização. Lista registrada antes da checagem (os itens 1, 2
e 4 foram atendidos no escopo descrito na seção final; movimento entre volumes,
rename em lote e duplicação continuam como evolução futura):

1. Histórico de operações e desfazer, com journal durável e recuperação no boot.
2. Histórico de exclusões com lista de restauração e gestão do espaço de recuperação.
3. Cópia e movimento entre volumes com staging, verificação de conteúdo e retomada.
4. Recuperação automática de falhas de reconfiguração do watcher, com retry no boot.
5. Rename em lote com prévia, detecção de conflitos e modelos de nomes.
6. Duplicação de assets e pastas com política explícita para tags e notas.

## Verificação

Os testes em `infra/database/library_operations/tests.rs` usam arquivos temporários
e o esquema migrado para verificar preservação de conteúdo, identidade e notas,
colisões, nomes inválidos, movimento de subárvores com `%`/`_`, compensação de
movimentos parcialmente executados e exclusão recuperável. Os testes frontend de
`folderPresentation.spec.ts` cobrem herança, mudança de cor de ancestral,
cor padrão, filtragem de descendentes e proteção contra ciclos.

Validação inicial: build Vite, TypeScript e ESLint sem erros;
suíte frontend de 139 testes e execução adicional dos seis testes de apresentação,
diálogo e drop aprovadas (três destes já faziam parte da suíte de 139).
`cargo check --offline` aprovado. Os testes Rust foram compilados com a toolchain
MSVC usada pelo script Windows do projeto, mas o executável foi encerrado pelo
runtime com `0xc0000139 / STATUS_ENTRYPOINT_NOT_FOUND`, antes de iniciar os testes.
Portanto, a execução dos testes de disco e banco e das regressões existentes de
pastas permanece pendente em um ambiente Windows com o runtime funcional.

Na revisão de UX e IPC, build Vite, TypeScript e ESLint passaram novamente.
`cargo +stable-x86_64-pc-windows-gnu check --offline -j 2` também passou, incluindo
a geração da ACL Tauri. A checagem MSVC havia sido interrompida durante a recompilação
das dependências para limitar o uso de recursos da máquina.
Doze regressões frontend cobrem disponibilidade do comando na ACL, edição inline,
falha e retry de rename, movimento de raízes para outra pasta, prevenção de ciclos,
payload `FOLDER`, nesting nas bordas das linhas e preservação da reordenação de tags.
Foi adicionado também um teste Rust com arquivos reais para rename de raiz seguido
de movimento para outra raiz, preservando conteúdo e propriedade dos assets.

Na revisão de Undo e padronização dos controles, vinte testes frontend passaram:
seleção por TreeView, criação inline, Undo/retry, regras de reset e checkboxes para
pastas e tags, e acesso explícito pela seção de lixeira. Build Vite, TypeScript e
ESLint também passaram. `cargo check --tests --offline -j 2` com Windows GNU
compilou o backend e os testes, incluindo roundtrip de restauração de arquivos e
metadados, colisão seguida de retry e reset de cor de tags. Os testes nativos não
foram executados nesta revisão; a limitação de runtime descrita acima permanece
separada da compilação aprovada. A execução no macOS foi validada depois (veja “Correções encontradas no macOS”).

## Checagem de conclusão — 2026-11-06

### Base, alcance e parecer

Revisão do commit `2c7569db4ecf96407ed4c38d7db8a47631f5f9c7`, confrontando
os fluxos de biblioteca, integração com watcher, recuperação, DnD, componentes
compartilhados e menus de tags com os guias de backend, frontend e documentação.
A árvore de trabalho estava limpa no início da revisão. Esta seção preserva os
achados históricos e as ações propostas naquele commit. A implementação posterior
está registrada na última seção; os trechos de comportamento abaixo não descrevem
necessariamente o código atual.

Os testes funcionais relatados no Windows e as correções/testes no macOS são uma
boa base de conclusão funcional. O registro acima informa `cargo test --lib`
aprovado no macOS com 240 testes; essa execução é evidência do relato de validação,
não uma execução repetida nesta revisão Windows. Não se deve interpretar os testes
aprovados como cobertura de crash, falha na compensação ou concorrência externa.

**Parecer:** a funcionalidade está implementada e os fluxos relatados estão
validados, mas a conformidade arquitetural, documental e de resiliência é
**parcial**. Não é adequado encerrar o trabalho afirmando que todos os padrões
estão atendidos ou que disco e SQLite formam uma transação ACID única. Os itens P1
abaixo devem ser tratados antes de declarar a etapa de endurecimento concluída.

Checagens locais adicionais, sem alteração do código de aplicação:

- `npm run typecheck`: aprovado.
- ESLint em 33 arquivos do recorte de biblioteca, menus, TreeView e contratos:
  **0 erros e 420 avisos**. Destes, **418** são `prettier/prettier` por finais de
  linha, **1** é `solid/reactivity` e **1** é `complexity`.
- A complexidade de `handleDragOver` em
  [`useTreeDragDrop.ts`](../../src/components/ui/TreeView/hooks/useTreeDragDrop.ts#L151)
  é **13**, acima do limite **10**. Ambos os limites de complexidade e tamanho de
  arquivo estão configurados como avisos no ESLint; “lint sem erros” não significa
  conformidade integral com os guias.
- `.editorconfig` e `.prettierrc` exigem LF; os atributos Git `text`/`eol` não estão
  definidos para o arquivo de TreeView consultado, e `core.autocrlf` local é
  `false`. A revisão não atribui os avisos automaticamente ao checkout ou ao commit:
  é necessário alinhar a política de EOL entre Git, editor e CI.
- Não foram repetidos build, testes frontend ou testes Rust nesta revisão de
  documentação; seus resultados anteriores continuam discriminados nas seções
  de validação. A inspeção estática identifica riscos, mas não substitui os testes
  de falha e concorrência propostos abaixo.

### Organização dos arquivos e limites de responsabilidade

| Área | Avaliação | Ajuste proposto |
| --- | --- | --- |
| `delivery/tauri/commands/library.rs` | Adequado: extrai estado e delega ao handler, sem SQL ou regra de negócio. | Preservar o adaptador fino e os testes de registro/permissões. |
| `core/ledger/library_operation.rs` | Adequado: contrato de intenções separado dos eventos de observação. | Manter essa distinção ao extrair políticas e tipar os resultados. |
| `infra/database/query_handlers/folder_queries.rs` | Consultas de leitura no lugar correto, sem mutações implícitas. | Preservar a separação e revisar a identidade dos caminhos. |
| `infra/database/library_operations/` | Localização geral compatível com um adaptador composto, mas `planning.rs` mistura validação pura, consultas transacionais, escrita SQL e inspeção/criação de diretórios. | Extrair SQL e efeitos catalográficos para `infra/database/handlers/library/`; manter ali as leituras que precisam da mesma transação. Não transferir essas validações para consultas independentes fora da transação. |
| `feature/library/operations.rs` | Orquestração no lugar certo, mas depende concretamente de `processing::watcher::WatcherService` e de `infra::filesystem::folder_recovery`. | Introduzir portas de observação de raízes e armazenamento de recuperação no Core; adaptar as implementações e conectá-las no bootstrap. Isso permite testar falhas sem iniciar sensores nativos. |
| `infra/filesystem/exclusive_move.rs` | Adequado: FFI e diferenças de plataforma isoladas, com `spawn_blocking`. | Manter o contrato de não substituição; ampliar testes de plataforma e concorrência. |
| `components/features/library/useFolderTreeActions.ts` | Hook específico da feature corretamente colocalizado. Não precisa ir para `core/hooks` só por ser um hook. | Extrair sua máquina de estados quando tratar cancelamento e concorrência; preservar a colocalização da parte visual. |
| `folderPresentation.ts`, sidebar e seletor de destino | Helpers específicos podem permanecer na feature. Entretanto, ancestralidade também é regra de validação, e a construção da hierarquia está duplicada. | Separar hierarquia/políticas de apresentação e compartilhar um construtor puro local. Promover ao Core apenas o contrato efetivamente compartilhado com outras features. |
| `ui/TreeView/utils/createTreeDragItem.ts` | O nome sugere utilitário genérico de UI, mas a função conhece os domínios `TAG`, `ASSET` e `FOLDER`. | Receber uma fábrica de payload pela API da árvore e manter o adaptador de domínio em `core/dnd` ou na feature. Não levar o algoritmo para `src/utils` indiscriminadamente. |
| `ui/ContextMenu/ContextMenuCheckboxItem.tsx` | Reuso correto de `Checkbox`, sem regra de biblioteca embutida. | Extrair uma interface documentada em `types.ts` e atualizar a documentação pública do módulo. |
| `core/store/library/folderActions.ts` e `folderDeletionUndo.ts` | Separação adequada: IPC/reconciliação no store, toast/ação visual na feature. | Manter notificações fora do store e fortalecer os tipos de comando e resultado. |

Os locais propostos são uma organização alvo, não uma exigência de criar uma pasta
por função. O objetivo é separar políticas puras, mutações transacionais, efeitos
nativos e apresentação, preservando a atomicidade das consultas de validação.

### Pendências de resiliência e integridade

**P1 — Recuperação durável de operações interrompidas.**
[`library_operations.rs`](../../src-tauri/src/infra/database/library_operations.rs)
prepara alterações no SQLite, executa movimentos e depois confirma a transação.
Uma queda entre movimento e commit pode deixar disco e catálogo divergentes;
o snapshot de exclusão participa da mesma transação e também pode não sobreviver
ao rollback do banco. Implementar um journal versionado com ID de operação,
origem/destino, fases duráveis e recuperação idempotente no boot. Validar com
interrupções reais em cada fase. A garantia desejada é coerência recuperável,
não uma promessa de atomicidade distribuída inexistente entre SQLite e filesystem.

**P1 — Compensação deve tentar todos os movimentos e preservar o diagnóstico.**
[`compensate`](../../src-tauri/src/infra/database/library_operations.rs#L89)
retorna na primeira falha, por causa de `?` dentro do loop reverso. Em uma seleção
com vários movimentos, uma única origem ocupada durante o rollback deixa os
demais movimentos sem tentativa de compensação. O erro inicial também é
substituído pelo erro de compensação. Agregar falhas, continuar os passos
independentes, registrar o estado de cada caminho e disponibilizar reparo. O teste
atual de falha parcial cobre compensação bem-sucedida, não falha na própria
compensação nem falha no commit. Esses cenários precisam de injeção de falhas.
Isso indica risco de divergência, não comprova apagamento físico de arquivos.

**P1 — Fechar o caminho de erro da correlação delete/create do watcher.**
No fluxo adjacente de
[`event_handler.rs`](../../src-tauri/src/feature/library/event_handler.rs#L102),
`handle_path_deleted` guarda uma entrada em `recent_removals` e despacha
`DeleteAsset` com `physical_delete: false`. Esse comando ainda remove a linha do
catálogo em `asset_handler.rs`. Se a descoberta seguinte entrar no ramo
**Fast Match**, ela tenta `UpdateAsset` pelo caminho antigo, descarta o resultado
e retorna sem executar a criação normal. Há, portanto, um caminho em que o
registro antigo já não existe e a descoberta pode terminar sem indexar o novo
arquivo. É uma pendência do fluxo existente, não uma regressão atribuída às
correções do macOS. Reproduzir com eventos delete/create separados e preservar a
identidade durante a janela de correlação, ou usar tombstone reversível. Tratar
explicitamente `NotFound` e outras falhas; testar também arquivos distintos com
mesmo tamanho/data, para não transferir metadados por uma heurística ambígua.

**P2 — Supressão do watcher precisa de reconciliação, não só TTL.**
A correção de `is_exactly_marked` para criação é pertinente e deve ser preservada.
Contudo, remoção e rename ainda consultam `is_marked` por ancestral em
`event_handler.rs`: uma alteração externa legítima dentro de uma pasta recém
movida pode ser descartada durante 15 segundos. O relógio também pode expirar
antes de um evento atrasado. Associar a supressão aos efeitos esperados da
operação e reconciliar caminhos ao concluir/expirar a janela. Testar criação,
remoção e segundo rename externo imediatamente após um movimento do Mundam.

**P2 — Reconfiguração de observação deve ser serializada e recuperável.**
[`feature/library/operations.rs`](../../src-tauri/src/feature/library/operations.rs#L34)
lê o caminho anterior antes da transação do Ledger e reconfigura o watcher depois
do commit. Operações concorrentes podem trabalhar com uma origem anterior
obsoleta. Além disso, `WatcherService::unwatch` remove os registros internos e
somente registra um warning se o sensor nativo falhar, retornando `Ok(())`.
Isso impede o chamador de detectar todas as falhas de reconfiguração. Retornar
origem/destino efetivamente confirmados, reconciliar o conjunto desejado de raízes
e registrar retry durável/outbox para efeitos pós-commit. Não sinalizar falha de
escrita depois de um commit bem-sucedido. Testar dois renames concorrentes,
falha de unwatch/watch e reinício durante a reconfiguração.

**P2 — Tornar a identidade de caminhos coerente com o volume.**
`COLLATE NOCASE` é usado em `planning.rs`, `recovery.rs`, `folder_handler.rs` e
resolvers compartilhados independentemente da sensibilidade do filesystem.
Em volume sensível a maiúsculas, diretórios `A` e `a` podem existir, mas a busca
do pai por caminho pode selecionar o ID errado. Em NTFS/APFS sem diferenciação,
o pré-check de destino existente também impede rename apenas de capitalização.
Definir uma política compartilhada para identidade, Unicode e separadores;
distinguir colisão real de referência ao próprio arquivo. Cobrir APFS sensível
e não sensível, Windows, NFC/NFD e rename apenas de capitalização. A solução deve
manter a proteção contra substituição; não basta retirar o pré-check.

**P2 — Cancelar criação deve ser uma transição explícita e segura.**
[`useFolderTreeActions.ts`](../../src/components/features/library/useFolderTreeActions.ts#L79)
descarta o rascunho com uma exclusão recursiva recuperável. Se um arquivo for
copiado para a pasta enquanto o editor está aberto, Esc também arquiva esse
conteúdo, sem a notificação de Undo do fluxo normal. A cópia continua preservada,
mas o efeito ultrapassa simplesmente cancelar uma pasta vazia. Também é preciso
coordenar Esc com rename em andamento, troca de nó e falha no descarte, pois
`pendingCreationId` é limpo antes do resultado. Modelar estados de criação,
edição, confirmação e cancelamento, com identidade da operação; descartar
automaticamente apenas um rascunho ainda vazio e pertencente à criação atual.
Adicionar testes com arquivo externo, cancelamento durante RPC e falha seguida
de nova tentativa, além dos casos de blur obsoleto já cobertos.

**P2 — Versionar e validar os snapshots de Undo.**
[`snapshot.rs`](../../src-tauri/src/infra/database/handlers/library/snapshot.rs)
exige igualdade de ordem/tabela/colunas com o schema atual. Uma migração futura
pode tornar snapshots persistidos irrestauráveis. Há ainda extrações com
`unwrap_or_default` para IDs/caminhos/formato e restauração de associações que
podem conflitar com tags removidas depois da exclusão. Adicionar versão do
formato, validação dos campos obrigatórios, conversão entre versões suportadas
e política explícita para referências ausentes. Diferenciar erro transitório de
incompatibilidade permanente no retry. Testar snapshot corrompido, upgrade do
schema, pai/tag removidos e falha após restaurar parte das tabelas, sempre
preservando o arquivo de recuperação e o rollback do catálogo.

**P2 — Cor de pasta deve ser uma mutação própria.**
O ramo `FolderColor` de
[`planning.rs`](../../src-tauri/src/infra/database/handlers/library/planning.rs)
faz um `UPDATE` e depois chama `handle_create_folder` para produzir o retorno.
Essa chamada também adota filhos órfãos e registra `CREATE_FOLDER`; além disso,
o helper de validação exige a pasta presente no disco. Uma preferência visual
fica acoplada a existência física, reconstrução de hierarquia e auditoria de
outra intenção. Criar um handler específico, sem upsert/adoption incidental,
com resultado e evento de atualização de cor. Definir e testar a edição de cor
de uma pasta offline e garantir que apenas os campos esperados sejam alterados.

**P2 — Dimensionar transações, memória e efeitos pós-commit.**
O snapshot mantém todas as linhas e múltiplas representações JSON em memória;
o rename de subárvore faz consultas por pasta e updates por asset, mantendo o
`BEGIN IMMEDIATE` aberto durante a preparação e os movimentos. A limpeza de
cache pós-commit espera o scan dos diretórios e, se falhar, só produz log.
Medir bibliotecas grandes, estabelecer limites e paginação/lotes, reduzir
roundtrips e manter tarefas de limpeza com retry. Não fragmentar a transação
de forma a perder garantias sem um protocolo de operação recuperável. O uso de
`std::fs::metadata` em handlers assíncronos do watcher também merece conversão
para Tokio ou execução bloqueante isolada, conforme o guia Rust.

**P2 — Validar o payload efetivo de DnD em todos os caminhos.**
[`useTreeDragDrop.ts`](../../src/components/ui/TreeView/hooks/useTreeDragDrop.ts)
valida o estado global de drag antes de ler `dataTransfer`; depois usa
`JSON.parse` como `DragItem` sem validar a forma do objeto decodificado. O grupo
de `TreeViewItem` tem outra leitura de JSON sem `try/catch`, e o drop de raiz tem
uma terceira política. Centralizar parsing de `unknown`, validação do discriminante
e campos e validação de domínio sobre o item realmente recebido. Aceitar `Promise`
no contrato e tratar rejeições do callback, em vez de depender de todo consumidor
capturá-las. O backend continua sendo a barreira de integridade, mas isso não
substitui o tratamento consistente dos eventos de UI. Testar JSON inválido,
payload incompleto, drag externo sem estado global e callback rejeitado.

### Qualidade, contratos, nomes e documentação

- **P2 — Contratos tipados por intenção.** `LibraryOperationOutcome` ainda
  encapsula um `Asset` fictício para operações de pasta. Evoluir para resultados
  próprios com ID de operação, entidade e efeitos confirmados; manter os tipos
  TS/Rust sincronizados por geração ou testes de serialização. Transformar
  `FolderDialogTarget` em união discriminada com IDs obrigatórios por ação,
  eliminando fallbacks `''` e o `default` que transforma ação ausente em exclusão.
  Substituir a sequência de parâmetros posicionais de `updateTag`, ampliada com
  `resetColor`, por um objeto de intenção documentado.
- **P2 — Reatividade e complexidade.** Investigar o aviso em
  `FolderContextMenu.tsx:40`, onde a fábrica `action` captura propriedades
  reativas; não presumir bug funcional apenas pelo warning. Extrair validação,
  cálculo de posição e feedback visual de `handleDragOver` para reduzir sua
  complexidade de 13 a no máximo 10. Manter a diferenciação entre nesting de
  pastas e reorder de tags, com os testes existentes.
- **P3 — Nomes e responsabilidades menores.** Ainda aparecem `tx`, `op_payload`,
  `path_str`, `folder_id_ref` e closures `e` em `folder_handler.rs`, e `a`/`b` na
  ordenação de `FolderTreeSidebarPanel.tsx`. Preferir `transaction`,
  `operation_payload`, `folder_path`, `folder_identifier`, `error`, `firstNode`
  e `secondNode`, conforme o contexto. Parte dessa nomenclatura é dívida de
  funções preexistentes que foram reutilizadas; não é toda uma regressão nova.
  `planning.rs` tem 309 linhas e um dispatcher com várias responsabilidades:
  dividir por operação é recomendável por SRP, sem aplicar a ele indevidamente
  o limite de 300 linhas que o guia declara para o frontend.
- **P3 — TSDoc incompleto.** Há sumários, mas os exports inspecionados de
  `useFolderTreeActions`, `FolderDestinationPicker`, `FolderActionsDialog`,
  `folderActions`, `ContextMenuCheckboxItem` e `createTreeDragItem` não possuem
  tags TSDoc. Completar parâmetros, retornos e erros relevantes, documentar os
  campos e expor interfaces nomeadas de propriedades em `types.ts` nos componentes
  compartilhados. Incluir exemplos compiláveis onde o guia os exige. Descrever
  corretamente a semântica de “retorno confirmado com warning” e falha de refresh.
- **P3 — Rustdoc incompleto ou desatualizado.** Os três handlers públicos de
  `folder_handler.rs`, além de `available` e `prepare` do planner, não têm a
  documentação exigida por item. `folder_queries.rs` usa seções `Returns` com
  exemplos de erro em vez do `# Errors` obrigatório. Documentar invariantes,
  efeitos e limites de cada operação, incluindo argumentos não óbvios. Corrigir
  o cabeçalho de `infra/filesystem/mod.rs`, que ainda afirma excluir mídia
  original, embora `exclusive_move` a manipule; atualizar o contexto de
  `core/path_guard.rs` para refletir operações antes do commit e supressão exata
  versus por ancestral.
- **P3 — Exemplos e comentários precisam acompanhar a API.** O exemplo de
  `ui/ContextMenu/index.ts` usa `onClick` e omite o discriminante `type`, enquanto
  a API real usa `type: 'item'` e `action`; é documentação preexistente que
  permaneceu desatualizada. Documentar também o novo checkbox exportado. Remover
  comentários de fases/seções ou descrições mecânicas, substituindo-os por
  funções com responsabilidade clara quando necessário. Em `folderDeletionUndo`,
  “durable undo action” deve distinguir o snapshot persistido da ação em toast,
  que não sobrevive a restart/dismiss.
- **P3 — Escalabilidade da árvore e do Undo.** As buscas repetidas por `find`
  para ancestralidade/cores e a construção duplicada das árvores podem ser
  substituídas por um índice `Map` por revisão de metadados. Acrescentar uma lista
  persistente de exclusões/restaurações e política de espaço/retenção para os
  snapshots e arquivos arquivados. Hoje a ação Undo depende do toast em memória,
  embora o snapshot permaneça no banco; acesso pelo explorador não restaura os
  metadados do catálogo.
- **P3 — Observabilidade e cobertura orientadas a falha.** Adicionar ID de
  operação e spans às intenções de usuário, correlacionando fases, compensação,
  avisos do watcher e limpeza. Revisar nomes de testes para corresponderem às
  asserções: o teste de blur obsoleto do hook não prova sozinho rename confirmado
  seguido de cancelamento; o teste de rename cujo nome menciona tags não verifica
  associações nessa função específica. Não confundir mocks de IPC/UI com testes
  reais de disco, transação e sensor nativo.

### Ordem proposta para fechamento

1. Corrigir o ramo de correlação do watcher e testar sua preservação de identidade;
   completar a compensação com diagnóstico de falhas múltiplas. Nenhum caminho
   de erro deve encerrar a descoberta silenciosamente ou ocultar movimentos que
   ainda exigem reparo.
2. Definir e implementar journal/recuperação e efeitos pós-commit idempotentes;
   validar crash e reconfiguração concorrente de raízes. Avisos operacionais devem
   ser rastreáveis e reparáveis, sem repetir uma escrita já confirmada.
3. Fortalecer snapshots, identidade de caminhos e a máquina de estados de criação;
   executar a matriz Windows/macOS, incluindo APFS sensível a maiúsculas e eventos
   externos dentro da janela de supressão. Explicitar a política de volumes distintos.
4. Extrair políticas e handlers, introduzir portas onde há dependência concreta,
   remover o acoplamento de domínio do construtor de payload da UI e dedicar um
   handler à cor. Preservar leituras transacionais e comportamento já validado.
5. Completar Rustdoc/TSDoc, exemplos e nomes; resolver os avisos do recorte e
   adicionar checks de formatação, complexidade e documentação à CI. Reexecutar
   testes após as correções, registrando plataforma, commit e limites de cobertura.

O encerramento funcional pode ser registrado com as evidências já relatadas.
O encerramento de resiliência e conformidade integral deve permanecer condicionado
aos itens acima, com prioridade explícita para P1. Esta revisão alterou somente
esta documentação e não afirma que as pendências levantadas já foram corrigidas.

## Implementação da checagem — 2026-11-06

Implementação iniciada a partir de `58da94bbb0d4f74d5c5ab676d6d784a785571d2e`.
O resultado de 240 testes Rust aprovado pelo usuário é a referência anterior a
estas mudanças. Os achados da seção de checagem foram tratados conforme abaixo.

### Responsabilidades e contratos

- `core/library` contém contratos de resultado, identidade, efeitos e portas de
  observação/armazenamento, além das políticas puras de nomes e limites. A camada
  de aplicação coordena intenção e reconciliação por portas; comandos Tauri
  delegam e não acessam diretamente o sensor concreto.
- Mutações, validação transacional, snapshots, recibos, cor e rascunhos estão em
  `infra/database/handlers/library`. Consultas de identidade, histórico e recibos
  estão em `query_handlers`. O coordenador e seu journal ficam em
  `infra/database/library_operations`; primitivas nativas ficam em
  `infra/filesystem`. Leituras que dependem da mesma transação continuam nela.
- O resultado público identifica operação, entidade, caminho anterior, efeitos
  confirmados e avisos. Pastas não são expostas no IPC como assets fictícios.
  Uma fixture compartilhada por testes Rust/TypeScript verifica a serialização.
  Diálogos usam união discriminada com IDs obrigatórios; atualizações de tags
  recebem um objeto de opções com intenção explícita de reset.
- A construção da hierarquia usa índices `Map` compartilhados pelo seletor e
  sidebar. A fábrica de payload de drag foi movida da UI para `core/dnd`;
  `TreeView` recebe a fábrica do consumidor. Parsing de `unknown`, validação de
  domínio e tratamento de callbacks assíncronos são comuns aos drops de linha,
  grupo e raiz, inclusive quando o payload diverge do estado global de drag.

### Integridade e recuperação

O manifesto é persistido e sincronizado antes do primeiro efeito no disco.
Movimentos exclusivos verificam identidade de origem e não substituem destinos;
a transação registra um recibo durável. Na inicialização, antes dos sensores,
o recibo decide entre finalizar uma operação confirmada e compensar uma não
confirmada. Repetir a recuperação é idempotente. Um erro de confirmação cuja
situação não pode ser consultada mantém o manifesto, sem adivinhar o resultado.

A compensação percorre todos os movimentos reversos possíveis, agrega falhas e
preserva diagnóstico com ID da operação e caminhos. Uma origem ocupada ou uma
identidade divergente não é sobrescrita. A criação só é descartada quando o
recibo, caminho e identidade provam a propriedade do rascunho e a pasta está
vazia, inclusive no catálogo. A janela entre criar o diretório e persistir sua
identidade é tratada como ambígua após crash, sem remoção automática insegura.

O watcher não correlaciona assets por tamanho/data nem descarta eventos por TTL
de ancestral. A identidade nativa persistida permite preservar IDs e metadados
quando remoção/criação representam o mesmo arquivo. O debouncer não usa nome,
extensão, tamanho ou data para inventar pares; pares nativos também passam pela
prova transacional, sem confundir hardlinks distintos. Seu consumidor roda
concorrentemente para evitar bloqueio do próprio canal em rajadas. Trackers
aguardam o par até o prazo, em vez de serem descartados no primeiro tick.
Remoções são adiadas e
cercadas por ID/caminho atual; escritas tardias não apagam registros já movidos.
Scans incompletos não podam o catálogo. Há reconciliação periódica a cada 600 s
e após perda de eventos, com intervalo mínimo de 60 s para essa recuperação.

Raízes desejadas vêm do banco, sob coordenação serializada. Falhas de watch/unwatch
são propagadas, mantêm o registro ainda necessário e recebem retry a cada 5 s.
Falhas de limpeza de cache ficam em recibos persistidos e recebem retry a cada
60 s. Avisos de observação, limpeza ou refresh após commit não transformam a
escrita confirmada em erro que incentive repetir a mutação.

Caminhos equivalentes exigem prova nativa da mesma entrada de diretório, incluindo
seus pais; hardlinks com nomes distintos permanecem distintos. Não há comparação
universal sem distinguir maiúsculas ou normalização Unicode indiscriminada.
Rename apenas de capitalização usa staging quando a identidade comprova alias.
Links simbólicos e reparse points são recusados. Movimentos entre volumes
continuam recusados, sem fallback de cópia/remoção.

### Restauração, preferências e limites

- Snapshots têm envelope de versão, validação anterior à escrita e suporte ao
  formato legado de cinco tabelas. Campos obrigatórios inválidos interrompem a
  restauração sem consumir o arquivo. Associações a tags removidas são ignoradas;
  o pai original precisa estar disponível e indexado, podendo ter um novo ID no
  mesmo caminho. Identidades ausentes em snapshots legados são lidas do arquivo
  arquivado antes de expor a restauração aos sensores.
- Settings → Trash oferece histórico paginado de exclusões/restaurações e retry
  de restauração. A ação do toast é transitória; os snapshots e o histórico são
  persistentes. O acesso ao diretório permanece uma ação avançada explícita.
  O novo comando de histórico tem registro e permissão Tauri próprios.
- A retenção é por tempo indeterminado até restauração. A configuração
  `folder_recovery_max_bytes`, lida pelo repositório de settings, limita o
  inventário lógico de recuperação a 50 GiB por padrão. Exceder a cota impede
  novas exclusões; não purga cópias antigas automaticamente. O valor é inteiro
  positivo em bytes; a UI permite configurá-lo em GiB.
- Snapshots são limitados a 64 MiB e 100 mil entidades, com consultas/inserções
  em páginas de 256. Seleções explícitas de movimento têm limite de 5 mil assets;
  isso não fragmenta o movimento nativo de uma pasta. O inventário de recuperação
  limita a travessia a um milhão de entradas e atualiza até 32 arquivos legados
  por rodada; a limpeza processa até 32 recibos por rodada.
- O inventário físico é calculado fora do writer SQLite. Rename de subárvore usa
  updates em lote. Fanout de scan e canais têm concorrência limitada, e metadados
  bloqueantes são executados fora dos handlers assíncronos ou por Tokio.
- Cor de pasta tem handler próprio, funciona offline e não recria/adota pastas.
  A edição inline usa estados explícitos e bloqueia cancelamento/troca durante
  confirmação. Falhas de descarte mantêm a propriedade do rascunho para retry;
  conteúdo externo impede descarte, preservando os arquivos.

### Qualidade e evidências de validação

Rustdoc/TSDoc, exemplos, nomes e contratos dos exports revisados foram atualizados.
Os avisos de reatividade e complexidade do recorte foram resolvidos. A CI executa
`npm run check:library`, com ESLint sem warnings, limite de complexidade e
checagem de documentação dos exports selecionados. Isso verifica o recorte
explicitado no script, não promete conformidade automática de todo o repositório.
`.gitattributes` define LF para texto e preserva arquivos binários.

Os testes novos usam disco/SQLite reais para recuperação, colisões, identidade,
rollback, snapshots e retry. Um subprocesso termina sem executar destructors nas
fases preparada, movida e confirmada; o processo pai reabre o banco e executa a
recuperação duas vezes. O helper desse subprocesso é intencionalmente ignorado
na execução direta e invocado pelo teste controlador. Isso simula encerramento
abrupto de processo, não prova resistência a toda falha física de energia/storage.

Validações concluídas nesta rodada:

- Suíte frontend completa: **172 testes aprovados em 36 arquivos**; a regressão
  de drop da raiz também foi repetida após isolar o IPC do ambiente de testes.
- `npm run typecheck`, `npm run build`, `npm run check:library` e
  `cargo fmt --all -- --check`: aprovados.
- Suíte Rust final em Windows/MSVC: **257 testes aprovados, zero falhas e um
  helper ignorado**, em 249,40 s. O helper é executado explicitamente pelo teste
  controlador de crash. Comando: `cargo +stable-x86_64-pc-windows-msvc test --lib --locked --quiet -j 2 --manifest-path src-tauri/Cargo.toml`.
- Clippy global com `-D warnings`: não aprovado por pendências preexistentes
  fora das correções desta checagem, como `unwrap` em `search_builder.rs`,
  assinaturas de streaming com argumentos excessivos e MSRV em extratores de
  mídia. A checagem final sem testes apontou 19 diagnósticos residuais;
  não foi suprimido lint global para produzir uma aprovação artificial. Os testes
  preexistentes do debouncer também mantêm cinco usos de `unwrap/expect`; eles
  estão incluídos no resultado não aprovado de Clippy com testes.

Os limites e a paginação controlam os custos, mas esta rodada não mede RSS ou
latência de uma biblioteca real com dezenas de milhares de assets. A validação
de desempenho nessa escala e a matriz de sistemas de arquivos abaixo continuam
necessárias antes de afirmar capacidade ou latência de produção.

### Confirmação por plataforma e operação de reparo

Esta rodada é executada no Windows/MSVC. A aprovação anterior de 240 testes no
macOS não cobre estas mudanças. Revalidar no macOS os fluxos de edição e Undo,
permissões Tauri, sincronização dos diretórios, APFS sensível/não sensível,
NFC/NFD, rename apenas de capitalização e recusa entre volumes. Movimentos entre
volumes, rename em lote e duplicação seguem fora do escopo funcional atual.

Se a recuperação encontrar um manifesto ambíguo, a inicialização retorna um
diagnóstico de reparo em vez de iniciar observadores sobre estado indefinido.
Preservar manifesto e arquivos de recuperação; resolver a colisão ou divergência
indicada sem sobrescrever conteúdo externo e reabrir o aplicativo para nova
tentativa. Não apagar o journal ou editar o catálogo para forçar continuidade:
isso elimina a evidência necessária à decisão segura. A falha permanece explícita
quando a propriedade dos arquivos não pode ser comprovada.
