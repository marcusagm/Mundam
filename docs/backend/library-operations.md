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
  a pasta movida. Falhas de reconfiguração retornam um aviso de reinicialização,
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

Os caminhos de origem e destino são marcados no `LedgerPathGuard`, incluindo seus
descendentes, para evitar que eventos derivados de uma edição do Mundam concorram
com a atualização explícita do banco. O rename de assets preserva ID, notas,
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
- A compensação cobre erros reportados durante a execução. Ainda não existe um
  journal durável que conclua ou reverta automaticamente uma operação interrompida
  por encerramento forçado ou queda de energia entre o movimento e o commit.
- macOS e Linux têm implementação própria; a validação local foi feita no Windows.

## Melhorias adicionais avaliadas

Abrir no explorador, copiar caminho e resetar a cor foram incluídos por serem
úteis no fluxo de organização. Próximas melhorias recomendadas:

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
separada da compilação aprovada. A validação de execução no macOS ainda é necessária.
